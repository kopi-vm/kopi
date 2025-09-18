# T-9r1su FS2 Dependency Retirement

## Metadata

- Type: Task
- Owner: Development Team
- Reviewers: TBD
- Status: Proposed

## Summary

Retire the `fs2` crate by migrating all remaining usages to supported platform APIs (for example `std::fs::File` locking and `sysinfo` disk queries) without coupling to ADR-8mnaz implementation scope.

## Links

- ADR: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Requirements: N/A – To be derived during design
- Design: N/A – Not started
- Plan: N/A – Not started
- Issue: N/A – Not created

## Outstanding fs2 Usage (2025-09-18)

- `src/storage/disk_space.rs:40-58` – uses `fs2::available_space()` during pre-install checks.
- `src/doctor/checks/jdks.rs:332-356` – uses `fs2::available_space()` in doctor report.
- `src/platform/file_ops.rs:138-220` – uses `fs2::FileExt` for `check_files_in_use()` on both Windows and Unix builds.

## Next Steps

1. Produce analysis of disk space API alternatives with cross-platform behaviour.
2. Design replacement for `check_files_in_use()` that relies on `std::fs::File` locking primitives (or an alternate strategy) and document trade-offs.
3. Author requirements (FR/NFR) capturing disk space reporting expectations and "file in use" detection outcomes.
4. Draft design and implementation plan independent of the locking ADR workstream, including coordinated removal of the `fs2` crate from `Cargo.toml`.
