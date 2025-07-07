# Shims System Implementation Plan

## Overview
This document outlines the phased implementation plan for the Kopi shims system, which provides transparent JDK version switching by intercepting Java tool invocations and routing them to the appropriate JDK version based on project configuration.

## Command Syntax
- `kopi setup` - Initial setup including shims directory creation and PATH configuration
- `kopi shim add <tool>` - Create shim for a specific tool
- `kopi shim remove <tool>` - Remove shim for a specific tool
- `kopi shim list` - List installed shims
- `kopi shim list --available` - Show available tools without creating shims
- `kopi shim verify` - Check and repair shims

## Phase 1: Core Shim Binary Implementation

### Input Resources
- `/docs/tasks/shims/design/` - Complete shims system design
- `/docs/adr/` - Architecture Decision Records
- `/src/models/jdk.rs` - JDK models and structures

### Deliverables
1. **Shim Binary Module** (`/src/shim/mod.rs`)
   - Tool name detection from argv[0]
   - Version resolution logic (.kopi-version, .java-version)
   - Environment variable caching (KOPI_JAVA_VERSION)
   - JDK path resolution
   - Platform-specific execution (exec on Unix, CreateProcess on Windows)
   - Performance optimizations (< 20ms overhead)

2. **Version Resolution Module** (`/src/shim/version_resolver.rs`)
   - Directory traversal for version files
   - Efficient file reading with pre-allocated buffers
   - Version file parsing (.kopi-version format: `distribution@version`)
   - Legacy .java-version support
   - Environment variable fallback
   - Global default resolution

3. **Process Execution Module** (`/src/shim/executor.rs`)
   - Unix implementation using exec()
   - Windows implementation using CreateProcess
   - Argument forwarding
   - Environment preservation
   - Error code propagation

4. **Unit Tests** (use mocks extensively)
   - `src/shim/mod.rs` - Tool name detection and path resolution tests (mock filesystem)
   - `src/shim/version_resolver.rs` - Version file parsing tests (mock file operations)
   - `src/shim/executor.rs` - Process execution tests (mock process operations)

5. **Integration Tests** (`/tests/shim_integration.rs`) (no mocks)
   - Real filesystem version file detection
   - Actual process execution on different platforms
   - Performance measurement (< 20ms overhead)
   - Error handling scenarios

### Success Criteria
- Shim binary detects tool name correctly
- Version resolution finds appropriate JDK
- Process execution maintains < 20ms overhead
- Arguments and environment pass through correctly
- Exit codes propagate properly

## Phase 2: Shim Management Infrastructure

### Input Resources
- Phase 1 deliverables
- `/docs/tasks/shims/design/10-shim-installation-management.md`
- Platform-specific shim creation requirements

### Deliverables
1. **Shim Installer Module** (`/src/shim/installer.rs`)
   - Shim directory creation (~/.kopi/shims/)
   - Unix: Symlink creation to kopi-shim binary
   - Windows: Individual .exe file copying
   - Shim verification and repair
   - Atomic shim updates
   - Cleanup on removal

2. **Tool Registry Module** (`/src/shim/tools.rs`)
   - Standard JDK tool definitions
   - Distribution-specific tool mappings
   - Tool availability checking
   - Version-based tool filtering (e.g., GraalVM js removed in v23+)
   - Tool categorization (core, debug, monitoring, etc.)

3. **Platform Utilities** (`/src/platform/shell.rs`)
   - Unix symlink operations
   - Windows file copying
   - Permission management
   - PATH environment detection
   - Shell-specific PATH update instructions

4. **Unit Tests** (use mocks extensively)
   - `src/shim/installer.rs` - Shim creation and removal tests (mock filesystem)
   - `src/shim/tools.rs` - Tool registry and filtering tests (mock data)
   - `src/platform/shell.rs` - Platform operation tests (mock OS operations)

5. **Integration Tests** (`/tests/shim_management.rs`) (no mocks)
   - Real shim creation on different platforms
   - Symlink/file verification
   - Permission checks
   - Tool discovery from actual JDK installations

### Success Criteria
- Shims created correctly on all platforms
- Symlinks point to correct binary (Unix)
- Individual executables work (Windows)
- Tool registry accurately reflects available tools
- Shim verification detects and fixes issues

## Phase 3: Command Implementation and CLI Integration

### Input Resources
- Phase 1 & 2 deliverables
- `/src/main.rs` - Existing CLI structure
- `/docs/adr/001-kopi-command-structure.md` - Command guidelines

### Deliverables
1. **Setup Command Enhancement** (`/src/commands/setup.rs`)
   - Create shims directory
   - Build kopi-shim binary
   - Install default shims for standard tools
   - Generate PATH update instructions
   - Shell detection and configuration

2. **Shim Commands** (`/src/commands/shim.rs`)
   - `add` subcommand - Create specific tool shim
   - `remove` subcommand - Remove specific tool shim
   - `list` subcommand - Show installed shims
   - `verify` subcommand - Check and repair shims
   - `--available` flag for listing without creation

3. **CLI Integration** (update `/src/main.rs`)
   - Add shim subcommand group with clap derive
   - Command-line options:
     - `--force`: Override existing shims
     - `--all`: Apply to all available tools
     - `--distribution`: Filter by distribution
   - Help text and examples

4. **Install Command Integration** (`/src/commands/install.rs` enhancement)
   - After successful JDK installation (after `finalize_installation`):
     - Call shim verification/creation functionality
     - Detect distribution-specific tools from the installed JDK
     - Create missing shims based on:
       - Standard JDK tools list
       - Distribution-specific tools (e.g., GraalVM's `gu`, `native-image`)
       - User configuration preferences
     - Report newly created shims to the user
   - Example output:
     ```
     Successfully installed graalvm 21.0.2 to ~/.kopi/jdks/graalvm-21.0.2
     
     Verifying shims...
     Created 3 new shims:
       - gu
       - native-image
       - polyglot
     
     To use this JDK, run: kopi use graalvm@21
     ```

5. **Shell Integration Module** (`/src/shell/mod.rs`)
   - Shell detection (bash, zsh, fish, PowerShell)
   - PATH update instruction generation
   - RC file detection (.bashrc, .zshrc, etc.)
   - Manual instruction formatting

6. **Unit Tests** (use mocks extensively)
   - `src/commands/setup.rs` - Setup logic tests (mock filesystem)
   - `src/commands/shim.rs` - Command logic tests (mock shim operations)
   - `src/commands/install.rs` - Post-install shim creation tests (mock shim operations)
   - `src/shell/mod.rs` - Shell detection tests (mock environment)

7. **Integration Tests** (`/tests/shim_commands.rs`) (no mocks)
   - Full command execution testing
   - Shell configuration verification
   - Multi-tool shim creation
   - Error message validation
   - Post-install shim creation verification

### Success Criteria
- `kopi setup` creates functioning shims
- `kopi shim add java` creates java shim
- `kopi shim verify` detects and fixes issues
- Clear PATH configuration instructions
- Shell-specific guidance provided

## Phase 4: Auto-Installation and Error Handling

### Input Resources
- Phase 1-3 deliverables
- `/docs/tasks/shims/design/08-error-handling.md`
- Install command implementation

### Deliverables
1. **Auto-Install Module** (`/src/shim/auto_install.rs`)
   - Missing JDK detection
   - Configuration checking (auto-install enabled)
   - User prompting (if configured)
   - Lock file coordination
   - Install command invocation
   - Progress indication
   - Timeout protection (5 minutes default)

2. **Error Handler Enhancement** (`/src/shim/errors.rs`)
   - Clear error categories:
     - No version found
     - JDK not installed
     - Tool not found
     - Permission denied
   - Actionable error messages
   - Suggested fixes
   - Error code mapping

3. **Configuration Integration** (`/src/config/mod.rs` updates)
   - Auto-install settings
   - Prompt preferences
   - Timeout configuration
   - Default distribution settings

4. **Unit Tests** (use mocks extensively)
   - `src/shim/auto_install.rs` - Auto-install logic tests (mock install)
   - `src/shim/errors.rs` - Error formatting tests (mock conditions)
   - Configuration loading tests (mock config files)

5. **Integration Tests** (`/tests/auto_install.rs`) (no mocks)
   - Missing JDK scenarios
   - Concurrent shim coordination
   - User prompt simulation
   - Timeout handling

### Success Criteria
- Missing JDKs trigger auto-install (when enabled)
- Clear prompts for user confirmation
- Concurrent shims coordinate properly
- Helpful error messages guide users
- Timeouts prevent hanging

## Phase 5: Performance Optimization and Security

### Input Resources
- All previous phase deliverables
- `/docs/tasks/shims/design/07-performance-optimizations.md`
- `/docs/tasks/shims/design/12-security.md`

### Deliverables
1. **Performance Optimizations** (`/src/shim/` modules)
   - Binary size reduction (< 1MB target)
   - Direct path construction
   - Memory allocation minimization
   - Compiler optimization flags
   - Profile-guided optimization

2. **Security Enhancements** (`/src/shim/security.rs`)
   - Path validation (stay within ~/.kopi)
   - Symlink target verification
   - Input sanitization
   - Permission verification
   - No privilege escalation

3. **Benchmark Suite** (`/benches/shim_bench.rs`)
   - Tool invocation overhead measurement
   - Version resolution timing
   - Process execution benchmarks
   - Comparison with direct execution

4. **Security Tests** (`/tests/shim_security.rs`)
   - Path traversal prevention
   - Symlink attack prevention
   - Input validation
   - Permission checks

5. **Documentation Updates**
   - Update `/docs/reference.md` with shim details
   - Security considerations
   - Performance characteristics
   - Troubleshooting guide

### Success Criteria
- Shim overhead < 20ms consistently
- Binary size < 1MB
- All security checks pass
- No privilege escalation possible
- Documentation complete

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
- Test individual components in isolation
- Mock filesystem, process, and OS operations
- Focus on logic correctness
- Test error conditions thoroughly
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;
      
      #[test]
      fn test_version_resolution_with_mock_fs() {
          let mut mock_fs = MockFileSystem::new();
          mock_fs.expect_read_file()
              .returning(|_| Ok("temurin@21".to_string()));
          // Test version resolution logic
      }
  }
  ```

#### Integration Tests (no mocks)
- Test complete shim workflow end-to-end
- Verify real filesystem operations
- Measure actual performance
- Test on multiple platforms
- Example:
  ```rust
  #[test]
  fn test_shim_execution_overhead() {
      let start = Instant::now();
      // Execute shim with real JDK
      let result = Command::new("~/.kopi/shims/java")
          .arg("-version")
          .output()
          .unwrap();
      let elapsed = start.elapsed();
      assert!(elapsed < Duration::from_millis(20));
  }
  ```

### Performance Priorities
1. Tool name detection: < 1ms
2. Version resolution (cached): < 1ms  
3. Version resolution (file): < 5ms
4. Process execution: < 5ms (Unix), < 20ms (Windows)
5. Total overhead target: 1-20ms

### Security Considerations
1. Validate all paths stay within ~/.kopi
2. Verify symlink targets before following
3. Sanitize version strings and tool names
4. Check executable permissions
5. Never require elevated privileges
6. Limit exposed tools to curated list

### Platform-Specific Considerations

#### Unix (Linux/macOS)
- Use symlinks for efficiency
- Leverage exec() for zero-overhead process replacement
- Handle different shells (bash, zsh, fish)

#### Windows
- Create individual .exe files
- Use CreateProcess and wait
- Handle PowerShell and CMD
- Consider antivirus scanning delays

## Next Steps
Begin with Phase 1, focusing on building the core shim binary with efficient tool detection, version resolution, and process execution capabilities while maintaining the performance target of < 20ms overhead.