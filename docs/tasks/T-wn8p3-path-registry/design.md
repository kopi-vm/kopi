# T-wn8p3 Path Registry Consolidation Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-wn8p3-path-registry-plan](./plan.md)
- Related Requirements:
  - [FR-hq1ns-canonical-path-registry](../../requirements/FR-hq1ns-canonical-path-registry.md)
  - [NFR-4sxdr-path-layout-compatibility](../../requirements/NFR-4sxdr-path-layout-compatibility.md)
- Related Analysis:
  - [AN-uqva7-path-registry-consolidation](../../analysis/AN-uqva7-path-registry-consolidation.md)

## Overview

Kopi must centralise all filesystem path construction under a single `src/paths/` surface so that installations, cache artefacts, shims, and locks share consistent sanitisation, directory provisioning, and documentation. This design outlines the module layout, data flow, and migration plan required to fulfill FR-hq1ns while guaranteeing the backward-compatible layout defined in NFR-4sxdr.

## Success Metrics

- [ ] All Kopi home path derivations for supported artefacts are exposed via the new `paths` API.
- [ ] Canonical helpers encapsulate sanitisation and directory creation behaviour with unit coverage for both success and error flows.
- [ ] Regression tests confirm on-disk paths and CLI output remain unchanged after migration.

## Background and Current State

- Context: Kopi core commands (install, uninstall, cache refresh, shim management, doctor) compute Kopi-home-relative paths independently, complicating audits and refactors.
- Current behavior: `KopiConfig` hosts some helper methods (`jdks_dir`, `cache_dir`, `shims_dir`), but most callers perform manual `PathBuf::join` operations with duplicated string literals (e.g., `"locks"`, `"install"`). Locking utilities already live under `src/paths/locking.rs`, yet their organisation is inconsistent with other subsystems.
- Pain points: Hard-coded segments drift across modules, sanitisation logic repeats, and no single module documents authoritative layout rules for support teams.
- Constraints: Existing layout under `$KOPI_HOME` must remain intact (NFR-4sxdr). No introduction of `unsafe` code or major dependency churn; design must align with ADR-8mnaz locking strategy and project naming conventions (no generic "manager"/"util").
- Related ADRs: [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md) influences locking path semantics.

## Proposed Design

### High-Level Architecture

```text
┌─────────────────┐    exposes helpers     ┌────────────────────────────┐
│ paths::mod.rs   │──────────────────────▶│ Consumer modules (storage, │
│ - home.rs       │                        │ shims, locking, doctor,    │
│ - install.rs    │                        │ commands)                  │
│ - cache.rs      │                        └────────────┬──────────────┘
│ - shims.rs      │                                     │ uses
│ - locking.rs    │◀────────────────────────────────────┘
│ - metadata.rs   │    sanitises & ensures
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ KopiConfig      │ provides Kopi home root
└─────────────────┘
```

### Components

- `paths::home`: Defines constants for directory names, provides `kopi_home_root()` validation helpers, and re-exports domain modules.
- `paths::install`: Produces installation directories, metadata file paths, and utility functions for staging directories.
- `paths::cache`: Returns cache directory, metadata cache file path, and temporary file helpers.
- `paths::shims`: Derives shims directory paths and shim binaries per tool (respecting platform-specific extensions).
- `paths::locking`: Existing locking helpers refactored to consume shared sanitisation utilities from `paths::shared::sanitize`.
- `paths::shared`: Houses sanitisation, slugging, and directory creation primitives shared by domain modules while avoiding circular dependencies with `crate::locking`.

### Data Flow

1. `KopiConfig::kopi_home()` supplies the root path (absolute `PathBuf`).
2. Callers invoke domain-specific helpers such as `paths::install::jdk_install_root(kopi_home, coordinate)`.
3. Helpers apply shared sanitisation (slugging distribution/vendor strings) and optionally ensure directories exist via idempotent `create_dir_all` wrappers returning `KopiError::ConfigError` on failure.
4. Consumers receive normalised `PathBuf` values and proceed without additional path manipulation.

### Storage Layout and Paths (if applicable)

- JDKs: `$KOPI_HOME/jdks/<distribution>-<version>[-fx][-arch]/`
- Shims: `$KOPI_HOME/shims/<tool>[.exe]`
- Config: `$KOPI_HOME/config.toml`
- Cache: `$KOPI_HOME/cache/metadata.json` and `$KOPI_HOME/cache/<temp>`
- Locks: `$KOPI_HOME/locks/install/<distribution>/<slug>.lock`, `$KOPI_HOME/locks/cache.lock`, `$KOPI_HOME/locks/config.lock`

### CLI/API Design (if applicable)

No new CLI surface; existing commands continue operating with updated internal helpers.

### Data Models and Types

- Introduce lightweight structs:
  - `InstallPaths` encapsulating root directories and metadata file locations for an installation coordinate.
  - `CachePaths` providing stable names for cache file and temp file conventions.
- Shared sanitisation function signature: `fn slug_segment(input: &str) -> String` returning lowercase dashed slug consistent with `sanitize_segment` today.

### Error Handling

- Directory provisioning helpers return `Result<PathBuf, KopiError>` using `KopiError::ConfigError` with descriptive messages when filesystem operations fail.
- Helper APIs avoid panicking; they propagate IO errors up the call stack.

### Security Considerations

- Normalise and validate that resulting paths remain within `KOPI_HOME` using `Path::starts_with` checks to prevent directory traversal via malicious configuration.
- Reuse existing shim security validations after refactor to ensure symlink targets stay within Kopi home.

### Performance Considerations

- Directory creation wrappers should short-circuit when directories already exist to minimise redundant IO; `create_dir_all` is already idempotent.
- Avoid excessive allocations by reusing `PathBuf` prefixes where possible (e.g., pre-compute `paths::home::locks_root`).

### Platform Considerations

#### Unix

- Maintain symlink-based shims and ensure permission bits remain unchanged; helpers return `PathBuf` only and let callers manage permissions.

#### Windows

- Ensure helpers append `.exe` extensions via `platform::executable_extension()` when constructing shim filenames; slug logic must strip characters invalid on NTFS.

#### Filesystem

- Preserve case-insensitive compatibility by lowercasing slugs; ensure UNC paths and long path prefixes (`\\?\`) remain intact by operating on `PathBuf` instead of strings when joining.

## Alternatives Considered

1. **Expand `KopiConfig` helper methods instead of creating a dedicated module**
   - Pros: Minimal module churn.
   - Cons: Keeps configuration concerns tightly coupled with filesystem logic, making reuse in tests/utilities awkward; does not guide non-config consumers.
2. **Introduce a `PathsRegistry` struct passed through constructors**
   - Pros: Object-oriented encapsulation, easier dependency injection.
   - Cons: Adds state to pass broadly, increases lifetime management complexity, and conflicts with guideline preferring functions over stateless structs.

Decision Rationale

- A dedicated `paths` namespace of free functions balances clarity with low coordination cost. It aligns with existing locking helpers, avoids adding stateful structs, and keeps `KopiConfig` focused on configuration concerns.

## Migration and Compatibility

- Backward/forward compatibility: The helper functions compute identical paths to pre-refactor logic; acceptance tests will compare outputs before and after migration using fixtures.
- Rollout plan: Feature flag optional; initial implementation introduces helpers alongside existing code, followed by staged migrations per subsystem, culminating in removal of legacy joins.
- Telemetry/Observability: Optionally add debug logging when falling back to legacy paths during transition (temporary instrumentation removed before completion).
- Deprecation plan: Deprecate direct joins by replacing them with helper calls and, if feasible, adding Clippy lint or code review checklist to prevent regressions.

## Testing Strategy

### Unit Tests

- Add unit tests in each domain module verifying slugging, directory creation behaviour, and sanitisation edge cases (spaces, special characters, Unicode).

### Integration Tests

- Extend existing installer and shim integration tests to assert actual filesystem paths returned by helpers match expectations on temp directories.
- Add regression test capturing `paths::cache::metadata_cache_path` output to guarantee compatibility.

### External API Parsing (if applicable)

- Not applicable; no external APIs involved.

### Performance & Benchmarks (if applicable)

- Monitor `cargo test --lib` runtime during migrations; no dedicated perf benchmarks planned unless path helpers appear in hot loops.

## Documentation Impact

- Update `docs/architecture.md` with a section describing the `paths` module hierarchy.
- Provide developer guidance in `docs/reference.md` referencing the new helpers.
- Coordinate with the external documentation repository (`../kopi-vm.github.io/`) if any user-facing messaging changes (expected to be minimal).

## External References (optional)

- [Rust Path API documentation](https://doc.rust-lang.org/std/path/) – semantics relied upon for path joining and validation.

## Open Questions

- [ ] Should migrations temporarily retain legacy helper aliases for downstream crates or plugins? → Resolve during implementation planning.
- [ ] Do we need a Clippy lint or CI guard to prevent new direct `join("jdks")` usage? → Investigate enforcement options in Phase 3.

## Appendix

### Diagrams

```text
Kopi CLI Command
    │
    ▼
retrieves KopiConfig.kopi_home()
    │
    ▼
paths::install::jdk_install_root(kopi_home, coordinate)
    │
    ▼
returns PathBuf (slugged, ensured)
```

### Examples

```rust
let install_root = paths::install::jdk_install_root(kopi_home, &coordinate)?;
let shim_path = paths::shims::tool_shim_path(kopi_home, "java");
let cache_path = paths::cache::metadata_cache_file(kopi_home)?;
```

### Glossary

- **Kopi home**: Root directory derived from `KOPI_HOME` or `~/.kopi` where all managed artefacts live.
- **Slug**: Lowercase, dash-separated identifier derived from user-provided strings (distribution/vendor names).

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
