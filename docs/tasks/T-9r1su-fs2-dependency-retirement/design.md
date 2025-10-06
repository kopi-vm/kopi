# FS2 Dependency Retirement Design

## Metadata

- Type: Design
- Status: Draft

## Links

- Related Requirements:
  - [FR-x63pa-disk-space-telemetry](../../requirements/FR-x63pa-disk-space-telemetry.md)
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - [ADR-8mnaz-concurrent-process-locking-strategy](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)

## Overview

This design replaces all remaining `fs2` usage with first-party alternatives. Disk space calculations will leverage `sysinfo`, which Kopi already bundles, while file-in-use detection adopts the Rust 1.89.0 standard library locking API. The outcome removes a supply-chain liability, maintains doctor diagnostics, and aligns with ADR-8mnaz.

## Success Metrics

- [ ] `fs2` crate removed from `Cargo.toml` and `cargo metadata` dependency graph.
- [ ] Disk space checks return within 50ms (p95) on local SSDs across macOS, Linux, and Windows.
- [ ] No regressions in doctor `jdks` check output compared with current snapshots.

## Background and Current State

- Context: Disk space validation and file-in-use detection guard Kopi's install/uninstall workflows and doctor checks.
- Current behavior: `DiskSpaceChecker` and `DoctorJdksCheck` call `fs2::available_space`. `check_files_in_use` imports `fs2::FileExt` for non-blocking locks.
- Pain points: `fs2` is loosely maintained, duplicates functionality now stable in Rust, and complicates security audits.
- Constraints: Preserve existing user-facing messages and error types; avoid introducing unsafe code.
- Related ADRs: `docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md` mandates standard library locks.

## Proposed Design

### High-Level Architecture

```text
+---------------------+      +---------------------+
| DiskSpaceChecker    |      | FileInUse Detection |
| (storage module)    |      | (platform module)   |
+----------+----------+      +----------+----------+
           |                              |
           v                              v
+---------------------+      +---------------------+
| SysinfoDiskProbe    |      | StdFileLockAdapter  |
| (new helper)        |      | (new helper)        |
+---------------------+      +---------------------+
```

### Components

- **SysinfoDiskProbe**: Internal helper providing `available_bytes(path: &Path) -> Result<u64>` using `sysinfo::Disks` snapshots filtered by mount point.
- **StdFileLockAdapter**: Wrapper exposing `try_lock_exclusive(path: &Path) -> Result<LockOutcome>` that uses `std::fs::File` methods and returns structured results for messaging.
- Updated `DiskSpaceChecker` and doctor `jdks` check to depend on `SysinfoDiskProbe` instead of calling `fs2` directly.
- Updated `check_files_in_use` functions to call `StdFileLockAdapter` and avoid conditional traits on `fs2`.

### Data Flow

1. Caller requests disk space check.
2. `SysinfoDiskProbe` refreshes disks, finds the mount for the target path, and returns available bytes.
3. `DiskSpaceChecker` converts bytes to MB/GB and applies thresholds.
4. For file-in-use detection, `StdFileLockAdapter` opens each critical binary, attempts `try_lock_exclusive`, and returns success/failure.
5. Doctor or uninstall flows translate outcomes into warnings or suggestions.

### Storage Layout and Paths (if applicable)

- JDKs remain under `~/.kopi/jdks/<distribution>-<version>/`; disk lookup will resolve mount from these paths.
- No new files introduced.

### CLI/API Design (if applicable)

No CLI changes: doctor and uninstall commands retain existing flags and output formats.

### Data Models and Types

- Introduce `enum LockOutcome { Acquired, InUse(String) }` (exact signature TBD) to encapsulate results.
- Potential trait `DiskProbe` to allow dependency injection during testing.

### Error Handling

- Preserve `KopiError::DiskSpaceError` for low space conditions and wrap probe failures in `KopiError::SystemError` with actionable guidance.
- When locking fails, continue returning human-readable strings describing the path; internal errors log at DEBUG.

### Security Considerations

- No new external dependencies. Ensure probe handles untrusted paths safely by canonicalising before mount mapping.
- Avoid leaking full paths in logs beyond existing behaviour.

### Performance Considerations

- Limit `sysinfo` refresh scope: use `System::new_with_specifics(RefreshKind::new().with_disks_list().with_disks())` to refresh only disks.
- Cache probe instance within a single command invocation where practical to avoid repeated refreshes.

### Platform Considerations

#### Unix

- Validate behaviour on APFS, ext4, and Btrfs to ensure mount detection works with symlinks in project directories.
- Confirm locking handles files with executable bit set but unreadable due to permissions.

#### Windows

- Ensure UNC paths or mapped network drives produce informative warnings when locks or disk statistics are unavailable.
- Close file handles promptly to avoid interfering with subsequent operations.

#### Filesystem

- Document expected degradation on network filesystems (locks may succeed but not be reliable; disk stats may fall back to warnings).

## Alternatives Considered

1. Continue using `fs2` with a fork
   - Pros: Minimal code churn
   - Cons: Ongoing maintenance burden, security audit friction
2. Implement raw OS bindings (`statvfs`, `GetDiskFreeSpaceEx`, `LockFileEx`)
   - Pros: Fine-grained control, no external crates
   - Cons: Requires unsafe code, increases platform-specific surface area

Decision Rationale

Using `sysinfo` and standard library APIs provides strong portability, matches existing dependencies, and eliminates the external crate without sacrificing correctness.

## Migration and Compatibility

- No breaking changes. Disk output and lock warnings retain current strings.
- Remove `fs2` from manifests and regenerate `Cargo.lock`.
- Update internal documentation referencing `fs2`; user docs mention "standard library locks" instead.
- Telemetry remains unchanged except for optional debug logs noting use of std locks.

## Testing Strategy

### Unit Tests

- Add unit tests for `SysinfoDiskProbe` with captured sample data representing Linux, macOS, and Windows JSON snapshots.
- Ensure `StdFileLockAdapter` has tests covering acquired locks, files missing, permission denied, and in-use scenarios (using temp files/threads).

### Integration Tests

- Extend doctor integration test (if available) to assert disk space string pattern and file-in-use messaging.
- Add CLI smoke test on Windows via CI job exercising uninstall flow with a spawned java process.

### External API Parsing (if applicable)

- Include inline sample outputs from `sysinfo --json` (or equivalent) to ensure parsing remains stable.

### Performance & Benchmarks (if applicable)

- Measure probe execution time during `cargo test -- --ignored disk_probe_benchmark` (lightweight micro-benchmark) to guarantee the 50ms target.

## Documentation Impact

- Update `docs/error_handling.md` if error variants gain new context messages.
- Update dependency references in `docs/tasks/T-9r1su-fs2-dependency-retirement/README.md` and cross-link user documentation repo to describe new behaviour.

## External References (optional)

- [sysinfo crate](https://docs.rs/sysinfo/) - Disk statistics used by `SysinfoDiskProbe`
- [Rust 1.89.0 file locking announcement](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html)

## Open Questions

- [ ] Should we reuse the existing global `System` instance used for shell detection to avoid duplicate refreshes? → Evaluate during implementation.
- [ ] How do we communicate reduced accuracy on network volumes to users? → Decide in plan's documentation tasks.

## Appendix

### Diagrams

```text
DiskSpaceChecker --> SysinfoDiskProbe
check_files_in_use --> StdFileLockAdapter --> std::fs::File
```

### Examples

```bash
# Doctor disk space warning remains unchanged
kopi doctor --checks jdks
```

### Glossary

- **Probe**: Helper responsible for translating platform-specific system data into Kopi measurements.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
