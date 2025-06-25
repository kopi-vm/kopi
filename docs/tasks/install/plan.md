# Install Command Implementation Plan

## Overview
This document outlines the phased implementation plan for the `kopi install` command, which is responsible for downloading and installing JDK distributions from foojay.io.

## Command Syntax
- `kopi install <version>` - Install latest JDK with specified version (defaults to Eclipse Temurin)
- `kopi install <distribution>@<version>` - Install specific distribution and version

**Note**: When no distribution is specified, Eclipse Temurin will be selected as the default distribution.

## Phase 1: API Integration and Metadata Handling

### Input Resources
- foojay.io API documentation
- `/docs/adr/` - Architecture Decision Records
- `/src/main.rs` - Existing CLI structure

### Deliverables
1. **API Client Module** (`/src/api/mod.rs`)
   - HTTP client configuration using `attohttpc`
   - API endpoint definitions with versioning support
   - Error handling for network failures
   - Rate limiting with exponential backoff
   - Retry logic with configurable attempts
   - API version negotiation and fallback

2. **Metadata Models** (`/src/models/jdk.rs`)
   - JDK metadata structures using `serde`
   - Distribution information
   - Version parsing and validation
   - API response version compatibility layer

3. **Unit Tests** (use mocks extensively)
   - `src/api/mod.rs` - API client unit tests (fully mock HTTP responses)
   - `src/models/jdk.rs` - Model serialization/deserialization tests

4. **Integration Tests** (`/tests/api_integration.rs`) (no mocks)
   - Real API endpoint testing (connect to actual foojay.io API)
   - Conditional execution in CI/CD environments (requires network connectivity)
   - Error scenario testing
   - Network timeout handling

### Success Criteria
- Successfully fetch JDK metadata from foojay.io
- Parse and validate JSON responses
- Handle network errors gracefully
- Respect API rate limits without failing
- Gracefully handle API version changes

## Phase 2: Download and Archive Extraction

### Input Resources
- Phase 1 deliverables (API client and models)
- Platform-specific archive formats documentation
- `/docs/reference.md` - Storage location specifications

### Deliverables
1. **Download Module** (`/src/download/mod.rs`)
   - Progress reporting during download
   - SHA256 checksum verification
   - Resume capability with HTTP Range requests
   - Partial file validation before resume
   - Temporary file handling with `tempfile`
   - Mirror/CDN fallback support
   - Bandwidth throttling option
   - Download size validation (warn if >1GB)

2. **Security Module** (`/src/security/mod.rs`)
   - HTTPS certificate validation
   - Digital signature verification (if available)
   - Checksum validation against official sources
   - Security audit logging

3. **Archive Handler** (`/src/archive/mod.rs`)
   - Platform-specific extraction logic
   - Support for tar.gz (Linux/macOS) using `tar`
   - Support for zip (Windows) using `zip`
   - Permission preservation
   - Archive integrity verification before extraction

4. **Storage Manager** (`/src/storage/mod.rs`)
   - JDK installation path management
   - Directory structure creation
   - Atomic installation (temp dir + rename)
   - Cleanup on failure
   - Disk space pre-check

5. **Unit Tests** (use mocks extensively)
   - `src/download/mod.rs` - Download progress, resume, checksum tests (mock HTTP client)
   - `src/security/mod.rs` - Certificate and signature validation tests (mock certificate verification)
   - `src/archive/mod.rs` - Archive extraction tests with test files (mock file system)
   - `src/storage/mod.rs` - Path management and cleanup tests (mock directory operations)

6. **Integration Tests** (`/tests/download_integration.rs`) (no mocks)
   - End-to-end download simulation (download actual test files)
   - Archive extraction on different platforms (use real archive files)
   - Storage location verification (write to actual file system)
   - Failure recovery scenarios (simulate real network/disk errors)

### Success Criteria
- Download JDK archives with progress indication
- Successfully resume interrupted downloads
- Verify checksums and signatures
- Extract archives preserving file permissions
- Install to correct directory structure atomically
- Handle large files (>500MB) efficiently

## Phase 3: Command Implementation and CLI Integration

### Input Resources
- Phase 1 & 2 deliverables
- `/src/main.rs` - Existing CLI structure with clap
- `/docs/adr/001-kopi-command-structure.md` - Command structure guidelines

### Deliverables
1. **Install Command** (`/src/commands/install.rs`)
   - Command argument parsing
   - Version resolution logic
   - Distribution selection
   - Progress reporting to user
   - `--force` flag for overwriting existing installations
   - `--dry-run` option for validation without installation
   - Conflict detection for existing installations

2. **Version Parser** (`/src/version/parser.rs`)
   - Parse version strings (e.g., "21", "17.0.9", "corretto@21")
   - Version semantics: "21" resolves to latest 21.x.x
   - Validate against available versions
   - Default to Eclipse Temurin when no distribution specified
   - Support version ranges (e.g., ">=17 <21")
   - LTS version recognition

3. **CLI Integration** (update `/src/main.rs`)
   - Add install subcommand with clap derive
   - Command-line options:
     - `--force`: Override existing installation
     - `--dry-run`: Show what would be installed
     - `--no-progress`: Disable progress bars
     - `--timeout`: Download timeout configuration
   - Help text and examples

4. **Unit Tests** (use mocks extensively)
   - `src/commands/install.rs` - Command logic and error handling tests (mock API/download modules)
   - `src/version/parser.rs` - Version string parsing tests (pure logic tests)
   - CLI argument parsing tests (mock command line input)

5. **Integration Tests** (`/tests/install_command_integration.rs`) (no mocks)
   - Full command execution testing (execute actual commands)
   - Various version format testing (use real JDK metadata)
   - Distribution selection verification (verify actual distributions)
   - Error message validation (validate with real error conditions)

### Success Criteria
- `kopi install 21` downloads and installs latest Eclipse Temurin 21.x.x
- `kopi install corretto@17` installs specific distribution (Corretto 17)
- `kopi install 21 --force` overwrites existing JDK 21 installation
- Clear error messages for invalid versions
- Default distribution (Eclipse Temurin) is used when not specified
- Version resolution is unambiguous and documented

## Phase 4: Metadata Caching and Optimization

### Input Resources
- Phase 1-3 deliverables
- Hybrid caching strategy from architecture docs
- `/docs/adr/` - Caching decisions

### Deliverables
1. **Cache Manager** (`/src/cache/mod.rs`)
   - Metadata caching in `~/.kopi/cache/metadata.json`
   - Cache invalidation logic (TTL-based)
   - Offline mode support
   - Cache size limits (default 100MB)
   - LRU eviction policy
   - File-based locking for concurrent access
   - Atomic cache updates

2. **Lock Manager** (`/src/lock/mod.rs`)
   - Process-safe file locking
   - Lock timeout handling
   - Stale lock detection and cleanup
   - Lock acquisition with retry

3. **Cache Integration** (update existing modules)
   - Check cache before API calls
   - Update cache after successful fetches
   - Handle stale cache scenarios
   - Garbage collection for old entries

4. **Unit Tests** (use mocks extensively)
   - `src/cache/mod.rs` - Cache read/write, expiry, invalidation tests (mock file system)
   - `src/lock/mod.rs` - Lock acquisition, timeout, cleanup tests (mock lock mechanisms)
   - Cache corruption handling tests (simulate corruption with mocks)
   - Concurrent access tests (control concurrency with mocks)

5. **Integration Tests** (`/tests/cache_integration.rs`) (no mocks)
   - Full caching workflow tests (cache to actual file system)
   - Offline mode simulation (actually disconnect network)
   - Cache performance benchmarks (measure real I/O performance)
   - Multi-process cache access (verify real inter-process contention)

### Success Criteria
- Second install attempt uses cached metadata
- Works offline with cached data
- Cache updates periodically (24-hour TTL)
- No data corruption with concurrent access
- Cache size stays within limits
- Stale locks are automatically cleaned up

## Phase 5: Integration Testing and Error Handling

### Input Resources
- All previous phase deliverables
- Error scenarios documentation
- Platform-specific considerations

### Deliverables
1. **Error Handler Enhancement** (`/src/error/mod.rs`)
   - Comprehensive error types
   - User-friendly error messages
   - Recovery suggestions

2. **End-to-End Integration Tests** (`/tests/install_e2e.rs`)
   - Complete install workflow scenarios
   - Multiple platform testing
   - Concurrent install handling
   - Network failure simulation
   - Disk space exhaustion testing

3. **Unit Tests** (use mocks extensively)
   - `src/error/mod.rs` - Error formatting and context tests (mock error conditions)
   - Error chain propagation tests (build error chains with mocks)
   - Recovery suggestion validation (mock recovery scenarios)

4. **Additional Integration Tests** (`/tests/install_scenarios.rs`) (no mocks)
   - Cross-platform compatibility tests (run on actual platforms)
   - Permission error handling (trigger real permission errors)
   - Interrupted download recovery (actually interrupt downloads)
   - Version conflict resolution (reproduce real version conflicts)

5. **Documentation Updates**
   - Update `/docs/reference.md` with install command details
   - Add troubleshooting section
   - Platform-specific notes

### Success Criteria
- All edge cases handled gracefully
- Clear error messages guide users
- Documentation complete and accurate

## Implementation Guidelines

### For Each Phase:
1. Start with `/clear` command to reset context
2. Load this plan.md and relevant phase resources
3. Implement deliverables incrementally
4. Run quality checks after each module:
   - `cargo fmt`
   - `cargo test`
   - `cargo clippy`
   - `cargo check`
5. Commit completed phase before proceeding

### Testing Strategy

#### Unit Tests (use mocks extensively)
- Test individual module functionality in isolation
- Fully mock external dependencies (HTTP, file system, processes, etc.)
- Focus on fast execution and deterministic results
- Comprehensively test edge cases and error conditions
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;
      
      #[test]
      fn test_download_with_mock_http_client() {
          let mut mock_client = MockHttpClient::new();
          mock_client.expect_get()
              .returning(|_| Ok(mock_response()));
          // Test logic here
      }
  }
  ```

#### Integration Tests (no mocks)
- Test complete command workflows end-to-end
- Verify integration with actual external services (foojay.io API)
- Confirm real file system operations
- Validate platform-specific behavior
- Conditional execution in CI environments (e.g., tests requiring network connectivity)
- Example:
  ```rust
  #[test]
  #[cfg(not(ci))] // Skip in CI environment
  fn test_real_jdk_download() {
      // Download JDK from actual foojay.io API
      let result = download_jdk("temurin", "21");
      assert!(result.is_ok());
      // Verify file actually exists
      assert!(Path::new(&result.unwrap()).exists());
  }
  ```

#### Other Testing
- Manual testing on Linux, macOS, and Windows
- Performance benchmarks for large downloads
- Security testing for certificate validation

### Error Handling Priorities
1. Network failures - retry with exponential backoff
2. Rate limiting - respect 429 responses, wait and retry
3. Disk space - check before download
4. Permissions - clear error messages with sudo hints
5. Invalid versions - suggest available alternatives
6. Corrupted downloads - verify checksums and retry
7. Certificate errors - fail securely, provide override option

### Security Considerations
1. Always validate HTTPS certificates
2. Verify file checksums from official sources
3. Use secure temporary directories
4. Clean up sensitive data on failure
5. Log security events for audit trail

## Next Steps
Begin with Phase 1, focusing on establishing reliable API communication with foojay.io and creating robust data models for JDK metadata.