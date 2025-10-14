# FR-v7ql4 Process-Level Cache Locking

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-gbsz6-lock-timeout-recovery](../requirements/FR-gbsz6-lock-timeout-recovery.md)
  - [NFR-g12ex-cross-platform-compatibility](../requirements/NFR-g12ex-cross-platform-compatibility.md)
  - [NFR-vcxp8-lock-cleanup-reliability](../requirements/NFR-vcxp8-lock-cleanup-reliability.md)
- Dependent Requirements:
  - [FR-x63pa-disk-space-telemetry](../requirements/FR-x63pa-disk-space-telemetry.md)
- Related Tasks:
  - [T-m13bb-cache-locking](../tasks/T-m13bb-cache-locking/README.md)

## Requirement Statement

The system SHALL serialise cache mutation operations using exclusive writer locks while allowing concurrent lock-free reads, ensuring JDK metadata cache consistency during refreshes.

## Rationale

Cache files store metadata fetched from foojay.io; concurrent refreshes without coordination risk corrupting files, serving incomplete data, or duplicating downloads.

## User Story (if applicable)

As a Kopi user, I want cache updates to occur safely without blocking reads so that I always receive consistent metadata even when background refreshes run.

## Acceptance Criteria

- [ ] Only one writer at a time acquires the cache lock; additional writers wait or time out per FR-gbsz6.
- [ ] Readers access cache files without locks and never observe partially written data.
- [ ] Cache refresh writes to a temporary file, `fsync`s, and then atomically replaces the target on success.
- [ ] Stress runs (100 iterations) with concurrent readers and one writer do not produce corrupted cache snapshots.
- [ ] The cache lock path (`$KOPI_HOME/locks/cache.lock`) is consistent across platforms and processes.

## Technical Details (if applicable)

### Functional Requirement Details

- Lock file path: `$KOPI_HOME/locks/cache.lock` using exclusive advisory locks.
- Refresh workflow: fetch metadata → write to temp file → `fsync`/`FlushFileBuffers` → atomic rename to final path.
- Readers validate cache integrity (e.g., checksum or length) after each read and fall back to refresh on failure.
- Writer retries align with FR-gbsz6 backoff and timeout configuration.

### Non-Functional Requirement Details

N/A – No additional non-functional constraints beyond related NFRs.

## Platform Considerations

### Unix

- Use POSIX rename for atomic replacement and `fsync` to guarantee durability before rename.

### Windows

- Use `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING` for atomic replacement and `FlushFileBuffers` for durability.

### Cross-Platform

- Temporary files must reside on the same filesystem (`cache.tmpXXXX`) to keep rename atomic.
- Normalise error handling and messaging to keep user feedback identical across platforms.

## Risks & Mitigation

| Risk                                       | Impact | Likelihood | Mitigation                                                   | Validation                          |
| ------------------------------------------ | ------ | ---------- | ------------------------------------------------------------ | ----------------------------------- |
| Non-atomic rename on specific filesystems  | High   | Low        | Detect and warn; fallback to copy + replace                  | Test on local + network filesystems |
| Temporary file left after crash            | Low    | Medium     | Cleanup orphaned temp files on startup                       | Hygiene job with test coverage      |
| Reader sees empty cache during first write | Medium | Low        | Ship default cache snapshot; guard against zero-length files | Integration tests for cold start    |

## Implementation Notes

- Use `.tmp` suffix for temporary files and ensure they sit beside the final cache path.
- Track refresh metrics and contention telemetry to inform timeout tuning.
- Emit debug logs whenever cache locks are acquired, contended, or released.

## External References

N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
