# ADR-0001: Concurrent Process Locking Strategy

## Metadata
- Type: ADR
- Owner: Development Team
- Reviewers: Architecture Team
- Status: Proposed
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->
- Date Created: 2025-09-02

## Links
<!-- Internal project artifacts only. The Links section is mandatory for traceability. If a link does not apply, use "N/A – <reason>". -->
- Requirements: See Analysis document for draft requirements (FR-0001 through FR-0005, NFR-0001 through NFR-0003)
- Design: N/A – Design phase not started
- Plan: N/A – Planning phase not started
- Related ADRs: N/A – First ADR for this feature
- Issue: N/A – No tracking issue created yet
- PR: N/A – Implementation not started
- Supersedes: N/A – First version
- Superseded by: N/A – Current version
- Analysis: [`docs/analysis/AN-0001-concurrent-process-locking.md`](../analysis/AN-0001-concurrent-process-locking.md)

## Context
<!-- What problem or architecturally significant requirement motivates this decision? Include constraints, assumptions, scope boundaries, and prior art. -->

### Problem Statement
Multiple kopi processes can run simultaneously without coordination, potentially causing:
- Race conditions during JDK installation/uninstallation
- Corrupted JDK directories from concurrent modifications
- Inconsistent cache state during parallel operations
- Configuration file conflicts

### Key Constraints
- Must work on both Unix and Windows platforms
- Must handle network filesystems (NFS, SMB)
- Cannot require elevated permissions
- Should not leave stale locks after process crashes

### Critical Discovery from Analysis
**Advisory locks (fs2/flock) vs Lock files** are fundamentally different:
- **Advisory locks**: Automatically released by kernel on process exit (even crash)
- **Lock files with PID**: Persist after crash, require stale detection

### Forces in Tension
- **Simplicity** vs **Edge case coverage**: Supporting NFS adds significant complexity
- **Perfect safety** vs **Practical safety**: Atomic operations provide sufficient safety
- **Current needs** vs **Future possibilities**: Can add NFS support when actually needed

### Key Insights from Discussion
After consulting with domain experts, we learned:
- NFS usage is minority case for development tools
- Atomic filesystem operations provide sufficient safety
- cargo successfully uses this approach in production
- YAGNI principle applies: don't build NFS support until proven necessary
- **JDK integrity is paramount**: Never modify vendor distributions
- **Staging + rename pattern**: Eliminates need for completion markers
- Directory existence itself signals completion (simpler, more robust)
- **Minimal locking is better**: Only 2 locks needed (per-version + cache writer)
- **Config doesn't need locks**: Atomic rename is sufficient
- **Timeout strategy**: Default 600s balances UX and practicality; infinite wait available on demand
- **Phased approach**: Start simple (fs2 + timeout), add complexity (heartbeat/lease) only if needed

## Success Metrics (optional)
<!-- Define measurable criteria to evaluate if this decision was successful -->
- Metric 1: Zero reported data corruption issues from concurrent operations (both local and NFS)
- Metric 2: Lock acquisition wait time < 1s in 95% of cases (local filesystems)
- Metric 3: 100% automatic cleanup on process crash (local filesystems with fs2)
- Metric 4: NFS operations complete without hanging (relies on atomic operations only)
- Metric 5: Clear user warning displayed when NFS detected and locking disabled
- Metric 6: < 5 user reports of NFS-related issues in first 6 months (validates YAGNI approach)
- Review date: 2025-12-01 (after 3 months in production)

## Decision
<!-- State the decision clearly in active voice. Start with "We will..." or "We have decided to..." and describe the core rules, policies, or structures chosen. Include short examples if clarifying. -->

We will implement a **simple fs2-based locking strategy** with atomic filesystem operations for safety, following cargo's approach of skipping locks on network filesystems.

### Core Approach
1. **Locking strategy**:
   - **Local filesystems**: fs2 advisory locks (automatic cleanup on crash)
   - **Network filesystems**: Skip locking, rely on atomic operations
   - **Detection**: Check filesystem type, warn user when on NFS

2. **Safety through atomic operations** (works everywhere):
   - Stage installs to `~/.kopi/jdks/.staging/<id>-<random>/`
   - Verify integrity before rename (checksum, java -version)
   - Atomic rename to final location when complete
   - Directory existence itself means installation complete (no markers needed)
   - Write config/cache via temp file + fsync + atomic rename

3. **Minimal lock strategy** (2 locks only):
   - **Per-version lock**: `~/.kopi/locks/<vendor>-<version>-<os>-<arch>.lock`
     - Guards install/uninstall of specific versions
     - Allows parallel installation of different versions
   - **Cache writer lock**: `~/.kopi/locks/cache.lock`
     - Exclusive lock for cache updates only
     - Cache readers do not acquire locks (lock-free reads)
   - **Config updates**: No lock needed - use atomic rename

4. **User experience (Phase 1 - Simple)**:
   - **Default timeout**: 600 seconds (10 minutes) - based on empirical testing (see Performance Measurements)
   - **Configuration** (Priority: CLI > Environment > Config file > Default):
     - CLI flags: 
       - `--wait=<seconds|infinite>`, `--no-wait` (same as `--wait=0`)
       - `--lock-mode=<auto|fs2|none>` for lock mechanism override
     - Environment variables:
       - `KOPI_LOCKING__TIMEOUT=<seconds|infinite>`
       - `KOPI_LOCKING__MODE=<auto|fs2|none>`
     - Config file: `[locking]` section with `timeout` and `mode` settings
   - **Clear messaging**:
     - When waiting: "Another process is installing. Waiting up to 600s (Ctrl-C to cancel, --wait=infinite for unlimited)"
     - On timeout: "Timed out after 600s. Try --wait=1200 or KOPI_LOCKING__TIMEOUT=infinite"
     - On NFS detected with auto mode: "Network filesystem detected; using atomic operations only"
   - **Progress indication**: Simple spinner with elapsed time

### Decision Drivers
- Simplicity over complexity (YAGNI principle)
- Atomic operations provide safety even without locks
- NFS is minority use case for development tools
- Following proven patterns from cargo

### Considered Options
- **Option A**: fs2 + skip on NFS (Chosen - like cargo)
- **Option B**: Hybrid fs2 + PID fallback (Over-engineering)
- **Option C**: Pure PID-based lock files (Unnecessary complexity)
- **Option D**: No locking (Too risky)

### Option Analysis
- **Option A** — Pros: Simple, proven, safe with atomics | Cons: No locks on NFS
- **Option B** — Pros: Full NFS support | Cons: Complex, may not be needed
- **Option C** — Pros: Works everywhere | Cons: Stale lock issues
- **Option D** — Pros: Simplest | Cons: Race condition risks

## Rationale
<!-- Explain why this decision was made. Tie back to drivers and context. Be explicit about trade-offs and why alternatives were not chosen. -->

### Why fs2 + Skip on NFS?
1. **Simplicity wins**: Avoid premature optimization for edge cases
2. **Atomic operations are sufficient**: Staging + rename pattern prevents corruption
3. **cargo validates this approach**: Production-proven strategy
4. **NFS is rare for dev tools**: Most users have local ~/.kopi

### Why Not Hybrid Approach? (Divergence from Analysis)

**Note**: The Analysis document (AN-0001) recommended a hybrid approach with PID-based fallback for NFS. After further consideration and discussion, we chose to diverge from this recommendation for the following reasons:

- **YAGNI (You Aren't Gonna Need It)**: No evidence of NFS demand yet
- **Complexity cost**: PID-based locks need stale detection, cleanup logic  
- **Maintenance burden**: Two code paths to test and maintain
- **Can add later**: If real users report NFS issues, we can enhance
- **cargo's success**: cargo uses the same "skip on NFS" approach successfully in production
- **Atomic operations suffice**: Our staging + rename pattern provides safety even without locks
- **Risk assessment**: The potential for race conditions on NFS is acceptable given:
  - Low probability (NFS usage is rare for dev tools)
  - Mitigated impact (atomic operations prevent corruption)
  - Clear user communication (warning when NFS detected)

### Why Not No Locking?
- Concurrent installs could corrupt directories
- Lost updates in config files
- Cache refresh race conditions

### Key Design Principle
**"Make it work, make it right, make it fast"** - We're making it work (fs2) and right (atomic ops). NFS optimization can come later if needed.

### Atomic Operations Provide Safety
```bash
# Even without locks, this is safe:
staging_dir="~/.kopi/jdks/.staging/temurin-21-$(uuidgen)"
download_and_extract "$staging_dir"
verify_checksum "$staging_dir"
"$staging_dir/bin/java" -version  # Verify it works
mv "$staging_dir" "~/.kopi/jdks/temurin-21"  # Atomic!
# Directory existence = installation complete
```

## Consequences
### Positive
- **Very simple**: Only 2 lock types (per-version + cache)
- Config updates lock-free (atomic rename)
- Parallel installation of different versions supported
- Automatic cleanup on crash via fs2
- Safe even on NFS through atomic operations
- Follows cargo's proven pattern
- JDK distributions remain pristine (no modification)
- No completion markers needed (directory existence is the marker)
- **Flexible timeout**: Default 600s works for most cases, infinite wait available
- **Clear UX**: Users understand what's happening and how to adjust
- **Phased approach**: Can add complexity later if needed

### Negative
- No explicit locking on NFS (relies on atomics only)
- Potential for race conditions on NFS (mitigated by atomic ops)
- Config writes use "last write wins" semantics

### Neutral
- NFS support can be added later if needed
- Most operations are safe with atomic patterns alone

## Implementation Notes

### Filesystem Detection

The system should detect network filesystems by examining the filesystem type using platform-specific system calls (statfs on Unix, GetVolumeInformation on Windows). Common network filesystem types to detect include NFS, NFS4, CIFS, and SMB. When a network filesystem is detected in auto mode, the system should skip file locking and rely solely on atomic operations.

### Lock Acquisition Strategy

The implementation should support three lock modes:
- **Auto**: Detects filesystem type and chooses the appropriate strategy automatically
- **Fs2**: Forces the use of fs2 advisory locks regardless of filesystem type
- **None**: Disables locking entirely, relying only on atomic operations

Lock wait behavior should support three patterns:
- **No wait**: Fail immediately if lock cannot be acquired (timeout = 0)
- **Finite wait**: Wait up to a specified number of seconds (default 600)
- **Infinite wait**: Wait indefinitely until lock is acquired or process is interrupted

Configuration resolution should follow strict priority ordering:
1. Command-line arguments have highest priority
2. Environment variables override configuration file
3. Configuration file values override defaults
4. Built-in defaults apply when nothing else is specified

### Lock File Organization

Lock files should be organized in a flat structure within `~/.kopi/locks/`:
- Per-version locks: Named as `{vendor}-{version}-{os}-{arch}.lock`
- Cache writer lock: Named as `cache.lock`
- All lock files should be created with owner-only permissions for security

### Configuration Updates

Configuration file updates do not require locking. Instead, they should use the atomic rename pattern:
1. Write new configuration to a temporary file
2. Validate the temporary file is complete and correct
3. Atomically rename the temporary file to replace the actual configuration

This ensures configuration updates are atomic and cannot result in partially written files.

### Atomic Installation Pattern

JDK installations should follow a staging pattern to ensure atomicity:

1. **Staging Directory**: Create a unique temporary directory in `~/.kopi/jdks/.staging/` using a combination of version identifier and a random UUID

2. **Download and Extract**: Perform all download and extraction operations in the staging directory

3. **Verification**: Before moving to final location, verify:
   - Checksum matches expected value
   - Java executable can be invoked successfully
   - Directory structure is complete

4. **Atomic Move**: Use filesystem rename operation to atomically move from staging to final location. This operation either succeeds completely or fails completely

5. **Metadata Storage**: Store installation metadata separately from the JDK directory to preserve vendor distribution integrity

### Idempotency Check

The system should treat directory existence as the primary indicator of installation completion:

- If the target JDK directory exists, the installation is considered complete
- Optionally verify associated metadata for additional validation
- If metadata is missing or corrupt but directory exists, offer repair options
- Never modify existing JDK directories; if reinstallation is needed, remove and reinstall completely

This approach ensures operations are idempotent and can be safely retried without side effects.

### Reentrancy and Nested Locks

To prevent self-deadlock when code paths may nest lock acquisitions:

- Implement reference counting within the same process (following volta's pattern)
- First acquisition of a lock increments count to 1 and acquires the fs2 advisory lock
- Subsequent acquisitions within the same process only increment the count
- Releases decrement the count, only releasing the fs2 lock when count reaches 0
- Use RAII pattern to ensure automatic cleanup even on early returns or panics

This allows safe composition of operations that may each require locks without risk of self-deadlock.

### Metadata Management

The existing metadata management implementation should be utilized:

- Metadata files are already stored as `{distribution}-{version}.meta.json` in the JDKs directory
- The `InstallationMetadata` structure already captures platform-specific information (java_home_suffix, structure_type, platform)
- Metadata saving already uses atomic write operations through the existing `save_jdk_metadata_with_installation` function
- The current implementation already keeps vendor JDK distributions unmodified by storing metadata separately

For concurrent operations, ensure that:
- Metadata writes use the same atomic rename pattern as other configuration updates
- Missing metadata files should not prevent JDK usage if the directory exists (already implemented)
- Metadata updates should be coordinated with the per-version lock to prevent conflicts

## Platform Considerations (required if applicable)

### Unix vs Windows
- **Unix**: Uses flock(2) system call
- **Windows**: Uses LockFileEx() API
- **Both**: fs2 provides uniform interface

### NFS Detection
- Check mount type via `/proc/mounts` (Linux)
- Use statfs/statvfs on other Unix systems
- Windows: Check if path is UNC or mapped network drive

### WSL Considerations
- WSL1 doesn't support fcntl locks (use flock like cargo)
- WSL2 behaves like native Linux

## Security & Privacy (required if applicable)
- fs2 advisory locks don't store any data in lock files (kernel-managed)
- Lock files exist as placeholders only, contain no content
- Lock files use restrictive permissions (owner-only)

## Monitoring & Logging (required if applicable)
- Log lock acquisition/release at DEBUG level
- Log lock contentions at INFO level
- Include wait duration in lock acquisition messages
- Error messages clearly indicate lock-related failures

## Open Questions
- [x] ~~Lock granularity?~~ → Resolved: 2 locks only (per-version + cache writer)
- [x] ~~Lock timeout strategy?~~ → Resolved: Default 600s, configurable, infinite option
- [When to add Phase 2 features (heartbeat/lease)?] → [User Feedback] → [After 6 months in production]
- [When to add NFS support?] → [User Feedback] → [Monitor for 6 months]
- [x] ~~Config file support for lock mode?~~ → Resolved: Added [locking] section in config.toml
- [CLI flag naming: --wait vs --lock-timeout?] → [UX Review] → [Before v1.0]

## External References
<!-- External standards, specifications, articles, or documentation only -->
- [fs2 crate documentation](https://docs.rs/fs2) - Cross-platform file locking for Rust
- [flock(2) man page](https://man7.org/linux/man-pages/man2/flock.2.html) - Unix advisory locking
- [Volta sync implementation](https://volta-cli.github.io/volta/main/volta_core/sync/index.html) - Reference implementation
- [Cargo flock.rs](https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs) - NFS handling example

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.