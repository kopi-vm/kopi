# T-q5f2p Process Activity Detection Design

## Metadata

- Type: Design
- Status: Draft
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-q5f2p-process-activity-detection-plan](plan.md)

## Overview

Implement a cross-platform facility for discovering operating system processes that currently hold file handles inside a target JDK installation directory. The detection API must power uninstall safeguards so Kopi can block or warn when binaries remain in use, without introducing `unsafe` code or external command dependencies.

## Success Metrics

- [ ] Correctly surface at least one owning process when integration tests replay captured Linux, macOS, and Windows handle snapshots for a busy JDK.
- [ ] `processes_using_path` enumerates results for a 500 MB JDK tree within 150 ms on Linux and macOS, and within 300 ms on Windows (measured on reference hardware in tests).
- [ ] No usages of `unsafe` and no reliance on shelling out to `lsof`, `handle.exe`, or similar utilities.

## Background and Current State

- Context: Kopi needs to warn before uninstalling or modifying JDKs that are still executing, aligning with the locking strategy defined in ADR-8mnaz.
- Current behavior: `src/platform/process.rs` only wraps `exec_replace` and shell launching; Kopi cannot introspect running processes or open file descriptors.
- Pain points: Uninstalls can silently remove files that are still in use, leading to crashes or undefined behavior in other applications. Operators lack actionable guidance when binaries are busy.
- Constraints: Must stay in safe Rust, maintain existing platform abstraction surface, and avoid new long-lived background daemons. Root access should not be required on Unix-like systems.
- Related ADRs: `/docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md`

## Proposed Design

### High-Level Architecture

```
+-------------------------+
| processes_using_path()  |
| (platform facade)       |
+-----------+-------------+
            |
            v
  +---------+---------+
  | Platform selector |
  +---------+---------+
            |
   +--------+---------+----------------+
   |                  |                |
   v                  v                v
Unix backend   macOS backend   Windows backend
(procfs)       (libproc)       (Win32 handles)
```

### Components

- `ProcessInfo` struct containing `pid: u32`, `exe_path: PathBuf`, and `handles: Vec<PathBuf>` where each handle path is rooted in the monitored JDK directory.
- `processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>>` exposed from `src/platform/process.rs`.
- Private, platform-specific helper functions (`enumerate_processes_unix`, `enumerate_processes_macos`, `enumerate_processes_windows`) defined behind `cfg` blocks inside `src/platform/process.rs` so that the entire implementation resides in a single file.
- Internal helper `fn normalize_target(target: &Path) -> Result<PathBuf>` to canonicalize and resolve symlinks so comparisons are consistent across filesystems.

### Data Flow

1. Callers provide a target JDK root path.
2. The facade canonicalizes the target and dispatches to the platform backend.
3. Each backend enumerates candidate processes, inspects their open file descriptors or handles, and filters entries whose resolved path is inside the canonical target directory.
4. The backend returns `ProcessInfo` records; the facade deduplicates handle paths per process (case-insensitive on Windows) and sorts by PID for deterministic output.

### Storage Layout and Paths (if applicable)

- JDKs are stored under `~/.kopi/jdks/<vendor>-<version>/`; detection receives full paths resolved by higher layers.

### CLI/API Design (if applicable)

- No direct CLI exposure; uninstall logic will leverage the new API to render existing diagnostic messages with expanded context (PID and executable path).

### Data Models and Types

- `ProcessInfo` as described, plus an internal `HandleLocation` enum capturing `File(PathBuf)` or `Symbolic(PathBuf)` variants to help deduplicate symlinked entries.
- Backend-specific lightweight structs (e.g., `UnixProcess`) remain private to keep the public API minimal.

### Error Handling

- Return `KopiError::SystemError` with descriptive messages when enumeration fails due to permissions or OS errors.
- When partial failures occur (e.g., access denied for one process), accumulate warnings via `ErrorContext` but continue processing others; only escalate to hard failure if no processes can be inspected.

### Security Considerations

- Never execute external binaries; rely strictly on OS APIs to avoid command injection vectors.
- Skip processes owned by other users only when the OS enforces permission errors; do not attempt privilege escalation.
- Normalize path comparisons to avoid path traversal or symlink escape issues by ensuring each handle path is canonicalized before comparison.

### Performance Considerations

- Reuse `procfs` iterators to avoid allocating large vectors when scanning `/proc`.
- Bound per-process descriptor traversal using `read_dir` streaming; avoid loading entire descriptor tables when early exit is possible.
- On Windows, batch query handles via `NtQuerySystemInformation` and cache volume path prefixes to minimize repeated `GetFinalPathNameByHandleW` calls.

### Platform Considerations

#### Unix

- Prefer the `procfs` crate to iterate `/proc/<pid>/fd` symlinks, resolving each via `read_link` and using `Path::starts_with` against the canonical target directory.
- Use `/proc/<pid>/exe` to resolve the executable path for reporting; gracefully handle `ENOENT` if the process exits mid-scan.
- Support Linux and other procfs-compatible systems; detect absence of `/proc` and return an informative error.

#### Windows

- Use the `windows` crate to call `NtQuerySystemInformation(SystemExtendedHandleInformation)` to enumerate all handles.
- Filter handles by type `File`, duplicate the handle into the current process, and call `GetFinalPathNameByHandleW` to obtain the canonical NT path; convert to Win32 path and compare against the normalized target (case-insensitive).
- Ensure all duplicated handles are closed promptly to avoid handle leaks.

#### Filesystem

- Handle NTFS junctions and symlinks by canonicalizing both target and candidate paths before comparison.
- Treat case sensitivity according to platform rules (case-insensitive on Windows, case-sensitive otherwise).

## Alternatives Considered

1. Invoke platform utilities (`lsof`, `handle.exe`).
   - Pros: Minimal development effort, mature tooling.
   - Cons: Adds runtime dependencies, fragile parsing, requires elevated privileges on some systems, contradicts portability goals.
2. Rely on `sysinfo` crate to enumerate open files.
   - Pros: Simpler API surface, cross-platform abstraction.
   - Cons: Current crate lacks reliable Windows handle enumeration and does not expose per-descriptor paths needed for precise filtering.

Decision Rationale

- Direct OS API usage keeps dependencies minimal, aligns with Kopi's portability goals, and avoids shelling out or adding brittle parsers. While Windows handle enumeration is complex, encapsulating it in a dedicated backend maintains clarity and testability.

## Migration and Compatibility

- Backward compatible; new API augments uninstall checks without altering existing locking semantics.
- Rollout via feature flag in uninstall command to allow gradual verification before default enablement.
- No deprecations required.

## Testing Strategy

### Unit Tests

- Add backend-specific tests using recorded fixture data (e.g., synthetic `/proc` directories served from `tests/fixtures/unix_proc`) to validate path matching, partial failures, and permission errors.
- Validate helper normalization and sorting behavior.

### Integration Tests

- Introduce cross-platform integration tests under `tests/process_usage.rs` guarded by platform-specific `cfg` attributes, using temporary directories with deliberately opened files to confirm live detection on systems where allowed by CI.
- Ensure tests are resilient to race conditions by closing handles in `drop` and retrying enumeration when processes exit during inspection.

### External API Parsing (if applicable)

- Not applicable; no external web APIs involved.

### Performance & Benchmarks (if applicable)

- Add criterion benchmarks (feature-gated) simulating thousands of open descriptors to confirm performance thresholds.

## Documentation Impact

- Update `docs/reference.md` uninstall section to mention PID reporting once implementation completes.
- Document limitations (e.g., requires `/proc` support) in the downstream MkDocs repository after validation.

## External References (optional)

- [Microsoft Docs: NtQuerySystemInformation](https://learn.microsoft.com/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation) – Handle enumeration API description.
- [Apple Developer: proc_pidinfo](https://developer.apple.com/documentation/kernel/1387446-proc_pidinfo) – macOS process information interfaces.

## Open Questions

- [ ] Can we reuse `libproc` crate safely across supported macOS versions, or do we need a small FFI binding to avoid unstable APIs?
- [ ] Do we require elevated privileges on macOS when scanning processes owned by other users, and how should Kopi message that limitation?
- [ ] What is the acceptable fallback behavior when Windows denies `DuplicateHandle` for system processes—warn or ignore?

## Appendix

### Diagrams

```
Caller → normalize_target → platform backend → process iterator → handle match → ProcessInfo list
```

### Examples

```bash
# Future uninstall diagnostic example
kopi uninstall temurin-21.0.3
# Output (conceptual)
# java (PID 8421) is still running from ~/.kopi/jdks/temurin-21.0.3/bin/java
# Close the process or rerun with --force to continue.
```

### Glossary

- Handle: OS-specific reference to an open file descriptor or file handle owned by a process.
- ProcessInfo: Struct capturing metadata for processes preventing safe uninstall.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
