# Shell Command Design

## Overview

The `kopi shell` command (alias: `kopi use`) sets a temporary JDK version for the current shell session by launching a new shell subprocess with the specified JDK version activated. This is the highest priority version setting and overrides all other configurations.

**Implementation Note**: This design leverages existing platform-specific code in `src/platform/` for process execution and shell types. The current unreliable `detect_shell()` function will be enhanced with parent process detection as part of this implementation, making it more reliable for all shell-related operations in Kopi.

## Command Syntax

```bash
kopi shell <version> [options]
kopi use <version> [options]     # Alias
```

### Current Implementation Status

**⚠️ The `kopi shell` command is not yet implemented**. The current CLI in `main.rs` has a `use` command but it shows a placeholder message: "Switching to JDK {version} (not yet implemented)".

**Available Infrastructure**: 
- Platform-specific shell detection in `src/platform/shell.rs` (though unreliable on Windows as noted)
- Process execution via `src/platform/process.rs::exec_replace()`
- Auto-installation system in shim infrastructure

**CLI Structure Difference**: The current implementation has `kopi use` as a standalone command rather than an alias for `kopi shell`. The design should be updated to match the current CLI structure or the CLI should be updated to match this design.

### Arguments

- `<version>`: The JDK version to activate
  - Format: `<major>`, `<major.minor.patch>`, or `<distribution>@<version>`
  - Examples: `17`, `17.0.5`, `temurin@17.0.5`

### Options

- `--shell <shell>`: Manually specify which shell to launch
  - Options: `bash`, `zsh`, `fish`, `powershell`, `cmd`
  - Default: Auto-detect from parent process

## Behavior

### Primary Function

Launches a new shell subprocess with the `KOPI_JAVA_VERSION` environment variable set. The shell type is determined by examining kopi's parent process or can be manually specified.

### Supported Shell Types

The command supports the following shell types (defined in `platform/shell.rs`):
- `Bash`: Bourne Again Shell  
- `Zsh`: Z Shell
- `Fish`: Friendly Interactive Shell
- `PowerShell`: Windows PowerShell or PowerShell Core
- `Cmd`: Windows Command Prompt

### Shell Detection

The command detects the parent shell by:
1. Getting the parent process ID
2. Reading the parent process information
3. Determining the shell type from the executable path (not process name)
4. Falling back to SHELL environment variable if detection fails (Unix only)

**Note**: 
- Process names can be arbitrarily changed, so executable paths are more reliable
- The detected executable path is used directly to launch the new shell, ensuring the same shell binary is used
- This avoids PATH resolution issues where a different shell binary might be found
- On Windows, environment variables like `PSModulePath` cannot reliably detect the current shell since they exist system-wide
- Only parent process detection via executable path provides accurate results
- If the parent process is not a shell (e.g., IDE, terminal emulator), explicit `--shell` option is required on Windows

### Auto-Installation

If the requested version is not installed:
1. Check if auto-installation is enabled in config
2. Prompt user for confirmation (if configured)
3. Trigger installation via `AutoInstaller`
4. Launch shell after successful installation

### Version Validation

Before launching the shell:
1. Check if the requested version is installed
2. If not installed, trigger auto-installation process
3. Resolve version aliases (e.g., `17` → `17.0.5`)
4. Validate distribution if specified

## Implementation Details

### Platform Module Integration

The implementation leverages existing platform-specific code in `src/platform/`:

1. **Process Execution** (`platform/process.rs`):
   - `exec_replace()`: Platform-specific process execution
   - Unix: Uses `exec()` to replace current process
   - Windows: Spawns subprocess and exits with its code

2. **Shell Types** (`platform/shell.rs`):
   - Existing `Shell` enum includes: `Bash`, `Zsh`, `Fish`, `PowerShell`, `Cmd`, `Unknown(String)`
   - `detect_shell()`: Current implementation uses environment variables (unreliable on Windows)
   - `get_config_file()`: Returns shell configuration file path
   - `get_shell_name()`: Returns display name for the shell
   - **Note**: The existing `detect_shell()` function will be enhanced as part of this implementation

3. **Constants** (`platform/constants.rs`):
   - `executable_extension()`: Returns ".exe" on Windows, "" on Unix
   - `path_separator()`: Returns ";" on Windows, ":" on Unix
   - `with_executable_extension()`: Adds platform-specific extension to executable names

### Command Flow

```rust
pub fn execute_shell_command(version: &str, shell_override: Option<&str>) -> Result<()> {
    // 1. Parse and validate version
    let version_request = parse_version_request(version)?;
    let resolved_version = resolve_version(&version_request)?;
    
    // 2. Check if version is installed
    if !is_version_installed(&resolved_version)? {
        // 3. Trigger auto-installation if enabled
        let config = KopiConfig::load()?;
        let auto_installer = AutoInstaller::new(&config);
        
        if auto_installer.should_auto_install() {
            if auto_installer.prompt_user(version)? {
                auto_installer.install_jdk(&version_request)?;
            } else {
                return Err(KopiError::VersionNotInstalled(version.to_string()));
            }
        } else {
            return Err(KopiError::VersionNotInstalled(version.to_string()));
        }
    }
    
    // 4. Detect or use specified shell
    let (shell_type, shell_path) = if let Some(shell_name) = shell_override {
        // When explicitly specified, find the shell in PATH
        // This allows users to launch a different shell than their current one
        let shell_type = parse_shell_name(shell_name)?;
        let shell_path = find_shell_in_path(&shell_type)?;
        (shell_type, shell_path)
    } else {
        // Use the exact path of the detected parent shell
        // This ensures we launch the same shell binary that launched kopi
        detect_parent_shell_with_path()?
    };
    
    // 5. Launch shell subprocess with environment
    launch_shell_with_version(shell_type, shell_path, &resolved_version)
}
```

### Parent Shell Detection

**Note**: This function needs to be implemented as an enhancement to the existing `platform/shell.rs` module. The current `detect_shell()` function relies on environment variables, which are unreliable on Windows.

**Windows Error Handling**: On Windows, if parent process detection fails at any stage, the function immediately returns an error requiring the user to specify `--shell` explicitly. Failure cases include:
- Current process not found in process list
- Parent process ID not available
- Parent process not found
- Executable path not retrievable from parent process
- Unknown shell type

This prevents incorrect shell detection and ensures the right shell syntax is used.

```rust
// This would be added to platform/shell.rs or a new module
fn detect_parent_shell_with_path() -> Result<(Shell, PathBuf)> {
    use sysinfo::{System, SystemExt, ProcessExt, PidExt};
    
    // Get current process ID
    let current_pid = std::process::id();
    
    // Get system information
    let mut system = System::new();
    system.refresh_processes();
    
    // Find parent process
    if let Some(current_process) = system.process(PidExt::from_u32(current_pid)) {
        if let Some(parent_pid) = current_process.parent() {
            if let Some(parent_process) = system.process(parent_pid) {
                // Get the executable path
                if let Some(exe_path) = parent_process.exe() {
                    log::debug!("Parent process executable: {:?}", exe_path);
                    
                    // Get just the file name from the path
                    if let Some(file_name) = exe_path.file_name() {
                        let file_str = file_name.to_string_lossy();
                        log::debug!("Parent process file name: {}", file_str);
                        
                        // Check executable file name and return both type and path
                        match file_str.as_ref() {
                            "bash" | "bash.exe" => return Ok((Shell::Bash, exe_path.to_path_buf())),
                            "zsh" | "zsh.exe" => return Ok((Shell::Zsh, exe_path.to_path_buf())),
                            "fish" | "fish.exe" => return Ok((Shell::Fish, exe_path.to_path_buf())),
                            "powershell" | "powershell.exe" => return Ok((Shell::PowerShell, exe_path.to_path_buf())),
                            "pwsh" | "pwsh.exe" => return Ok((Shell::PowerShell, exe_path.to_path_buf())),
                            "cmd" | "cmd.exe" => return Ok((Shell::Cmd, exe_path.to_path_buf())),
                            _ => {
                                log::debug!("Parent process is not a recognized shell: {}", file_str);
                                #[cfg(windows)]
                                {
                                    return Err(KopiError::ShellDetectionError(format!(
                                        "Parent process '{}' is not a recognized shell. Please specify shell type with --shell option",
                                        file_str
                                    )));
                                }
                                // On Unix, continue to try other detection methods
                            }
                        }
                    }
                }
                
                // If we can't get the executable path on Windows, fail immediately
                #[cfg(windows)]
                {
                    log::error!("Failed to get executable path for parent process");
                    return Err(KopiError::ShellDetectionError(
                        "Cannot determine parent shell executable path. Please specify shell type with --shell option".to_string()
                    ));
                }
            }
        }
    }
    
    // On Windows, we cannot proceed without parent process detection
    #[cfg(windows)]
    {
        return Err(KopiError::ShellDetectionError(
            "Cannot detect parent shell on Windows. Please specify shell type with --shell option".to_string()
        ));
    }
    
    // Unix: Fallback to environment detection
    #[cfg(not(windows))]
    {
        let shell_type = detect_shell_from_env()?;
        let shell_path = find_shell_in_path(&shell_type)?;
        Ok((shell_type, shell_path))
    }
}

fn find_shell_in_path(shell: &Shell) -> Result<PathBuf> {
    let shell_name = match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => if cfg!(windows) { "powershell" } else { "pwsh" },
        Shell::Cmd => "cmd",
    };
    
    which::which(shell_name)
        .map_err(|_| KopiError::ShellNotFound(shell_name.to_string()))
}

fn parse_shell_name(name: &str) -> Result<Shell> {
    match name.to_lowercase().as_str() {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" | "pwsh" => Ok(Shell::PowerShell),
        "cmd" => Ok(Shell::Cmd),
        _ => Err(KopiError::UnsupportedShell(name.to_string())),
    }
}

fn detect_shell_from_env() -> Result<Shell> {
    // Check SHELL environment variable (Unix)
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("bash") {
            return Ok(Shell::Bash);
        } else if shell.contains("zsh") {
            return Ok(Shell::Zsh);
        } else if shell.contains("fish") {
            return Ok(Shell::Fish);
        }
    }
    
    // Windows-specific detection
    #[cfg(windows)]
    {
        // On Windows, we cannot reliably detect the shell from environment variables
        // Parent process detection is required
        // Note: KopiError::ShellDetectionError should be defined in error module
        return Err(KopiError::ShellDetectionError(
            "Cannot detect shell on Windows. Please specify shell type with --shell option".to_string()
        ));
    }
    
    // Default fallback
    Ok(Shell::Bash)
}
```

### Shell Subprocess Launch

```rust
fn launch_shell_with_version(shell_type: Shell, shell_path: PathBuf, version: &str) -> Result<()> {
    use crate::platform::process::exec_replace;
    
    log::debug!("Launching shell: {:?} at {:?}", shell_type, shell_path);
    
    // Build arguments for the shell
    let mut args = Vec::new();
    
    // Add shell-specific initialization flags
    match shell_type {
        Shell::Bash | Shell::Zsh => {
            // Load user's shell configuration
            args.push(OsString::from("-l")); // Login shell
        }
        Shell::Fish => {
            args.push(OsString::from("-l")); // Login shell
        }
        Shell::PowerShell => {
            args.push(OsString::from("-NoExit"));
        }
        Shell::Cmd => {
            args.push(OsString::from("/K")); // Keep window open
        }
    }
    
    // Set KOPI_JAVA_VERSION environment variable
    env::set_var("KOPI_JAVA_VERSION", version);
    
    // Use platform-specific process execution
    // On Unix: exec() replaces the current process
    // On Windows: spawns subprocess and exits with its status code
    let err = exec_replace(&shell_path, args);
    
    // exec_replace only returns on error
    Err(KopiError::SystemError(format!(
        "Failed to execute shell: {}",
        err
    )))
}
```

## User Experience

### Success Scenarios

#### Launching Shell with Installed Version
```bash
$ kopi shell 17
[kopi] Activating JDK 17.0.5 in new bash shell...
bash-5.1$ java -version
openjdk version "17.0.5" 2022-10-18
OpenJDK Runtime Environment Temurin-17.0.5+8 (build 17.0.5+8)
bash-5.1$ echo $KOPI_JAVA_VERSION
17.0.5
```

#### Specifying Shell Type
```bash
$ kopi shell 17 --shell zsh
[kopi] Activating JDK 17.0.5 in new zsh shell...
% java -version
openjdk version "17.0.5" 2022-10-18
```

#### Windows Shell Usage
```bash
# From cmd.exe
C:\> kopi shell 17 --shell cmd
[kopi] Activating JDK 17.0.5 in new cmd shell...
C:\> java -version
openjdk version "17.0.5" 2022-10-18

# From PowerShell
PS C:\> kopi shell 17 --shell powershell
[kopi] Activating JDK 17.0.5 in new PowerShell session...
PS C:\> java -version
openjdk version "17.0.5" 2022-10-18
```

#### Auto-Installation Flow
```bash
$ kopi shell 21
JDK 21 is not installed. Would you like to install it now? [Y/n] y
[kopi] Installing JDK 21...
[=====================================>] 100% 
[kopi] Successfully installed temurin@21.0.1
[kopi] Activating JDK 21.0.1 in new bash shell...
bash-5.1$ 
```

### Error Scenarios

#### Shell Detection Failed (Windows)
```bash
$ kopi shell 17
Error: Cannot detect shell on Windows. Please specify shell type with --shell option
Hint: Use one of the following:
  kopi shell 17 --shell powershell
  kopi shell 17 --shell cmd
```

#### Parent Process Detection Failed (Windows)
```bash
$ kopi shell 17
Error: Cannot determine parent shell executable path. Please specify shell type with --shell option
Hint: Use one of the following:
  kopi shell 17 --shell powershell
  kopi shell 17 --shell cmd
```

#### Executed from Non-Shell Process (Windows)
```bash
# From VSCode integrated terminal or other IDE
$ kopi shell 17
Error: Parent process 'Code.exe' is not a recognized shell. Please specify shell type with --shell option
Hint: Use one of the following:
  kopi shell 17 --shell powershell
  kopi shell 17 --shell cmd
  
# From Windows Terminal
$ kopi shell 17
Error: Parent process 'WindowsTerminal.exe' is not a recognized shell. Please specify shell type with --shell option
Hint: Use one of the following:
  kopi shell 17 --shell powershell
  kopi shell 17 --shell cmd
```

#### Version Not Installed (Auto-Install Disabled)
```bash
$ kopi shell 19
Error: JDK version '19' is not installed
Hint: Run 'kopi install 19' to install this version
      Or enable auto-installation in ~/.kopi/config.toml
```

#### Invalid Version Format
```bash
$ kopi shell invalid@version
Error: Invalid version format 'invalid@version'
Hint: Use format like '17', '17.0.5', or 'temurin@17'
```

#### Unknown Shell Type
```bash
$ kopi shell 17 --shell tcsh
Error: Unsupported shell type 'tcsh'
Supported shells: bash, zsh, fish, powershell, cmd
```

#### Shell Not Found
```bash
$ kopi shell 17 --shell zsh
Error: Shell 'zsh' not found in PATH
Hint: Make sure zsh is installed on your system
```

## Integration Features

### Nested Shell Sessions

The command supports nested sessions with different versions:

```bash
$ echo $KOPI_JAVA_VERSION
11.0.2
$ kopi shell 17
[kopi] Activating JDK 17.0.5 in new bash shell...
bash-5.1$ echo $KOPI_JAVA_VERSION
17.0.5
bash-5.1$ kopi shell 21
[kopi] Activating JDK 21.0.1 in new bash shell...
bash-5.1$ echo $KOPI_JAVA_VERSION
21.0.1
bash-5.1$ exit
bash-5.1$ echo $KOPI_JAVA_VERSION
17.0.5
```

### Shell Prompt Integration

Users can add version info to their prompt:

```bash
# ~/.bashrc
if [ -n "$KOPI_JAVA_VERSION" ]; then
    PS1="[java:$KOPI_JAVA_VERSION] $PS1"
fi
```

## Testing Strategy

### Unit Tests
- Version parsing and validation
- Parent process detection logic
- Shell type parsing from command line
- Auto-installation decision logic

### Integration Tests
- Launching shells with correct environment
- Parent shell detection across platforms
- Auto-installation workflow
- Nested shell sessions
- Error handling for missing shells

### Manual Testing
- Test on each supported shell type
- Verify parent process detection accuracy
- Test auto-installation prompts and flow
- Test manual shell specification
- Cross-platform behavior (Unix/Windows)

## Security Considerations

- No execution of arbitrary code
- Version strings are validated before use
- Shell binary paths are resolved securely
- Environment variable names are hardcoded
- No shell command injection vulnerabilities
- Auto-installation uses trusted kopi binary only

## Auto-Installation Integration

### Configuration

Auto-installation behavior is controlled by `~/.kopi/config.toml`:

```toml
[auto_install]
enabled = true        # Enable auto-installation
prompt = true         # Prompt before installing
timeout_secs = 300    # Installation timeout
```

### Enhanced detect_shell() Function

The existing `detect_shell()` function in `platform/shell.rs` will be improved to:

```rust
// platform/shell.rs - Enhanced version
pub fn detect_shell() -> Result<(Shell, PathBuf), ShellDetectionError> {
    // Try parent process detection first (most reliable)
    if let Ok((shell, path)) = detect_from_parent_process() {
        return Ok((shell, path));
    }
    
    // Unix: Fall back to SHELL environment variable
    #[cfg(unix)]
    if let Ok(shell_path) = env::var("SHELL") {
        let path = PathBuf::from(&shell_path);
        if path.exists() {
            if let Some(shell_type) = identify_shell_from_path(&path) {
                return Ok((shell_type, path));
            }
        }
    }
    
    // Windows: No reliable fallback, return error
    #[cfg(windows)]
    {
        return Err(ShellDetectionError::NoShellDetected(
            "Cannot detect shell on Windows. Parent process detection failed.".into()
        ));
    }
    
    // Unix: Final fallback
    #[cfg(unix)]
    {
        // Try to find bash in PATH as last resort
        if let Ok(bash_path) = which::which("bash") {
            return Ok((Shell::Bash, bash_path));
        }
    }
    
    Err(ShellDetectionError::NoShellDetected(
        "No shell could be detected".into()
    ))
}

fn detect_from_parent_process() -> Result<(Shell, PathBuf), ShellDetectionError> {
    // Implementation using sysinfo crate
    // Returns shell type and exact executable path
}

fn identify_shell_from_path(path: &Path) -> Option<Shell> {
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_string_lossy();
        match name.as_ref() {
            "bash" | "bash.exe" => Some(Shell::Bash),
            "zsh" | "zsh.exe" => Some(Shell::Zsh),
            "fish" | "fish.exe" => Some(Shell::Fish),
            "powershell" | "powershell.exe" => Some(Shell::PowerShell),
            "pwsh" | "pwsh.exe" => Some(Shell::PowerShell),
            "cmd" | "cmd.exe" => Some(Shell::Cmd),
            _ => None,
        }
    } else {
        None
    }
}
```

### Implementation Details

```rust
fn handle_missing_version(version_request: &VersionRequest, config: &KopiConfig) -> Result<()> {
    let auto_installer = AutoInstaller::new(config);
    
    if !auto_installer.should_auto_install() {
        return Err(KopiError::VersionNotInstalled(
            version_request.to_string()
        ));
    }
    
    // Show what will be installed
    eprintln!("JDK {} is not installed.", version_request);
    
    if auto_installer.prompt_user(&version_request.to_string())? {
        // Show progress during installation
        eprintln!("[kopi] Installing JDK {}...", version_request);
        auto_installer.install_jdk(version_request)?;
        eprintln!("[kopi] Successfully installed {}", version_request);
        Ok(())
    } else {
        Err(KopiError::InstallationCancelled)
    }
}
```

## Implementation Status

### Existing Code

1. **Platform Module** (`src/platform/`):
   - `process::exec_replace()`: Already implemented for process execution
   - `shell::Shell` enum: Already defined with all shell types
   - `shell::detect_shell()`: Exists but uses unreliable environment variables
   - Constants and utilities: Already implemented

2. **Error Types**:
   - Need to add `ShellDetectionError` to `KopiError` enum
   - Need to add `ShellNotFound` error variant

### To Be Implemented

1. **Enhanced Shell Detection** in `platform/shell.rs`:
   - Improve existing `detect_shell()` function:
     - Change return type from `Shell` to `Result<(Shell, PathBuf), ShellDetectionError>`
     - Add parent process detection using sysinfo crate as primary method
     - Remove unreliable PSModulePath check on Windows
     - Add strict error handling for Windows when detection fails
     - Keep SHELL environment variable as fallback on Unix only
   - Add helper functions:
     - `detect_from_parent_process()`: Parent process inspection
     - `identify_shell_from_path()`: Shell type identification from executable path
   - Add `ShellDetectionError` enum for proper error handling

2. **Shell Command Module** (`src/commands/shell.rs`):
   - `ShellCommand` struct
   - `execute()` method with auto-installation support
   - Integration with `VersionResolver` and `JdkRepository`

3. **CLI Integration** (`src/main.rs`):
   - Add `Shell` command variant with options:
     - `version: String`
     - `shell: Option<String>` for `--shell` flag

## Platform-Specific Behavior

### Unix/Linux/macOS
- Uses `exec()` to replace current process with shell
- Parent process detection via `/proc` or system APIs
- Login shells load user configuration files

### Windows
- Launches shell as subprocess (no exec equivalent)
- Parent process detection via Windows API
- PowerShell and CMD have different initialization flags

**Important Note on Windows Shell Detection**:
- `PSModulePath` environment variable exists system-wide when PowerShell is installed
- It is present even when running from cmd.exe, so it cannot be used to detect the current shell
- Parent process detection is the only reliable method on Windows
- When parent process detection fails, the command will error and require explicit `--shell` option
- This prevents launching the wrong shell type with incorrect syntax

**Why Use Detected Path Directly**:
Example scenario:
- User has `/opt/homebrew/bin/bash` (Homebrew bash) as their current shell
- System also has `/usr/bin/bash` (macOS system bash)
- If we just run `bash`, PATH resolution might pick `/usr/bin/bash`
- By using the detected path, we ensure the same shell binary is used

This is particularly important when:
- Multiple versions of the same shell are installed
- Custom shells are used from non-standard locations
- Shell behavior differs between versions

## Usage Considerations

### When Called from Non-Shell Processes

The `kopi shell` command is designed to be run from within a shell. When executed from:

1. **IDE Integrated Terminals**: Most IDEs (VSCode, IntelliJ) run a shell within their terminal, so detection usually works. However, some IDEs may show up as the parent process on Windows if they manage the shell process differently
2. **Terminal Emulators**: The emulator itself is not a shell, but it runs a shell process that kopi can detect
3. **Task Runners**: When using npm scripts or Makefiles, specify the shell explicitly:
   ```json
   // package.json
   "scripts": {
     "dev:shell": "kopi shell 17 --shell bash",
     "dev:windows": "kopi shell 17 --shell cmd"
   }
   ```
   ```makefile
   # Makefile
   dev:
   	kopi shell 17 --shell bash
   ```
4. **CI/CD**: Always use explicit `--shell` option in automated environments

### Best Practices

- Use `--shell` option when scripting or automating
- For frequently used non-shell environments, consider creating aliases or wrapper scripts
- On Windows, always prefer explicit shell specification for reliability
- Use debug logging (`-vv`) to see what parent process was detected:
  ```bash
  $ kopi shell 17 -vv
  [DEBUG] Parent process executable: /usr/local/bin/vscode
  [DEBUG] Parent process is not a recognized shell: vscode
  ```

## Future Enhancements

1. **Shell Customization**: Pass additional shell arguments
2. **Version Validation**: Pre-flight check for Java executable
3. **Shell History**: Maintain separate history for kopi shells
4. **Quick Switch**: `kopi shell -` to switch to previous version
5. **List Mode**: `kopi shell --list` to show available versions