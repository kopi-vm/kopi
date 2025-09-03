# Process-level locking for uninstallation operations

## Metadata

- ID: FR-ui8x2
- Type: Functional Requirement
- Category: Platform
- Priority: P0 (Critical)
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Accepted
- Date Created: 2025-09-02
- Date Modified: 2025-09-03

## Links

- Implemented by Tasks: N/A – Not yet implemented
- Related Requirements: FR-02uqo, FR-gbsz6
- Related ADRs: [ADR-8mnaz](../adr/ADR-8mnaz-concurrent-process-locking-strategy.md)
- Tests: N/A – Not yet tested
- Issue: N/A – No tracking issue created yet
- PR: N/A – Not yet implemented

## Requirement Statement

The system SHALL provide exclusive process-level locking for JDK uninstallation operations to ensure atomic and safe removal of JDK installations.

## Rationale

Without process-level locking, concurrent uninstallation operations could result in:

- Partial directory deletions leaving orphaned files
- Race conditions with installation processes
- Conflicts when one process is using a JDK while another removes it
- Inconsistent metadata after partial removal

## User Story (if applicable)

As a kopi user, I want uninstallation operations to be safe and atomic, so that concurrent operations don't leave my system in an inconsistent state.

## Acceptance Criteria

- [ ] Process acquires exclusive lock for specific JDK version before uninstallation
- [ ] No other process can install or uninstall that version concurrently
- [ ] JDK directory is completely removed or operation fails with no partial removal
- [ ] Installation attempts wait or fail when uninstallation is in progress
- [ ] All associated metadata is removed after successful uninstallation
- [ ] System state remains consistent after operation

## Technical Details (if applicable)

### Functional Requirement Details

- Use the same lock file as installation: `~/.kopi/locks/{vendor}-{version}-{os}-{arch}.lock`
- Lock type: Exclusive (writer) lock
- Lock must be held for entire uninstallation process
- Metadata cleanup must occur after successful directory removal
- Operation must be atomic - either complete success or complete rollback

## Verification Method

### Test Strategy

- Test Type: Integration
- Test Location: `tests/locking_tests.rs` (planned)
- Test Names: `test_fr_ui8x2_concurrent_uninstall`, `test_fr_ui8x2_install_uninstall_conflict`

### Verification Commands

```bash
# Specific commands to verify this requirement
cargo test test_fr_ui8x2
```

### Success Metrics

- Metric 1: Zero partial removals during concurrent uninstall attempts
- Metric 2: 100% atomic operations (complete success or complete failure)

## Dependencies

- Depends on: FR-02uqo (shared locking infrastructure)
- Blocks: N/A – Blocks nothing

## Platform Considerations

### Unix

- Directory removal using `std::fs::remove_dir_all`
- Lock files in `$KOPI_HOME/locks/`

### Windows

- Directory removal handling Windows file locks
- Lock files in `%KOPI_HOME%\locks\`

### Cross-Platform

- Consistent lock file naming across platforms
- Handle platform-specific file deletion behaviors

## Risks & Mitigation

| Risk                        | Impact | Likelihood | Mitigation                  | Validation                      |
| --------------------------- | ------ | ---------- | --------------------------- | ------------------------------- |
| JDK in use during uninstall | High   | Medium     | Check for running processes | Test with active Java processes |
| Partial deletion on crash   | Medium | Low        | Use transactional approach  | Kill process during uninstall   |
| Lock file orphaned          | Low    | Low        | Cleanup on startup          | Monitor lock directory          |

## Implementation Notes

- Check if JDK is currently set as default before removal
- Warn user if JDK appears to be in use
- Consider two-phase removal: mark for deletion, then remove
- Clean up symlinks and shims after directory removal

## External References

N/A – No external references

## Change History

- 2025-09-02: Initial version
- 2025-09-03: Updated to use 5-character ID format
