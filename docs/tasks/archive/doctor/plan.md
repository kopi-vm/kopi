# Doctor Command Implementation Plan

## Overview
This document outlines the phased implementation plan for the Kopi doctor command, which provides comprehensive diagnostics for kopi installations. The command performs health checks across multiple categories and provides actionable solutions for detected issues.

## Command Syntax
- `kopi doctor` - Run all diagnostic checks and display results
- `kopi doctor --json` - Output results in JSON format
- `kopi doctor --verbose` - Show detailed diagnostic information
- `kopi doctor --check <category>` - Run only specific category of checks
  - Categories: `installation`, `shell`, `jdks`, `permissions`, `network`, `cache`

## Phase 1: Core Diagnostic Framework

### Input Resources
- `/docs/tasks/doctor/design.md` - Complete doctor command design
- `/src/error/exit_codes.rs` - Existing error codes and handling
- `/src/config.rs` - Configuration management

### Deliverables
1. **Diagnostic Engine Module** (`/src/doctor/mod.rs`)
   - Core diagnostic framework
   - Check result types and enums:
     - `CheckResult` struct with name, category, status, message, details, suggestion
     - `CheckStatus` enum: Pass, Fail, Warning, Skip
     - `CheckCategory` enum: Installation, Shell, Jdks, Permissions, Network, Cache
   - Result aggregation logic
   - Exit code determination based on results
   - Performance tracking (each check should complete < 100ms)

2. **Command Module** (`/src/commands/doctor.rs`)
   - CLI command structure using clap derive
   - Command options parsing:
     - `--json` flag
     - `--verbose` flag
     - `--check <category>` option
   - Basic command execution flow
   - Output formatting dispatch (human vs JSON)

3. **Output Formatters** (`/src/doctor/formatters.rs`)
   - Human-readable formatter:
     - Category grouping with status indicators
     - Color-coded output (✓ green, ✗ red, ⚠ yellow)
     - Actionable suggestions formatting
   - JSON formatter:
     - Structured output with version, timestamp, summary
     - Category-based results
     - Machine-readable format

4. **Unit Tests** (use mocks extensively)
   - `src/doctor/mod.rs` - Framework logic tests (mock checks)
   - `src/doctor/formatters.rs` - Output formatting tests (mock data)
   - `src/commands/doctor.rs` - Command parsing tests

5. **Integration Tests** (`/tests/commands/doctor.rs`) (no mocks)
   - Full command execution
   - Output format verification
   - Exit code testing

### Success Criteria
- Diagnostic framework can run checks and aggregate results
- Command parses options correctly
- Both output formats work as specified
- Exit codes match specification (0, 1, 2, 20)

## Phase 2: Installation and Configuration Checks

### Input Resources
- Phase 1 deliverables
- `/src/installation/` - Installation management code
- `/src/config.rs` - Configuration structures

### Deliverables
1. **Installation Checker Module** (`/src/doctor/checks/installation.rs`)
   - Kopi binary detection:
     - Find kopi executable in PATH
     - Verify execute permissions
   - Version check:
     - Compare current version with latest (if network available)
   - Installation directory verification:
     - Check `~/.kopi` exists and is accessible
     - Verify subdirectories: `jdks/`, `shims/`, `cache/`
   - Config file validation:
     - Parse `~/.kopi/config.toml` if present
     - Validate configuration structure
     - Report invalid settings

2. **Permission Checker Module** (`/src/doctor/checks/permissions.rs`)
   - Directory write permissions:
     - Test write access to `~/.kopi`
     - Check subdirectory permissions
   - Binary execute permissions:
     - Verify kopi binary is executable
     - Check shim executables
   - Ownership verification:
     - Detect ownership mismatches
     - Platform-specific permission checks

3. **Platform Integration** (use existing `/src/platform/` modules)
   - Leverage `src/platform/permissions.rs` for permission checks
   - Use `src/platform/file_ops.rs` for file operations
   - Utilize existing cross-platform abstractions

4. **Unit Tests** (use mocks extensively)
   - `src/doctor/checks/installation.rs` - Installation check tests (mock filesystem)
   - `src/doctor/checks/permissions.rs` - Permission check tests (mock OS calls)

5. **Integration Tests** (`/tests/doctor/installation_checks.rs`) (no mocks)
   - Real filesystem checks
   - Permission verification on different platforms

### Success Criteria
- All installation components detected correctly
- Permission issues identified accurately
- Config validation catches common errors
- Clear suggestions for fixing issues

## Phase 3: Shell Integration and PATH Checks

### Input Resources
- Phase 1 & 2 deliverables
- `/src/shell/` - Shell detection and configuration
- `/src/shim/` - Shim management (if implemented)

### Deliverables
1. **Shell Checker Module** (`/src/doctor/checks/shell.rs`)
   - PATH verification:
     - Check if `~/.kopi/shims` is in PATH
     - Verify PATH priority (shims before system Java)
     - Detect PATH syntax issues
   - Shell detection:
     - Identify current shell (bash, zsh, fish, PowerShell, CMD)
     - Find shell configuration files
   - Configuration verification:
     - Check if shell init files contain kopi setup
     - Validate PATH export syntax
   - Shim functionality:
     - Test if shims directory exists
     - Verify shim executables work

2. **Shell Detection Enhancement** (leverage existing platform code)
   - Use `src/platform/shell.rs` for shell detection
   - Enhance existing shell utilities if needed:
     - Configuration file discovery
     - Bash: ~/.bashrc, ~/.bash_profile, ~/.profile
     - Zsh: ~/.zshrc, ~/.zprofile
     - Fish: ~/.config/fish/config.fish
     - PowerShell: $PROFILE paths
   - PATH parsing for different shells

3. **Unit Tests** (use mocks extensively)
   - `src/doctor/checks/shell.rs` - Shell check tests (mock environment)
   - Platform code tests remain in existing `src/platform/` test modules

4. **Integration Tests** (`/tests/doctor/shell_checks.rs`) (no mocks)
   - Real shell detection
   - PATH parsing on different shells
   - Configuration file discovery

### Success Criteria
- Correctly identifies shell type
- Detects PATH configuration issues
- Provides shell-specific fix instructions
- Handles edge cases (multiple shells, custom configs)

## Phase 4: JDK Installation Health Checks

### Input Resources
- Phase 1-3 deliverables
- `/src/storage/` - JDK repository and storage
- `/src/models/jdk.rs` - JDK data structures

### Deliverables
1. **JDK Checker Module** (`/src/doctor/checks/jdks.rs`)
   - Installation enumeration:
     - List all JDKs in `~/.kopi/jdks/`
     - Parse directory names for version info
   - Installation integrity:
     - Verify expected directory structure
     - Check for required executables (java, javac, etc.)
     - Validate executable permissions
   - Version consistency:
     - Match installed versions with directory names
     - Check against metadata if available
   - Disk space analysis:
     - Calculate JDK sizes
     - Check available disk space
     - Warn if low on space

2. **JDK Validation Integration**
   - Reuse existing JDK models from `src/models/jdk.rs`
   - Leverage storage utilities from `src/storage/`
   - Add doctor-specific validation logic in checks module
   - Size calculation and executable verification

3. **Unit Tests** (use mocks extensively)
   - `src/doctor/checks/jdks.rs` - JDK check tests (mock JDK installations)

4. **Integration Tests** (`/tests/doctor/jdk_checks.rs`) (no mocks)
   - Real JDK validation
   - Performance measurement
   - Multiple JDK scenarios

### Success Criteria
- Detects all installed JDKs
- Identifies corrupted installations
- Provides specific remediation steps
- Completes checks quickly (< 100ms per JDK)

## Phase 5: Network and Cache Validation

### Input Resources
- Phase 1-4 deliverables
- `/src/api/` - API client implementation
- `/src/cache/` - Cache management

### Deliverables
1. **Network Checker Module** (`/src/doctor/checks/network.rs`)
   - API connectivity:
     - Test HTTPS connection to api.foojay.io
     - Verify DNS resolution
     - Check response time
   - Proxy detection:
     - Check HTTP_PROXY, HTTPS_PROXY environment variables
     - Validate proxy configuration format
   - TLS/SSL verification:
     - Test certificate validation
     - Check TLS version support
   - Timeout handling:
     - Implement reasonable timeouts (5s default)
     - Graceful failure on timeout

2. **Cache Checker Module** (`/src/doctor/checks/cache.rs`)
   - Cache file verification:
     - Check if metadata cache exists
     - Validate file permissions
   - Format validation:
     - Parse JSON structure
     - Check required fields
   - Staleness check:
     - Compare cache timestamp with current time
     - Use configured max age (default 30 days)
   - Size analysis:
     - Report cache file size
     - Check for abnormal growth

3. **Network Test Integration**
   - Reuse existing API client from `src/api/`
   - Leverage existing network utilities
   - Add doctor-specific timeout and connectivity checks

4. **Unit Tests** (use mocks extensively)
   - `src/doctor/checks/network.rs` - Network check tests (mock HTTP client)
   - `src/doctor/checks/cache.rs` - Cache check tests (mock cache files)

5. **Integration Tests** (`/tests/doctor/network_cache_checks.rs`) (no mocks)
   - Real network connectivity tests
   - Cache validation with real files
   - Proxy scenario testing

### Success Criteria
- Network issues detected accurately
- Cache problems identified clearly
- Timeout handling prevents hanging
- Suggestions guide users to solutions

## Phase 6: Integration and Polish

### Input Resources
- All previous phase deliverables
- User feedback and testing results

### Deliverables
1. **Check Orchestration** (`/src/doctor/orchestrator.rs`)
   - Parallel check execution (where safe)
   - Progress indication for slow checks
   - Check dependency management
   - Category filtering implementation
   - Timeout protection (30s total timeout)

2. **Enhanced Error Context**
   - Integrate with existing `src/error/` infrastructure
   - Add doctor-specific error context and suggestions
   - Platform-specific error messages
   - Common issue patterns and solutions

3. **Performance Optimizations**
   - Lazy check initialization
   - Parallel execution where possible
   - Caching of expensive operations
   - Early exit on critical failures

4. **Documentation Updates**
   - Update `/docs/reference.md` with doctor command
   - Add troubleshooting guide
   - Document common issues and solutions

5. **Comprehensive Tests** (`/tests/doctor/full_suite.rs`)
   - Full diagnostic suite execution
   - Performance benchmarks
   - Error scenario coverage
   - Multi-platform testing

### Success Criteria
- All checks complete within 5 seconds
- Clear, actionable output for all scenarios
- Robust error handling
- Documentation complete and helpful

## Implementation Guidelines

### For Each Phase:
1. Start with `/clear` command to reset context
2. Load this plan.md and relevant phase resources
3. Create necessary test directories (e.g., `/tests/doctor/` for integration tests)
4. Implement deliverables incrementally
5. Run quality checks after each module:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo check`
   - `cargo test --lib --quiet`
6. Update todo list to track progress
7. Commit completed phase before proceeding

### Testing Strategy

Test files are organized following the kopi project structure:
- Command tests: `/tests/commands/doctor.rs` - Main command execution tests
- Feature integration tests: `/tests/doctor/` subdirectory
  - `installation_checks.rs` - Installation and permission checks
  - `shell_checks.rs` - Shell integration checks
  - `jdk_checks.rs` - JDK validation checks
  - `network_cache_checks.rs` - Network and cache checks
  - `full_suite.rs` - Comprehensive test suite

#### Unit Tests (use mocks extensively)
- Test individual checks in isolation
- Mock filesystem, network, and OS operations
- Focus on logic correctness
- Test error conditions thoroughly
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;
      
      #[test]
      fn test_path_check_with_mock_env() {
          let mut mock_env = MockEnvironment::new();
          mock_env.expect_var()
              .with(eq("PATH"))
              .returning(|_| Ok("/usr/bin:/home/user/.kopi/shims".to_string()));
          // Test PATH verification logic
      }
  }
  ```

#### Integration Tests (no mocks)
- Test complete diagnostic workflows
- Verify real system interactions
- Measure actual performance
- Test on multiple platforms
- Example:
  ```rust
  #[test]
  fn test_full_diagnostic_suite() {
      let doctor = DoctorCommand::new();
      let results = doctor.run_all_checks().unwrap();
      
      assert!(results.total_time < Duration::from_secs(5));
      assert!(results.categories.len() == 6);
  }
  ```

### Performance Goals
1. Individual checks: < 100ms each
2. Network checks: < 5s with timeout
3. Total execution: < 5s for all checks
4. JSON formatting: < 10ms
5. Human formatting: < 20ms

### Error Handling Principles
1. Never panic - handle all errors gracefully
2. Provide context for every error
3. Suggest specific fixes
4. Use existing error codes (0, 1, 2, 20)
5. Log verbose details only with --verbose

### Output Quality Standards
1. **Clarity**: Users understand the issue immediately
2. **Actionability**: Every problem has a suggested fix
3. **Brevity**: Concise messages, details with --verbose
4. **Consistency**: Uniform formatting across all checks

### Platform-Specific Considerations

#### Unix (Linux/macOS)
- Use standard permission checks (stat)
- Handle different shell configurations
- Consider snap/flatpak installations

#### Windows
- Handle Windows ACLs properly
- Check both PowerShell and CMD
- Consider antivirus interference
- Handle WSL environments

#### macOS
- Check for Gatekeeper issues
- Handle Homebrew interactions
- Consider system integrity protection

## Design Principles

### Key Requirements
1. **Non-invasive**: Read-only checks, no modifications
2. **Fast**: Complete quickly to encourage regular use
3. **Helpful**: Every issue has a clear solution
4. **Reliable**: No false positives
5. **Comprehensive**: Cover all common issues

### Architecture Principles
1. **Separation of Concerns**: 
   - Check logic resides in `src/doctor/checks/`
   - Platform-specific implementations in `src/platform/`
   - Checks call into platform modules for OS-specific operations
2. **Reuse Existing Code**: Leverage existing platform abstractions rather than duplicating
3. **Testability**: Mock platform operations in unit tests, use real operations in integration tests

### Error Message Guidelines
- **Problem**: What is wrong
- **Impact**: Why it matters
- **Solution**: How to fix it
- **Example format**:
  ```
  ✗ ~/.kopi/shims not found in PATH
    
    Impact: Java commands won't automatically use kopi-managed versions
    
    To fix: Add this line to ~/.zshrc:
      export PATH="$HOME/.kopi/shims:$PATH"
    
    Then reload your shell:
      source ~/.zshrc
  ```

## Future Enhancements (Post-MVP)
1. **Fix suggestions as scripts**: Generate fix scripts users can review and run
2. **Historical tracking**: Compare diagnostics over time
3. **Plugin system**: Allow custom health checks
4. **IDE integration checks**: Verify IDE configurations
5. **Performance profiling**: Identify slow kopi operations

## Next Steps
Begin with Phase 1, implementing the core diagnostic framework with proper result types, command structure, and output formatting. This foundation will support all subsequent check implementations.