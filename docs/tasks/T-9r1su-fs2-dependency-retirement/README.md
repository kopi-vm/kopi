# FS2 Dependency Retirement Task

## Metadata

- Type: Task
- Owner: Development Team
- Reviewers: TBD
- Status: Proposed

## Links

- Analysis: N/A – Task derived directly from ADR discussions
- Requirements: N/A – To be authored once replacement approach is selected
- ADRs: [`ADR-8mnaz-concurrent-process-locking-strategy.md`](../../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Design: N/A – Not started
- Plan: N/A – Not started
- Issue: N/A – Not created

## Summary

Retire the `fs2` crate by migrating all remaining usages to supported platform APIs (for example `std::fs::File` locking and `sysinfo` disk queries) without coupling to ADR-8mnaz implementation scope.

## Scope

- In scope: Identify and replace every `fs2` usage in the repository; document the new locking/disk APIs; remove the dependency from `Cargo.toml`.
- Out of scope: Broader refactors unrelated to file locking or disk space checks; updates to external tooling that still depends on `fs2`.

## Success Metrics

- `fs2 dependency removed`: No references to the crate in source files or manifests.
- `Functional parity`: Disk space checks and file-in-use detection validated on macOS, Linux, and Windows with automated or documented manual tests.

## Detailed Plan

- Audit `src/storage/disk_space.rs`, `src/doctor/checks/jdks.rs`, and `src/platform/file_ops.rs` to confirm current behaviour and replacement requirements.
- Prototype disk space retrieval using a supported crate (e.g., `sysinfo`) and capture sample outputs for regression tests.
- Design and implement a `std::fs::File`-based alternative for `check_files_in_use()` with platform-specific verification notes.
- Produce follow-up requirements/design/plan documents once the approach is validated and ready for implementation.
- Remove `fs2` from `Cargo.toml`, run the full Rust completing-work commands, and update documentation that references `fs2`.

## Notes

- Outstanding `fs2` usage as of 2025-09-18:
  - `src/storage/disk_space.rs:40-58`
  - `src/doctor/checks/jdks.rs:332-356`
  - `src/platform/file_ops.rs:138-220`

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
