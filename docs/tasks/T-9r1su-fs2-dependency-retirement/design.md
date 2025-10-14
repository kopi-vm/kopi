# T-9r1su FS2 Dependency Retirement Design

## Metadata

- Type: Design
- Status: Approved
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-9r1su-fs2-dependency-retirement-plan](./plan.md)

## Overview

Replace all remaining `fs2` usage with first-party alternatives: use `sysinfo` for disk space reporting and Rust 1.89.0 standard library locks for file-in-use detection. The design removes a supply-chain risk while preserving diagnostics and aligns with ADR-8mnaz guidance.

## Success Metrics

- [x] `fs2` crate removed from `Cargo.toml` and dependency graph.
- [ ] Disk space checks return within 50 ms p95 on macOS, Linux, and Windows.
- [ ] No regressions in doctor `jdks` check output compared with current snapshots.

## Background and Current State

- Context: Disk space validation and file-in-use detection protect installation, uninstall, and doctor workflows.
- Current behaviour: `DiskSpaceChecker` and doctor checks rely on `fs2::available_space`; `check_files_in_use` imports `fs2::FileExt`.
- Pain points: `fs2` is lightly maintained, duplicates functionality now stable in Rust, and complicates audits.
- Constraints: Preserve existing messaging, avoid `unsafe` code, maintain cross-platform parity.
- Related ADRs: `docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`.

## Proposed Design

### High-Level Architecture

```text
+----------------------+       +-------------------------+
| DiskSpaceChecker     |       | File-In-Use Detection   |
| (storage module)     |       | (platform module)       |
+----------+-----------+       +-----------+-------------+
           |                               |
           v                               v
+---------------------------+   +----------------------------+
| disk_probe::available_bytes | | try_lock_exclusive helper  |
| (new helper using sysinfo)  | | (std::fs::File locking)    |
+---------------------------+   +----------------------------+
```

### Components

- `disk_probe::available_bytes(&Path) -> Result<u64>`: wraps `sysinfo` snapshot logic and mount resolution.
- `try_lock_exclusive(path: &Path) -> LockStatus`: encapsulates standard library locking with RAII guard support.
- Updated `DiskSpaceChecker`, doctor checks, and uninstall workflows to consume helpers instead of `fs2`.

### Data Flow

1. Caller requests disk space measurement.
2. Probe refreshes disk list, maps path to mount, returns free bytes.
3. Consumer formats data via existing human-readable helpers.
4. For file-in-use detection, helper attempts `try_lock_exclusive`; on failure returns actionable status for doctor/uninstall.

### Storage Layout and Paths (if applicable)

- No new directories introduced; lock files remain under `$KOPI_HOME/locks/`.
- Temporary files for disk probing not required; cache renames unchanged.

### CLI/API Design (if applicable)

No CLI changes; existing commands reuse shared helpers. Documentation references “standard library locks” instead of `fs2`.

### Data Models and Types

- `LockStatus` enum describes `Available`, `InUse`, and `Error` outcomes.
- Probe returns raw byte counts; formatting remains in `DiskSpaceChecker`.

### Error Handling

- Wrap probe failures in `KopiError::SystemError` with guidance to re-run doctor manually.
- Lock helper converts IO errors into contextual `KopiError` variants reused by doctor/uninstall flows.

### Security Considerations

- Lock files retain owner-only permissions; helpers avoid leaking paths in logs beyond existing behaviour.
- No additional telemetry beyond aggregate lock contention counters.

### Performance Considerations

- Limit `sysinfo` refresh scope to relevant disks; reuse snapshots when possible within a single command.
- Lock helper keeps polling lightweight by leveraging existing timeout primitives.

### Platform Considerations

#### Unix

- Validate probe behaviour on ext4, APFS (via macOS), and Btrfs.
- Ensure locking helper handles symlinked JDK paths correctly.

#### Windows

- Confirm `sysinfo` returns accurate data for NTFS and provide warnings for UNC paths.
- Ensure `try_lock_exclusive` closes handles promptly to avoid lingering locks.

#### Filesystem

- Document degraded behaviour on network filesystems and rely on fallback messaging in ADR-8mnaz.

## Alternatives Considered

1. Fork `fs2` and maintain internally
   - Pros: Minimal code churn.
   - Cons: Ongoing maintenance burden, no safety improvement.
2. Implement raw OS bindings (`statvfs`, `GetDiskFreeSpaceEx`, `LockFileEx`)
   - Pros: Fine-grained control.
   - Cons: Requires `unsafe` code, increases platform-specific complexity.

Decision Rationale

- `sysinfo` already ships in Kopi; leveraging it avoids new dependencies.
- Standard library locking became stable in Rust 1.89.0, providing a first-party solution.
- Forking or reimplementing OS bindings would add maintenance risk without meaningful benefit.

## Migration and Compatibility

- Backward compatible; disk output and lock warnings retain existing phrasing.
- Remove `fs2` from manifests and regenerate lockfile.
- Update documentation referencing `fs2` to note historical context.

## Testing Strategy

### Unit Tests

- Probe helper tests with captured `sysinfo` data for macOS, Linux, Windows.
- Lock helper tests covering acquired locks, in-use paths, missing files, and permission errors.

### Integration Tests

- Extend doctor and uninstall integration tests to assert new messaging and behaviour.
- Add CLI smoke tests in Windows CI verifying uninstall flows with a running java process.

### External API Parsing (if applicable)

- Store `sysinfo` JSON snapshots in tests to guard against upstream schema changes.

### Performance & Benchmarks (if applicable)

- Record disk probe timings and lock wait CPU usage to confirm targets set in FR-x63pa and NFR-z6kan.

## Documentation Impact

- Update `docs/error_handling.md` and task README to reflect dependency removal.
- Coordinate with external docs repository for any user-facing messaging changes.

## External References

- [sysinfo crate](https://docs.rs/sysinfo/) – Disk statistics API
- [Rust 1.89.0 release notes](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/index.html)

## Open Questions

- [ ] Should we reuse the existing global `System` instance used for shell detection to avoid duplicate refreshes? → Evaluate in follow-up refactor.
- [ ] How do we communicate degraded behaviour on network volumes in user docs? → Coordinate with documentation maintainers.

## Appendix

### Diagrams

```text
DiskSpaceChecker --> disk_probe::available_bytes
check_files_in_use --> try_lock_exclusive --> std::fs::File
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
