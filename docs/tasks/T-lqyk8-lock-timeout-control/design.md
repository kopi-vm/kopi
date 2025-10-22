# T-lqyk8 Lock Timeout Control Design

## Metadata

- Type: Design
- Status: Draft
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-lqyk8-lock-timeout-control-plan](./plan.md)

## Overview

Establish a lock waiting policy that honours CLI, environment, and configuration precedence, offers precise finite and infinite timeout budgets, supports user-triggered cancellation, and emits reusable instrumentation callbacks so downstream commands can surface contention feedback without duplicating logic. The design extends the locking foundation from T-ec5ew to satisfy FR-gbsz6 and NFR-z6kan.

## Success Metrics

- [ ] Timeout resolution enforces CLI flag > `KOPI_LOCK_TIMEOUT` > configuration > default precedence and accepts `0` and `infinite` values.
- [ ] Lock waits maintain ±1 second accuracy across 99% of simulated contention scenarios while keeping polling CPU utilisation under 0.1%.
- [ ] Cancellation and timeout paths emit distinct English errors with dedicated exit codes validated by integration tests.

## Background and Current State

- Context: Kopi’s locking subsystem (introduced in T-ec5ew) handles advisory/fallback coordination for installations, cache writers, and hygiene.
- Current behaviour: `LockController` accepts only a single `Duration` sourced from `LockingConfig::timeout()` and uses a fixed 50 ms retry delay; no CLI or environment overrides, infinite semantics, or instrumentation hooks exist.
- Pain points: FR-gbsz6 requirements for precedence, special values, and cancellation are unmet; operators cannot surface elapsed/remaining wait time; timeout accuracy relies on constant polling; Ctrl-C simply interrupts the thread without structured error handling.
- Constraints: Must remain cross-platform, avoid `unsafe`, keep configuration in English, and reuse ADR-8mnaz decisions for advisory vs. fallback behaviour.
- Related ADRs: [`docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md).

## Proposed Design

### High-Level Architecture

```text
CLI/Env/Config inputs
        │
        ▼
LockTimeoutResolver ───► LockAcquisitionRequest ──► LockController::acquire_with
                                                │                │
                                                │                ├─► Advisory backend
                                                │                └─► Fallback backend
                                                │
                                                ├─► LockWaitObserver (instrumentation)
                                                └─► CancellationToken (Ctrl-C / tests)
```

### Components

- **LockTimeoutValue**: Enum (`Finite(Duration)`, `Infinite`) representing the resolved budget with helper methods for logging and validation.
- **LockTimeoutResolver**: Pure function module combining CLI `--lock-timeout`, environment `KOPI_LOCK_TIMEOUT`, configuration (`locking.timeout`), and scope defaults. Provides `resolve(scope, cli_override)` returning `LockTimeoutBudget`.
- **LockAcquisitionRequest**: Struct passed to `LockController` containing `scope`, the resolved `LockTimeoutValue`, the exponential `PollingBackoff`, optional `Arc<dyn LockWaitObserver>`, and a `CancellationToken`.
- **PollingBackoff**: Strategy object producing delays starting at 10 ms, doubling up to a 1 s cap while recording cumulative wait time for accuracy tracking.
- **CancellationToken & CancellationRegistry**: Thin wrapper around `Arc<AtomicBool>` set by signal handlers (Unix: `SIGINT`/`SIGTERM` via `signal_hook::flag`; Windows: console control handler) and exposed for tests to trigger manually.
- **LockWaitObserver Trait**: Callback interface (`on_wait_start`, `on_retry`, `on_progress_tick`, `on_acquired`, `on_timeout`, `on_cancelled`) enabling instrumentation consumers (e.g., progress indicators in T-60h68) to react uniformly.
- **LockingTelemetryEmitter**: Default implementation wiring observer events to structured logs and optional status reporters without hard-coding CLI UX.
- **Error & Exit Code Updates**: New `KopiError::LockingCancelled` variant, enriched timeout messaging, and exit code mapping distinguishing cancellation vs. timeout.

### Data Flow

1. Clap parses optional global `--lock-timeout <seconds|infinite>` flag; commands feed the parsed value into `LockTimeoutResolver`.
2. `LockTimeoutResolver` inspects:
   - CLI override (highest precedence)
   - `KOPI_LOCK_TIMEOUT` environment variable
   - Config field (`locking.timeout`, accepting integer seconds or string `"infinite"`)
   - Scope defaults (`install` 30 s, `cache` 10 s, `uninstall` 30 s; otherwise global default 600 s)
3. The resolved `LockTimeoutValue` and `PollingBackoff` populate a `LockAcquisitionRequest`.
4. `LockController::acquire(scope)` delegates to `acquire_with(request)` which:
   - Emits `on_wait_start`
   - Attempts advisory acquisition, invoking `LockWaitObserver::on_retry` after each contention with elapsed/remaining time
   - Checks `CancellationToken` before each retry; on cancellation, emits `on_cancelled` and returns `KopiError::LockingCancelled`
   - Applies exponential sleep via `PollingBackoff`
   - Falls back to `fallback::acquire` using the same request semantics
5. On success, emits `on_acquired` with total wait metrics; on timeout, emits `on_timeout` before returning `KopiError::LockingTimeout`.

### Storage Layout and Paths (if applicable)

No changes to paths; lock files remain under `$KOPI_HOME/locks/`.

### CLI/API Design

Usage

```bash
kopi <command> --lock-timeout <seconds|infinite>
```

Options

- `--lock-timeout <seconds|infinite>`: Overrides lock acquisition timeout for the current invocation. Accepts integer seconds (`0` means immediate failure) or the string `infinite`.

Environment

- `KOPI_LOCK_TIMEOUT=<seconds|infinite>` overrides configuration for all commands in the shell.

Configuration

- `locking.timeout = <integer>` (seconds) or `locking.timeout = "infinite"` in `config.toml`.

Examples

```bash
kopi install 21 --lock-timeout 45
KOPI_LOCK_TIMEOUT=infinite kopi uninstall 17
```

Implementation Notes

- Use Clap’s global argument support so `--lock-timeout` applies to all subcommands.
- Add a custom parser to map string inputs to `LockTimeoutValue`.

### Data Models and Types

- `enum LockTimeoutValue { Finite(Duration), Infinite }`
- `struct LockTimeoutBudget { value: LockTimeoutValue, started_at: Instant }`
- `struct LockAcquisitionRequest<'a> { scope: LockScope, budget: LockTimeoutBudget, observer: &'a dyn LockWaitObserver, cancellation: CancellationToken, polling: PollingBackoff }`
- `trait LockWaitObserver { … }`
- `struct CancellationToken { cancelled: Arc<AtomicBool> }`
- `struct PollingBackoff { initial: Duration, factor: u32, cap: Duration }`

### Error Handling

- Introduce `KopiError::LockingCancelled { scope, waited_secs }` with exit code distinct from timeouts.
- Update `KopiError::LockingTimeout` message to include override hints (e.g., “Set --lock-timeout or KOPI_LOCK_TIMEOUT to adjust the 30 s limit.”).
- Extend `error::context` to push precedence details (`source=cli/env/config/default`) for diagnostics.

### Security Considerations

- Signal handling uses `signal_hook`’s safe flag API to avoid unsafe code.
- Cancellation tokens do not expose file paths; logging continues to redact absolute locations per existing policy.

### Performance Considerations

- Exponential backoff starting at 10 ms with 1 s cap keeps CPU usage below 0.1% during waits.
- Reuse `Instant` for monotonic timing to ensure ±1 s accuracy.
- Observer callbacks must execute quickly; document expectation that callbacks avoid long blocking operations.

### Platform Considerations

#### Unix

- Register `SIGINT`/`SIGTERM` handlers via `signal_hook::flag`; ensure EINTR handling in advisory path loops already restarts gracefully.

#### Windows

- Use `ctrlc::set_handler` (thin wrapper) or `signal_hook::flag`’s Windows support to set the cancellation flag upon console events.

#### Filesystem

- No change to lock file placement; fallback path honours existing atomic semantics.

## Alternatives Considered

1. **Kernel-level timed locks (`pthread_mutex_timedlock`, `WaitForSingleObject`)**
   - Pros: Native timeout enforcement without polling.
   - Cons: Requires platform-specific `unsafe` code and breaks ADR-8mnaz’s reliance on `std::fs::File`; rejected for portability and safety.
2. **Busy-wait loop with constant sleep**
   - Pros: Simple implementation.
   - Cons: Fails NFR-z6kan CPU target and compromises timeout accuracy; existing approach already exhibits these downsides.

Decision Rationale

The proposed layering keeps locking policy within Rust’s safe abstractions, enables future UX enhancements through observers, and satisfies precedence/cancellation requirements without duplicating logic across commands.

## Migration and Compatibility

- Existing configuration files remain valid; numeric values continue to parse, and `"infinite"` adds backwards-compatible flexibility.
- CLI invocations remain unchanged unless users opt into `--lock-timeout`.
- Downstream tasks (e.g., install, uninstall) gain new helper APIs but can opt into defaults while incrementally adding feedback handling.
- Telemetry hooks are additive; current logging continues to work with the default no-op observer.

## Testing Strategy

### Unit Tests

- `lock_timeout::tests` verifying precedence resolution, special values, and error cases.
- `LockController` tests simulating contention to assert timeout accuracy, backoff growth, and cancellation exit path using mocked observers.
- Observer unit tests confirming event order and payloads.

### Integration Tests

- Extend `tests/locking_lifecycle.rs` to cover CLI/env overrides, cancellation via synthetic signal, and instrumentation callback wiring.
- Add ignored stress test verifying CPU usage and ±1 s accuracy over 5-minute wait loops (gated behind `integration_tests` feature).

### Performance & Benchmarks (if applicable)

- Benchmark polling strategy against baseline to confirm <0.1% CPU overhead on Linux runner (document results in plan).

## Documentation Impact

- Update `docs/architecture.md` locking section with timeout resolver and observer details.
- Update `docs/error_handling.md` with new `LockingCancelled` variant, exit codes, and guidance.
- Coordinate external documentation (`../kopi-vm.github.io/`) for user-facing flag/environment instructions once implementation ships.

## External References (optional)

- [`signal-hook` crate documentation](https://docs.rs/signal-hook/latest/signal_hook/)
- [`ctrlc` crate documentation](https://docs.rs/ctrlc/latest/ctrlc/)

## Open Questions

- [ ] Should observer callbacks carry telemetry payloads for metrics (counts, durations), or remain minimal events until T-60h68 refines UX?
- [ ] Do we need per-scope configuration fields (e.g., `locking.install_timeout`), or can downstream tasks rely on scoped defaults plus the global override?

## Appendix

### Diagrams

```text
LockAcquisitionRequest
    ├─ scope (LockScope)
    ├─ timeout (LockTimeoutValue)
    ├─ observer (dyn LockWaitObserver)
    ├─ cancellation (CancellationToken)
    └─ polling (PollingBackoff)
            │
            ▼
    LockController::acquire_with(request)
        ├─ advisory path (try_lock loop)
        └─ fallback path (atomic create_new loop)
```

### Examples

```rust
let request = LockAcquisitionRequest::builder(LockScope::CacheWriter)
    .with_timeout(resolver.resolve(scope, cli_override))
    .with_observer(progress_observer)
    .with_cancellation(cancellation_registry.global_token())
    .build();
let acquisition = controller.acquire_with(request)?;
```

### Glossary

- **Lock timeout budget**: The total time Kopi is allowed to wait for a lock before erroring.
- **Observer**: Consumer of lock wait events responsible for user feedback or telemetry.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
