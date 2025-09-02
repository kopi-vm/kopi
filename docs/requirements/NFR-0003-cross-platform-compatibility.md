# NFR-0003: Cross-platform lock compatibility

## Metadata
- Type: Non-Functional Requirement
- Category: Compatibility
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Approved
- Priority: P0
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
- Analysis: [`docs/analysis/AN-0001-concurrent-process-locking.md`](../analysis/AN-0001-concurrent-process-locking.md)
- Related ADRs: [`ADR-0001-concurrent-process-locking-strategy.md`](../adr/ADR-0001-concurrent-process-locking-strategy.md)
- Related Requirements: FR-0001, FR-0002, FR-0003 (all lock operations)
- Issue: N/A – No tracking issue created yet
- Task: N/A – Implementation not started

## Requirement Statement

The system SHALL provide identical locking behavior and semantics across Unix (Linux, macOS) and Windows platforms using Rust's standard library abstractions.

## Rationale

Cross-platform compatibility ensures:
- Consistent user experience regardless of operating system
- Single codebase without platform-specific branches
- Predictable behavior in mixed-OS environments
- Simplified testing and maintenance

## Acceptance Criteria

1. **Uniform API Usage**
   - The same Rust std::fs::File locking API SHALL work on all platforms
   - No platform-specific code paths SHALL be required for basic operations
   - Platform differences SHALL be handled by the standard library

2. **Behavioral Consistency**
   - Lock acquisition, holding, and release SHALL behave identically
   - Timeout mechanisms SHALL work the same across platforms
   - Error conditions SHALL be reported consistently

3. **Platform Coverage**
   - Linux (x86_64, aarch64) SHALL be fully supported
   - macOS (x86_64, aarch64/M1) SHALL be fully supported
   - Windows (x86_64) SHALL be fully supported
   - WSL2 SHALL be supported as Linux environment

4. **Filesystem Compatibility**
   - ext4, APFS, NTFS SHALL support full locking functionality
   - FAT32 SHALL gracefully degrade if locking unavailable
   - Network filesystems SHALL be detected on all platforms

## Measurement Methods

- **Platform Testing**: Run test suite on Linux, macOS, Windows
- **CI/CD Coverage**: Automated tests on all target platforms
- **Filesystem Testing**: Test on native filesystem for each OS
- **Behavior Verification**: Compare operation logs across platforms

## Target Metrics

| Metric | Target | Minimum Acceptable |
|--------|--------|-------------------|
| Platform test pass rate | 100% | 95% |
| API consistency | 100% uniform | 100% uniform |
| Performance variance | < 10% | < 25% |
| Platform-specific bugs | 0 | < 2 minor |

## Implementation Notes

- Rust std::fs::File provides platform abstraction:
  - Unix: Uses flock(2) system call
  - Windows: Uses LockFileEx() API
  - Both: Automatic cleanup on process exit
- Avoid platform-specific features unless absolutely necessary
- Use cfg attributes only for platform-specific optimizations

## Verification Steps

1. **Linux Test Suite**
   - Run full test suite on Ubuntu/Debian
   - Verify all locking operations succeed

2. **macOS Test Suite**
   - Run full test suite on macOS (Intel and M1)
   - Verify identical behavior to Linux

3. **Windows Test Suite**
   - Run full test suite on Windows 10/11
   - Verify equivalent functionality

4. **WSL2 Test**
   - Run tests in WSL2 environment
   - Verify Linux-compatible behavior

5. **Cross-Platform Scenario**
   - Share filesystem between OS instances
   - Verify lock interoperability

## Dependencies

- Rust standard library 1.89.0+
- Platform-specific system calls (abstracted by std)
- CI/CD infrastructure for multi-platform testing

## Out of Scope

- BSD variants support
- Mobile platforms (iOS, Android)
- Embedded systems
- Legacy OS versions (Windows 7, Ubuntu 16.04)