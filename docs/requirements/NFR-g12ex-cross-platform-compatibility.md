# Cross-platform lock compatibility

## Metadata

- ID: NFR-g12ex
- Type: Non-Functional Requirement
- Category: Compatibility
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-02uqo, FR-ui8x2, FR-v7ql4
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL provide identical locking behavior and semantics across Unix (Linux, macOS) and Windows platforms using Rust's standard library abstractions.

## Rationale

Cross-platform compatibility ensures:

- Consistent user experience regardless of operating system
- Single codebase without platform-specific branches
- Predictable behavior in mixed-OS environments
- Simplified testing and maintenance

## User Story (if applicable)

The system shall work identically across all supported platforms to ensure users have a consistent experience regardless of their operating system.

## Acceptance Criteria

- [ ] Same Rust std::fs::File locking API works on all platforms
- [ ] No platform-specific code paths required for basic operations
- [ ] Platform differences handled by the standard library
- [ ] Lock acquisition, holding, and release behave identically
- [ ] Timeout mechanisms work the same across platforms
- [ ] Error conditions reported consistently
- [ ] Linux (x86_64, aarch64) fully supported
- [ ] macOS (x86_64, aarch64/M1) fully supported
- [ ] Windows (x86_64) fully supported
- [ ] WSL2 supported as Linux environment
- [ ] ext4, APFS, NTFS support full locking functionality
- [ ] FAT32 gracefully degrades if locking unavailable
- [ ] Network filesystems detected on all platforms

## Technical Details (if applicable)

### Non-Functional Requirement Details

- Compatibility: 100% API compatibility across platforms
- Performance: Platform-specific optimizations allowed if behavior unchanged
- Reliability: Same error recovery on all platforms
- Security: Consistent permission model where applicable

### Platform Support Matrix

| Platform | Architecture    | Filesystem       | Lock Support |
| -------- | --------------- | ---------------- | ------------ |
| Linux    | x86_64, aarch64 | ext4, xfs, btrfs | Full         |
| macOS    | x86_64, aarch64 | APFS, HFS+       | Full         |
| Windows  | x86_64          | NTFS             | Full         |
| WSL2     | x86_64, aarch64 | ext4             | Full         |
| All      | All             | FAT32            | Degraded     |
| All      | All             | Network FS       | Fallback     |

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/cross_platform_tests.rs` (planned)
- Test Names: `test_nfr_g12ex_platform_behavior`, `test_nfr_g12ex_filesystem_support`

### Verification Commands

```bash
# Linux testing
cargo test test_nfr_g12ex

# macOS testing
cargo test test_nfr_g12ex

# Windows testing
cargo test test_nfr_g12ex

# CI matrix testing
# Defined in .github/workflows/test.yml
```

### Success Metrics

- Metric 1: 100% test pass rate on all supported platforms
- Metric 2: Identical behavior logs across platforms for same operations
- Metric 3: No platform-specific bug reports

## Dependencies

- Depends on: Rust std::fs::File locking API (1.89.0+)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Uses flock() system call
- POSIX-compliant behavior
- Handle EINTR appropriately

### Windows

- Uses LockFileEx() API
- Windows-specific error codes mapped to std::io::Error
- Handle ERROR_LOCK_VIOLATION

### Cross-Platform

- Rust standard library abstracts platform differences
- Use cfg attributes only for platform-specific optimizations
- Ensure consistent error messages

## Risks & Mitigation

| Risk                     | Impact | Likelihood | Mitigation                   | Validation                  |
| ------------------------ | ------ | ---------- | ---------------------------- | --------------------------- |
| Platform API differences | High   | Low        | Use std library abstractions | CI testing on all platforms |
| Filesystem quirks        | Medium | Medium     | Test on various filesystems  | Filesystem test matrix      |
| WSL compatibility issues | Low    | Medium     | Test in WSL environments     | Include WSL in CI           |

## Implementation Notes

- Rely on Rust standard library for platform abstraction
- Avoid platform-specific code unless absolutely necessary
- Use #[cfg(target_os)] only for optimizations, not behavior
- Ensure error messages are platform-agnostic
- Test with platform-specific CI runners

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
