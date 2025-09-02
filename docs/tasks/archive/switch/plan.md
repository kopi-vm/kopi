# Version Switching Commands Implementation Plan

## Overview
This document outlines the phased implementation plan for the Kopi version switching commands, which provide flexible JDK version management across three scopes: shell (temporary), project (local), and global (system-wide). These commands enable developers to seamlessly switch between different JDK versions based on their current context and needs.

## Command Syntax
- `kopi current [options]` - Show the currently active JDK version and its source
  - `-q, --quiet` - Show only version number
  - `--json` - Output in JSON format
- `kopi shell <version> [--shell <type>]` (alias: `use`) - Set JDK version for current shell session
- `kopi local <version>` (alias: `pin`) - Set project-specific JDK version
- `kopi global <version>` (alias: `default`) - Set system-wide default JDK version

## Phase 1: Core Infrastructure Enhancements

### Input Resources
- `/docs/tasks/switch/design/` - Complete switching system design
- `/src/version/resolver.rs` - Existing version resolution logic
- `/src/platform/shell.rs` - Current shell detection implementation
- `/docs/adr/` - Architecture Decision Records

### Deliverables
1. **Enhanced Version Resolver** (`/src/version/resolver.rs`)
   - Add `VersionSource` enum:
     ```rust
     pub enum VersionSource {
         Environment(String),      // KOPI_JAVA_VERSION
         ProjectFile(PathBuf),     // .kopi-version or .java-version
         GlobalDefault(PathBuf),   // ~/.kopi/version
         None,                     // No version configured
     }
     ```
   - New method `resolve_version_with_source()` returning `(Option<VersionRequest>, VersionSource)`
   - Maintain backward compatibility with existing `resolve_version()` method
   - Track exact file path for project files
   - Add debug logging for resolution steps

2. **Enhanced Shell Detection** (`/src/platform/shell.rs`)
   - Add **sysinfo** crate dependency to `Cargo.toml`
   - New function `detect_parent_shell_with_path()` returning `Result<(Shell, PathBuf)>`
   - Parent process detection using sysinfo:
     - Get current process PID
     - Find parent process
     - Extract executable path and name
   - Platform-specific logic:
     - Unix: Fallback to SHELL environment variable if parent detection fails
     - Windows: Strict detection with no fallback (fail if detection fails)
   - Remove unreliable environment variable checks on Windows
   - Add comprehensive error handling

3. **Shell Detection Tests** (`/src/platform/shell.rs` test module)
   - Unit tests for shell type detection from executable names
   - Integration tests for parent process detection
   - Platform-specific test cases
   - Error condition testing

4. **Version Resolver Tests** (`/src/version/resolver.rs` test module)
   - Test source tracking for all resolution paths
   - Test priority ordering (env > project > global)
   - Test file traversal for project files
   - Mock filesystem for unit tests

### Success Criteria
- Version resolver accurately tracks resolution source
- Shell detection reliably identifies parent shell and path
- All existing version resolution functionality remains intact
- Tests pass on all supported platforms
- sysinfo crate integrated successfully

## Phase 2: Current Command Implementation

### Input Resources
- Phase 1 deliverables
- `/docs/tasks/switch/design/current-command.md`
- `/src/commands/mod.rs` - Command structure
- `/src/main.rs` - CLI definition

### Deliverables
1. **Current Command Module** (`/src/commands/current.rs`)
   - Command handler implementation
   - Output formatting logic:
     - Standard format: version, source, installation status
     - Quiet format: version only
     - JSON format: structured output
   - Version source display:
     - "Environment (KOPI_JAVA_VERSION)" 
     - "Project file (.kopi-version)" with relative path
     - "Global default (~/.kopi/version)"
     - "Not configured"
   - Installation status checking via JdkRepository
   - Warning messages for uninstalled versions

2. **CLI Integration** (update `/src/main.rs`)
   - Add `Current` command to Commands enum with clap derive:
     ```rust
     #[derive(Subcommand)]
     enum Commands {
         // ... existing commands
         /// Show the currently active JDK version
         Current {
             /// Show only version number
             #[arg(short = 'q', long)]
             quiet: bool,
             /// Output in JSON format
             #[arg(long)]
             json: bool,
         },
     }
     ```
   - Command routing in main match statement

3. **JSON Output Support** (`/src/commands/current.rs`)
   - Define output structure:
     ```rust
     #[derive(Serialize)]
     struct CurrentOutput {
         version: Option<String>,
         source: String,
         source_path: Option<String>,
         installed: bool,
         installation_path: Option<String>,
     }
     ```
   - Serialize using serde_json

4. **Unit Tests** (`src/commands/current.rs` test module)
   - Test all output formats
   - Test with various version sources
   - Test uninstalled version warnings
   - Mock JdkRepository for installation checks

5. **Integration Tests** (`/tests/current_command.rs`)
   - Full command execution with real filesystem
   - Test with actual version files
   - Verify output formatting
   - Test JSON parsing

### Success Criteria
- Command shows correct version and source
- All output formats work correctly
- Installation status accurately reported
- Warnings displayed for uninstalled versions
- Help text clear and informative

## Phase 3: Global Command Implementation

### Input Resources
- Phase 1 & 2 deliverables
- `/docs/tasks/switch/design/global-command.md`
- `/src/commands/install.rs` - Reference for auto-installation
- Platform symlink utilities

### Deliverables
1. **Global Command Module** (`/src/commands/global.rs`)
   - Version parsing and validation
   - Installation verification (mandatory)
   - File management:
     - Write to `~/.kopi/version` (not `~/.kopi/default-version`)
     - Create parent directory if needed
     - Atomic file writing
   - Symlink management:
     - Create/update `~/.kopi/default` symlink
     - Point to specific JDK installation
     - Platform-specific implementation
   - Auto-installation integration:
     - Check if version installed
     - Trigger auto-installation if not
     - Fail if user declines installation

2. **Version File Migration** (`/src/commands/global.rs`)
   - Check for legacy `~/.kopi/default-version` file
   - Migrate to new `~/.kopi/version` location
   - One-time migration with user notification

3. **Auto-Installation Integration**
   - Use existing `AutoInstaller` from install command
   - Mandatory installation (no skip option)
   - Progress feedback during installation
   - Clear error messages if installation fails

4. **CLI Integration** (update `/src/main.rs`)
   - Add `Global` command and `Default` alias:
     ```rust
     /// Set the global default JDK version
     Global {
         /// JDK version to set as global default
         version: String,
     },
     /// Set the global default JDK version (alias for global)
     #[command(alias = "default")]
     Default {
         /// JDK version to set as global default
         version: String,
     },
     ```

5. **Unit Tests** (`src/commands/global.rs` test module)
   - Test version validation
   - Test file writing with mocked filesystem
   - Test installation verification
   - Test symlink creation

6. **Integration Tests** (`/tests/global_command.rs`)
   - Full command execution
   - Verify file creation at correct path
   - Test symlink functionality
   - Test with uninstalled versions

### Success Criteria
- Global version file created at `~/.kopi/version`
- Installation verification prevents setting uninstalled versions
- Symlinks created correctly on all platforms
- Auto-installation triggered when needed
- Clear error messages for all failure cases

## Phase 4: Local Command Implementation

### Input Resources
- Phase 1-3 deliverables
- `/docs/tasks/switch/design/local-command.md`
- Version file format specifications

### Deliverables
1. **Local Command Module** (`/src/commands/local.rs`)
   - Version parsing and validation
   - File creation logic:
     - Always create `.kopi-version` file
     - Format: `distribution@version` or just `version`
     - Create in current directory
   - Installation checking (optional):
     - Check if version installed
     - Offer auto-installation if not
     - Create file regardless of user choice
   - User feedback:
     - Confirm file creation
     - Warn if version not installed
     - Suggest installation command

2. **Project File Support** (`/src/commands/local.rs`)
   - Write `.kopi-version` in Kopi format
   - Support for reading `.java-version` (compatibility)
   - No modification of existing `.java-version` files
   - Clear precedence: `.kopi-version` > `.java-version`

3. **Auto-Installation Prompt**
   - Optional installation (can decline)
   - Clear prompt explaining implications
   - File created even if installation declined
   - Different from global command behavior

4. **CLI Integration** (update `/src/main.rs`)
   - Add `Local` command and `Pin` alias:
     ```rust
     /// Set the project-specific JDK version
     Local {
         /// JDK version to use for this project
         version: String,
     },
     /// Set the project-specific JDK version (alias for local)
     #[command(alias = "pin")]
     Pin {
         /// JDK version to use for this project
         version: String,
     },
     ```

5. **Unit Tests** (`src/commands/local.rs` test module)
   - Test file creation
   - Test version format validation
   - Test optional installation flow
   - Mock filesystem and auto-installer

6. **Integration Tests** (`/tests/local_command.rs`)
   - Create actual `.kopi-version` files
   - Test in various directory structures
   - Verify version resolution picks up new file
   - Test with declined installation

### Success Criteria
- `.kopi-version` file created in current directory
- File created even when JDK not installed
- Optional auto-installation works correctly
- Clear user feedback for all scenarios
- Version resolution correctly uses new file

## Phase 5: Shell Command Implementation

### Input Resources
- All previous phase deliverables
- `/docs/tasks/switch/design/shell-command.md`
- Enhanced shell detection from Phase 1
- Platform process utilities

### Deliverables
1. **Shell Command Module** (`/src/commands/shell.rs`)
   - Version parsing and validation
   - Shell detection using enhanced `detect_parent_shell_with_path()`
   - Installation verification (mandatory like global)
   - Environment setup:
     - Set `KOPI_JAVA_VERSION` environment variable
     - Preserve existing environment
   - Shell launching:
     - Unix: Use exec() to replace current process
     - Windows: Spawn new process and wait
   - Error handling for detection failures

2. **Shell Type Override** (`/src/commands/shell.rs`)
   - `--shell` option for manual shell selection
   - Validate shell type is supported
   - Find shell executable in PATH
   - Override detection when specified

3. **Platform-Specific Execution**
   - Unix implementation:
     ```rust
     // Use platform::process::exec_replace()
     let mut cmd = Command::new(&shell_path);
     cmd.env("KOPI_JAVA_VERSION", version.to_string());
     platform::process::exec_replace(cmd)?;
     ```
   - Windows implementation:
     ```rust
     // Spawn and wait
     let status = Command::new(&shell_path)
         .env("KOPI_JAVA_VERSION", version.to_string())
         .status()?;
     std::process::exit(status.code().unwrap_or(1));
     ```

4. **Auto-Installation Integration**
   - Mandatory installation (like global command)
   - Cannot use uninstalled version
   - Clear error if user declines

5. **CLI Integration** (update `/src/main.rs`)
   - Add `Shell` command and `Use` alias:
     ```rust
     /// Set JDK version for current shell session
     Shell {
         /// JDK version to use
         version: String,
         /// Override shell detection
         #[arg(long)]
         shell: Option<String>,
     },
     /// Set JDK version for current shell session (alias for shell)
     #[command(alias = "use")]
     Use {
         /// JDK version to use
         version: String,
         /// Override shell detection
         #[arg(long)]
         shell: Option<String>,
     },
     ```

6. **Unit Tests** (`src/commands/shell.rs` test module)
   - Test shell detection integration
   - Test environment variable setup
   - Test shell override functionality
   - Mock process execution

7. **Integration Tests** (`/tests/shell_command.rs`)
   - Test actual shell spawning
   - Verify environment variable set correctly
   - Test on multiple shell types
   - Test detection failures

### Success Criteria
- Shell detection works reliably on all platforms
- New shell launched with correct environment
- Version switching works in spawned shell
- Clear error messages for detection failures
- Manual shell override works correctly

## Implementation Guidelines

### For Each Phase:
1. Start with `/clear` command to reset context
2. Load this plan.md and relevant phase resources
3. Implement deliverables incrementally
4. Run quality checks after each module:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo check`
   - `cargo test --lib`
5. Commit completed phase before proceeding

### Testing Strategy

#### Unit Tests (use mocks extensively)
- Test command logic in isolation
- Mock JdkRepository, AutoInstaller, filesystem
- Focus on command behavior and error handling
- Test all code paths thoroughly
- Example:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use mockall::*;
      
      #[test]
      fn test_global_requires_installation() {
          let mut mock_repo = MockJdkRepository::new();
          mock_repo.expect_is_installed()
              .returning(|_| false);
          // Verify command fails without installation
      }
  }
  ```

#### Integration Tests (no mocks)
- Test complete command workflows
- Use real filesystem and version files
- Verify actual behavior end-to-end
- Test shell integration
- Example:
  ```rust
  #[test]
  fn test_version_switching_workflow() {
      // Set global version
      run_kopi(&["global", "17"]);
      
      // Create local version
      run_kopi(&["local", "21"]);
      
      // Verify current shows local
      let output = run_kopi(&["current"]);
      assert!(output.contains("21"));
      assert!(output.contains(".kopi-version"));
  }
  ```

### Version Resolution Priority
Always maintain the three-tier priority system:
1. **Shell scope** (highest): `KOPI_JAVA_VERSION` environment variable
2. **Project scope** (medium): `.kopi-version` or `.java-version` files
3. **Global scope** (lowest): `~/.kopi/version` file

### Error Message Guidelines
All commands should provide clear, actionable error messages:
```
Error: JDK version 'temurin@21' is not installed

To install this version, run:
  kopi install temurin@21

Or to see available versions:
  kopi search 21
```

### Platform Considerations

#### Unix (Linux/macOS)
- Use exec() for shell command (zero overhead)
- Symlinks for global default
- Shell detection via parent process or SHELL variable

#### Windows
- Spawn new process for shell command
- Junction points or copying for defaults
- Strict shell detection (no fallback)
- Handle antivirus delays

## Dependency Management

### New Dependencies Required
- **sysinfo** crate (^0.31): For parent process detection in shell command
  ```toml
  [dependencies]
  sysinfo = "0.31"
  ```

### Existing Dependencies Used
- **clap**: Command-line parsing with derive
- **serde/serde_json**: JSON output formatting
- **dirs**: User directory paths
- **which**: Finding shell executables

## Design Principles

### Command Behavior Consistency
1. **Installation Requirements**:
   - `current`: No installation required (read-only)
   - `global` and `shell`: Installation mandatory
   - `local`: Installation optional

2. **File Locations**:
   - Global: `~/.kopi/version` (not `default-version`)
   - Local: `.kopi-version` in current directory
   - Shell: Environment variable only

3. **Error Handling**:
   - Always check version format validity
   - Provide clear, actionable error messages
   - Show installation commands when JDK missing

### User Experience Goals
1. **Intuitive Commands**: Aliases match other version managers (use, pin, default)
2. **Clear Feedback**: Always show what changed and where
3. **Team Friendly**: Local command creates file even without installation
4. **Fast Operation**: Minimal overhead for all commands

## Success Metrics
- All commands complete in < 100ms (excluding installation)
- Version resolution follows documented priority
- Error messages guide users to solutions
- Commands work consistently across platforms
- Integration with existing Kopi features seamless

## Next Steps
Begin with Phase 1, focusing on the core infrastructure enhancements needed by all commands. The enhanced version resolver and shell detection are foundational components that subsequent phases will build upon.