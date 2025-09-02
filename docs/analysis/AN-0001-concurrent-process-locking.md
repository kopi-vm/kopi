# Concurrent Process Locking Analysis

## Metadata
- Type: Analysis
- Owner: Development Team
- Status: Active
- Date Created: 2025-09-02
- Date Modified: 2025-09-02

## Links
<!-- Internal project artifacts only -->
- Related Analyses: N/A – Standalone analysis
- Existing Requirements: N/A – New area
- Existing ADRs: N/A – No related decisions yet
- Issue/Discussion: N/A – No tracking issue

## Executive Summary

This analysis explores the need for a locking mechanism in kopi to handle concurrent process execution safely. Currently, multiple kopi processes can run simultaneously without coordination, potentially causing race conditions during JDK installation, uninstallation, cache operations, and configuration updates. The analysis identifies critical sections requiring synchronization and proposes a file-based locking strategy to ensure data integrity and prevent conflicts.

## Problem Space

### Current State
- Multiple kopi processes can execute simultaneously without coordination
- No locking mechanism exists for protecting shared resources
- Potential race conditions in:
  - JDK installation/uninstallation operations
  - Cache refresh and metadata updates
  - Configuration file modifications (.kopi-version, config.toml)
  - Shim installation and updates
- Current mutex usage is limited to internal thread synchronization, not cross-process coordination

### Desired State  
- Safe concurrent execution of multiple kopi processes
- Atomic operations for critical sections (install, uninstall, cache refresh)
- Consistent state maintenance across all operations
- Clear feedback to users when operations are blocked by locks
- Minimal performance impact for non-conflicting operations

### Gap Analysis
- Missing: Cross-process synchronization mechanism
- Missing: Lock acquisition and release strategy
- Missing: Timeout and deadlock prevention
- Missing: User feedback for lock contention
- Missing: Cleanup mechanism for stale locks

## Stakeholder Analysis

| Stakeholder | Interest/Need | Impact | Priority |
|------------|---------------|---------|----------|
| End Users | Reliable JDK management without corruption | High | P0 |
| CI/CD Systems | Parallel build processes with kopi | High | P0 |
| Development Teams | Multiple terminal sessions using kopi | Medium | P1 |
| System Administrators | Automated JDK provisioning | Medium | P1 |

## Research & Discovery

### User Feedback
- Potential issue: Concurrent installations may corrupt JDK directories
- Potential issue: Cache refresh during installation may cause inconsistencies
- Potential issue: Multiple processes modifying .kopi-version simultaneously

### Competitive Analysis

#### nvm (Node Version Manager)
- **Implementation**: No explicit file locking mechanism
- **Architecture**: POSIX-compliant bash script
- **Concurrency Strategy**: Relies on sequential command execution and shell behavior
- **Lock Files**: None identified
- **Observations**: Uses temporary directories for downloads and careful file management, but no flock, pid files, or lock files
- **Risk Assessment**: Potential for race conditions in concurrent executions
- **Source**: https://github.com/nvm-sh/nvm/blob/master/nvm.sh

#### pyenv (Python Version Manager)
- **Implementation**: Shell's `noclobber` option for atomic file creation
- **Lock File Location**: `${PYENV_ROOT}/shims/.pyenv-shim` (prototype shim file)
- **Timeout Mechanism**: 60-second default timeout with retry logic (0.1s sleep if supported, else 1s)
- **Lock Acquisition**:
  ```bash
  acquire_lock() {
    local ret
    set -o noclobber
    echo > "$PROTOTYPE_SHIM_PATH" 2>| /dev/null || ret=1
    set +o noclobber
    [ -z "${ret}" ]
  }
  ```
- **Purpose**: Ensures only one pyenv-rehash process runs at a time
- **Cleanup**: Uses trap to clean up prototype shim on exit
- **Advantages**: Portable, no external dependencies, works on NFS
- **Source**: https://github.com/pyenv/pyenv/blob/master/libexec/pyenv-rehash

#### volta (JavaScript Tool Manager - Rust)
- **Implementation**: `fs2` crate for cross-platform file locking
- **Lock File**: `volta.lock` in the Volta directory
- **Architecture**: RAII pattern with reference counting for nested locks
- **Lock Strategy**:
  - First lock acquires file lock and sets count to 1
  - Subsequent locks increment count (within same process)
  - Lock release decrements count
  - File lock released when count reaches 0
- **Purpose**: Prevents multiple processes from modifying Volta directory simultaneously
- **Intra-process Handling**: Count mechanism prevents deadlocks when multiple code paths need locks
- **Platform Support**: Uses flock(2) on Unix, LockFileEx on Windows
- **Source**: https://volta-cli.github.io/volta/main/volta_core/sync/index.html

#### rustup/cargo (Rust Toolchain Manager)
- **Implementation**: Custom flock wrapper in `cargo/util/flock.rs`
- **Platform-specific**:
  - Unix: flock() system call (replaced fcntl for WSL compatibility)
  - Windows: LockFileEx() with mandatory locking
  - Solaris: Special handling required
- **Lock Types**: Shared (multiple readers) and exclusive (single writer)
- **User Feedback**: "Blocking waiting for file lock" message during contention
- **Special Cases**:
  - **NFS Handling**: Skips locking entirely on NFS mounts (can block forever)
  - **Filesystem Support**: Gracefully handles filesystems without lock support
- **Error Handling**: Detailed error context, ignores unsupported lock errors
- **No Timeout**: Blocks indefinitely (design choice)
- **Source**: https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs

#### sdkman (Software Development Kit Manager)
- **Implementation**: No explicit file locking mechanism found
- **Architecture**: Bash scripts with Rust rewrite in progress
- **Installation Process**: Sequential operations without lock protection
- **Observations**: No flock usage, lock files, or pid-based locking identified
- **Risk Assessment**: Vulnerable to concurrent installation conflicts
- **Source**: https://github.com/sdkman/sdkman-cli

### Competitive Analysis Summary

#### Key Findings
1. **Locking Adoption**: Only 2 out of 5 tools (40%) implement explicit file locking
2. **Rust Tools Lead**: Both Rust-based tools (volta, rustup/cargo) have robust locking
3. **Shell Scripts Lag**: Bash-based tools mostly lack locking (except pyenv's creative approach)
4. **Platform Challenges**: NFS and WSL compatibility issues are common concerns

#### Implementation Patterns
- **No Locking** (nvm, sdkman): Risk of race conditions but simpler implementation
- **Shell-based** (pyenv): Creative use of noclobber for portability
- **Native System Calls** (volta, rustup): Robust but requires platform-specific code
- **Reference Counting** (volta): Sophisticated approach for nested operations

#### Lessons for kopi
1. **Start with fs2**: Proven in volta and cargo, cross-platform support
2. **Implement User Feedback**: Follow rustup's example of clear messaging during lock waits
3. **Handle NFS Carefully**: Consider cargo's approach of skipping locks on NFS
4. **Add Timeout Support**: Unlike rustup, implement timeouts to prevent indefinite blocking
5. **Use RAII Pattern**: Ensure automatic cleanup like volta's implementation

### Technical Investigation

#### Advisory Locks vs Lock Files
**Critical Distinction**: There are two fundamentally different locking approaches:

1. **Advisory Locks (flock/fs2)** - Used by volta and cargo
   - **Automatic cleanup**: OS releases lock when process dies (even on crash)
   - **No stale lock problem**: Kernel manages lock lifecycle
   - **Limitation**: Doesn't work reliably on NFS
   - **How it works**: Lock is held on file descriptor, not file content

2. **Lock Files with Content** - Traditional approach
   - **Manual cleanup needed**: File persists after process crash
   - **Stale lock problem**: Requires PID checking or timeout
   - **Works on NFS**: File creation is atomic
   - **How it works**: Creates file with PID/timestamp info

#### Why fs2 Eliminates Most Stale Lock Issues
```rust
// With fs2: Lock automatically released on process exit
let file = File::open("kopi.lock")?;
file.lock_exclusive()?;  // Held by kernel
// Process crashes here → Kernel releases lock automatically

// Without fs2: Lock file remains after crash
let lock_data = LockInfo { pid: process::id(), ... };
fs::write("kopi.lock", serde_json::to_string(&lock_data)?)?;
// Process crashes here → Lock file remains, needs cleanup logic
```

#### When Timeouts Are Still Needed
- **Lock acquisition timeout**: Prevent waiting forever for legitimate lock holder
- **NFS fallback**: When fs2 doesn't work, fallback to lock files needs stale detection
- **Deadlock prevention**: Nested operations might deadlock without timeout

#### Implementation precedents from competitive analysis:
- Volta's reference counting for nested locks (prevents self-deadlock)
- Cargo's NFS detection and bypass (avoids unreliable locking)
- Pyenv's timeout is for **waiting**, not stale lock detection

### Data Analysis
- Critical operations requiring locks:
  - Install: ~30-60 seconds (downloading + extracting)
  - Uninstall: ~1-5 seconds
  - Cache refresh: ~5-30 seconds (API calls)
  - Config updates: <1 second
- Lock contention expected to be low in typical usage

## Discovered Requirements

### Functional Requirements (Potential)
- [x] **FR-DRAFT-1**: Process-level locking for installation operations → Formalized as FR-0001 in ADR-0001
  - Rationale: Prevent concurrent installations to same JDK version
  - Priority: P0
  - Acceptance Criteria: Only one process can install a specific JDK version at a time

- [x] **FR-DRAFT-2**: Process-level locking for uninstallation operations → Formalized as FR-0002 in ADR-0001
  - Rationale: Prevent deletion conflicts and partial removals
  - Priority: P0
  - Acceptance Criteria: Uninstall operations are atomic and exclusive

- [x] **FR-DRAFT-3**: Process-level locking for cache operations → Formalized as FR-0003 in ADR-0001
  - Rationale: Ensure consistent metadata state
  - Priority: P0
  - Acceptance Criteria: Cache refresh operations complete atomically

- [x] **FR-DRAFT-4**: Lock timeout and recovery mechanism → Formalized as FR-0004 in ADR-0001
  - Rationale: Prevent deadlocks from crashed processes
  - Priority: P0
  - Acceptance Criteria: Stale locks are detected and can be recovered

- [x] **FR-DRAFT-5**: User feedback for lock contention → Formalized as FR-0005 in ADR-0001
  - Rationale: Users need to understand why operations are waiting
  - Priority: P1
  - Acceptance Criteria: Clear messages when waiting for locks

### Non-Functional Requirements (Potential)
- [x] **NFR-DRAFT-1**: Lock acquisition timeout limit → Formalized as NFR-0001 in ADR-0001
  - Category: Performance
  - Target: Default 600 seconds (10 minutes) wait time before timeout, configurable
  - Rationale: Based on empirical JDK download measurements; prevents indefinite waiting

- [x] **NFR-DRAFT-2**: Lock cleanup reliability → Formalized as NFR-0002 in ADR-0001
  - Category: Reliability
  - Target: 100% automatic cleanup with fs2 (local), no locking on NFS (atomic operations only)
  - Rationale: fs2 provides automatic cleanup via kernel; NFS relies on atomic filesystem operations

- [x] **NFR-DRAFT-3**: Cross-platform lock compatibility → Formalized as NFR-0003 in ADR-0001
  - Category: Reliability
  - Target: Identical behavior on Unix and Windows platforms
  - Rationale: Consistent user experience across platforms

## Design Considerations

### Technical Constraints
- Must work on both Unix and Windows platforms
- Lock files must be in user-writable locations
- Should handle network filesystems (NFS, SMB) gracefully
- Must not require elevated permissions

### Potential Approaches
1. **Option A**: Pure fs2 advisory locking (Simple but limited)
   - Pros: Automatic cleanup on crash, no stale locks, proven in volta/cargo
   - Cons: Doesn't work on NFS, no timeout support
   - Effort: Low
   - Precedent: volta and cargo (with NFS detection)

2. **Option B**: Hybrid fs2 + PID-based fallback (Recommended)
   - Pros: Best of both worlds - automatic cleanup locally, NFS support
   - Cons: More complex implementation
   - Effort: Medium
   - Strategy:
     - Use fs2 on local filesystems (automatic cleanup)
     - Fallback to PID-based lock files on NFS (with stale detection)
     - Acquisition timeout for both mechanisms

3. **Option C**: Pure PID-based lock files
   - Pros: Works everywhere including NFS, full control
   - Cons: Requires stale lock detection, manual cleanup
   - Effort: Medium
   - Precedent: Traditional Unix approach

4. **Option D**: No locking (status quo)
   - Pros: Simplest implementation, no overhead
   - Cons: Race conditions possible, data corruption risk
   - Effort: None
   - Precedent: nvm and sdkman operate this way

### Architecture Impact
- New ADR needed for lock file strategy and location
- New ADR needed for timeout and recovery policies
- Potential new module: `src/locking/` or integration into existing modules
- Impact on error handling for lock-related failures

## Risk Assessment

| Risk | Probability | Impact | Mitigation Strategy |
|------|------------|--------|-------------------|
| Deadlock from crashed processes | Medium | High | Implement lock timeout and stale lock detection |
| Performance degradation | Low | Medium | Use fine-grained locks, allow parallel non-conflicting operations |
| Platform incompatibility | Low | High | Use well-tested cross-platform library (fs2) |
| Lock file corruption | Low | Medium | Use atomic file operations, implement lock validation |

## Open Questions

- [ ] Should locks be per-JDK-version or global per-operation-type? → Owner: Architecture Team → Due: 2025-09-05
- [ ] How to detect NFS reliably across platforms? → Method: Research cargo's implementation
- [ ] Should we implement lock priority/queuing? → Method: Benchmark typical usage patterns
- [x] ~~How to handle stale locks?~~ → Resolved: fs2 handles automatically, NFS fallback uses PID checking

## Recommendations

### Immediate Actions
1. Adopt fs2 crate for advisory locking (automatic cleanup on crash)
2. Implement acquisition timeout (600 seconds default, configurable) to prevent indefinite waiting
3. Add NFS detection with skip strategy (atomic operations only)

### Implementation Strategy

**Note**: After further analysis and discussion, the ADR-0001 chose a simpler approach than the hybrid strategy initially considered here.

**Chosen Approach (per ADR-0001): fs2 + Skip on NFS**
- **Local filesystems**: fs2 advisory locks (automatic cleanup on crash)
- **Network filesystems**: Skip locking, rely on atomic operations
- **Detection**: Check filesystem type, warn user when on NFS
- **Rationale**: YAGNI principle, cargo's proven pattern, atomic operations provide sufficient safety

**Alternative Considered: Hybrid fs2 + PID-based fallback**
- Would provide explicit locking on NFS
- Adds complexity: stale detection, cleanup logic
- Can be added later if real users report NFS issues
- Example PID-based approach (not chosen):
  ```rust
  // Check if lock holder is still alive
  if !is_process_alive(lock_info.pid) {
      // Safe to override stale lock
      force_acquire_lock();
  }
  ```

### Next Steps
1. [x] Create formal requirements: FR-0001 through FR-0005 → Completed in ADR-0001
2. [x] Create formal requirements: NFR-0001 through NFR-0003 → Completed in ADR-0001
3. [x] Draft ADR for: Lock file strategy using fs2 with timeout wrapper → Completed as ADR-0001
4. [ ] Create task for: Implementing core locking module with fs2
5. [ ] Monitor production: Collect NFS usage data to validate YAGNI approach

### Out of Scope
- Distributed locking across multiple machines
- Lock priority or fair queuing mechanisms (may revisit if contention becomes an issue)
- GUI for lock monitoring

## Appendix

### Meeting Notes
N/A - Initial analysis

### References
- fs2 crate documentation: https://docs.rs/fs2
- nvm source code: https://github.com/nvm-sh/nvm/blob/master/nvm.sh
- pyenv rehash implementation: https://github.com/pyenv/pyenv/blob/master/libexec/pyenv-rehash
- volta sync module: https://volta-cli.github.io/volta/main/volta_core/sync/index.html
- cargo flock implementation: https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs
- sdkman CLI repository: https://github.com/sdkman/sdkman-cli
- File locking best practices in Rust
- POSIX advisory locking specification
- flock(2) man page: https://man7.org/linux/man-pages/man2/flock.2.html

### Raw Data
Example lock file structure (only relevant for PID-based locking approach, not chosen in ADR-0001):
```json
{
  "pid": 12345,
  "operation": "install",
  "target": "temurin@21",
  "timestamp": "2025-09-02T10:30:00Z",
  "hostname": "dev-machine"
}
```

Note: The chosen fs2 advisory lock approach doesn't require storing any content in lock files.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](README.md#analysis-template-analysismd) in the templates README.