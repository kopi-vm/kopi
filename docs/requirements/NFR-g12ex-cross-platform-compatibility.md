# Cross-platform lock compatibility

## Metadata

- Type: Non-Functional Requirement
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-02uqo, FR-ui8x2, FR-v7ql4
- Related ADRs: ADR-8mnaz
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL deliver identical lock semantics and reliability across Linux, macOS, Windows, and WSL platforms using Rust standard library abstractions, ensuring consistent behavior regardless of operating system or filesystem.

## Rationale

Cross-platform consistency guarantees a predictable user experience, avoids divergent code paths, simplifies testing, and prevents subtle lock bugs when teams operate across multiple operating systems and filesystems.

## User Story (if applicable)

The system shall provide consistent locking across all supported platforms to ensure users receive the same behavior regardless of operating system.

## Acceptance Criteria

- [ ] Lock acquisition, holding, and release semantics behave identically on Linux, macOS, Windows, and WSL environments.
- [ ] Standard library file locking APIs (`std::fs::File`) suffice without platform-specific branches for primary flows.
- [ ] Timeout mechanisms and error reporting return equivalent messages and exit codes across platforms.
- [ ] Supported filesystems include ext4, APFS, NTFS, and WSL ext4 with full lock support; detection occurs at runtime.
- [ ] Unsupported or degraded filesystems (e.g., FAT32, certain network filesystems) trigger graceful fallback strategies (documented warnings plus atomic ops).
- [ ] Platform coverage in CI executes lock tests on Linux, macOS, Windows, and WSL runners.

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Not applicable.

### Non-Functional Requirement Details

- Compatibility: Maintain 100% API parity across supported operating systems and architectures.
- Reliability: Ensure lock lifecycle hooks (acquire/release) map to platform-specific APIs without divergence.
- Performance: Allow platform-specific optimizations provided they do not alter behavior.
- Security: Normalize permission handling for lock files across platforms.

#### Platform Support Matrix

| Platform | Architecture        | Filesystem       | Lock Support |
| -------- | ------------------- | ---------------- | ------------ |
| Linux    | `x86_64`, `aarch64` | ext4, xfs, btrfs | Full         |
| macOS    | `x86_64`, `aarch64` | APFS, HFS+       | Full         |
| Windows  | `x86_64`            | NTFS             | Full         |
| WSL2     | `x86_64`, `aarch64` | ext4             | Full         |
| All      | All                 | FAT32            | Degraded     |
| All      | All                 | Network FS       | Fallback     |

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/cross_platform_tests.rs` (planned)
- Test Names: `test_nfr_g12ex_platform_behavior`, `test_nfr_g12ex_filesystem_support`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_nfr_g12ex_platform_behavior
cargo test test_nfr_g12ex_filesystem_support
# CI workflows execute on Linux, macOS, Windows, and WSL runners
```

### Success Metrics

- Metric 1: 100% pass rate for lock integration tests across all supported platforms in CI.
- Metric 2: Identical log outputs for equivalent operations across platforms (line-by-line comparison allowance ± timestamps).
- Metric 3: Zero platform-specific lock regressions reported across two release cycles.

## Dependencies

- Depends on: Rust `std::fs::File` locking API (1.89.0+)
- Blocks: FR-02uqo, FR-ui8x2, FR-v7ql4 (all require cross-platform guarantees)

## Platform Considerations

### Unix

- Uses `flock` via Rust standard library; handle `EINTR` and other POSIX-specific nuances.

### Windows

- Uses `LockFileEx`; ensure proper error translation to `std::io::Error` and support for overlapped operations when necessary.

### Cross-Platform

- Detect filesystem characteristics at runtime to decide between advisory locks and fallback strategies.
- Provide consistent logging and error messaging regardless of OS.

## Risks & Mitigation

| Risk                     | Impact | Likelihood | Mitigation                                            | Validation                  |
| ------------------------ | ------ | ---------- | ----------------------------------------------------- | --------------------------- |
| Platform API differences | High   | Low        | Rely on std library abstractions; integration testing | CI matrix coverage          |
| Filesystem quirks        | Medium | Medium     | Detect filesystem type and adjust strategy            | Filesystem acceptance tests |
| WSL compatibility issues | Low    | Medium     | Treat WSL as Linux but verify path handling           | Include WSL in CI runs      |

## Implementation Notes

- Avoid `#[cfg]` branches except where absolutely necessary; when required, encapsulate them behind a common trait.
- Cache filesystem detection results to minimize repeated syscalls while retaining change detection.
- Surface platform and filesystem info in debug logs to aid support cases.
- Document fallback behavior in user-facing docs maintained in `../kopi-vm.github.io/`.

## External References

N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd).
