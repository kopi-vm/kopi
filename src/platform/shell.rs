// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::{KopiError, Result};
use std::env;
use std::path::{Path, PathBuf};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Detected shell type
#[derive(Debug, Clone, PartialEq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
    Unknown(String),
}

/// Detect the parent shell with its executable path
pub fn detect_shell() -> Result<(Shell, PathBuf)> {
    // Get current process ID
    let current_pid = std::process::id();

    // Get system information
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All);

    // Traverse up the process tree to find a shell
    let mut current_pid = Pid::from_u32(current_pid);
    let mut depth = 0;
    const MAX_DEPTH: usize = 10; // Prevent infinite loops

    loop {
        if depth >= MAX_DEPTH {
            log::debug!("Reached maximum depth ({MAX_DEPTH}) while searching for shell");
            break;
        }

        let Some(process) = system.process(current_pid) else {
            log::debug!("Process with PID {current_pid:?} not found");
            break;
        };

        // Check if this process is a shell
        if let Some(exe_path) = process.exe() {
            if let Some(file_name) = exe_path.file_name() {
                let file_str = file_name.to_string_lossy();
                log::debug!("Checking process at depth {depth}: {file_str} (PID: {current_pid:?})");

                // Check executable file name and return both type and path
                match file_str.as_ref() {
                    "bash" | "bash.exe" => {
                        log::debug!("Found bash shell at depth {depth}");
                        return Ok((Shell::Bash, exe_path.to_path_buf()));
                    }
                    "zsh" | "zsh.exe" => {
                        log::debug!("Found zsh shell at depth {depth}");
                        return Ok((Shell::Zsh, exe_path.to_path_buf()));
                    }
                    "fish" | "fish.exe" => {
                        log::debug!("Found fish shell at depth {depth}");
                        return Ok((Shell::Fish, exe_path.to_path_buf()));
                    }
                    "powershell" | "powershell.exe" => {
                        log::debug!("Found PowerShell at depth {depth}");
                        return Ok((Shell::PowerShell, exe_path.to_path_buf()));
                    }
                    "pwsh" | "pwsh.exe" => {
                        log::debug!("Found PowerShell Core at depth {depth}");
                        return Ok((Shell::PowerShell, exe_path.to_path_buf()));
                    }
                    "cmd" | "cmd.exe" => {
                        log::debug!("Found cmd shell at depth {depth}");
                        return Ok((Shell::Cmd, exe_path.to_path_buf()));
                    }
                    _ => {
                        // Not a recognized shell, continue searching
                    }
                }
            }
        }

        // Move to parent process
        let Some(parent_pid) = process.parent() else {
            log::debug!("No parent process found for PID {current_pid:?}");
            break;
        };

        // Check for process loops (shouldn't happen but be safe)
        if parent_pid == current_pid {
            log::debug!("Detected process loop: process is its own parent");
            break;
        }

        current_pid = parent_pid;
        depth += 1;
    }

    // On Windows, we couldn't find a shell in the process tree
    #[cfg(windows)]
    {
        Err(KopiError::ShellDetectionError(
            "Cannot detect shell in process tree. Please specify shell type with --shell option"
                .to_string(),
        ))
    }

    // Unix: Fallback to environment detection
    #[cfg(not(windows))]
    {
        let shell_type = detect_shell_from_env()?;
        let shell_path = find_shell_in_path(&shell_type)?;
        Ok((shell_type, shell_path))
    }
}

/// Detect shell from environment variables (Unix fallback)
#[cfg(not(windows))]
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

    // Default fallback
    Ok(Shell::Bash)
}

/// Find shell executable in PATH
pub fn find_shell_in_path(shell: &Shell) -> Result<PathBuf> {
    let shell_name = match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => {
            if cfg!(windows) {
                "powershell"
            } else {
                "pwsh"
            }
        }
        Shell::Cmd => "cmd",
        Shell::Unknown(name) => name,
    };

    which::which(shell_name).map_err(|_| KopiError::ShellNotFound(shell_name.to_string()))
}

/// Parse shell name from string
pub fn parse_shell_name(name: &str) -> Result<Shell> {
    match name.to_lowercase().as_str() {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" | "pwsh" => Ok(Shell::PowerShell),
        "cmd" => Ok(Shell::Cmd),
        _ => Err(KopiError::UnsupportedShell(name.to_string())),
    }
}

impl Shell {
    /// Get the configuration file for this shell
    pub fn get_config_file(&self) -> Option<PathBuf> {
        match self {
            Shell::Bash => {
                // Try .bashrc first, then .bash_profile
                if let Ok(home) = env::var("HOME") {
                    let home = PathBuf::from(home);
                    let bashrc = home.join(".bashrc");
                    if bashrc.exists() {
                        return Some(bashrc);
                    }
                    let bash_profile = home.join(".bash_profile");
                    if bash_profile.exists() {
                        return Some(bash_profile);
                    }
                    // Default to .bashrc even if it doesn't exist
                    return Some(bashrc);
                }
                None
            }
            Shell::Zsh => {
                if let Ok(home) = env::var("HOME") {
                    Some(PathBuf::from(home).join(".zshrc"))
                } else {
                    None
                }
            }
            Shell::Fish => {
                if let Ok(home) = env::var("HOME") {
                    Some(PathBuf::from(home).join(".config/fish/config.fish"))
                } else {
                    None
                }
            }
            Shell::PowerShell => {
                // PowerShell profile location varies by version and platform
                if let Ok(profile) = env::var("PROFILE") {
                    Some(PathBuf::from(profile))
                } else {
                    None
                }
            }
            Shell::Cmd => None, // CMD doesn't have a standard config file
            Shell::Unknown(_) => None,
        }
    }

    /// Get the shell name for display
    pub fn get_shell_name(&self) -> &str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "PowerShell",
            Shell::Cmd => "cmd",
            Shell::Unknown(name) => name,
        }
    }

    /// Get the PATH configuration command for this shell
    pub fn get_path_config_command(&self) -> String {
        match self {
            Shell::Bash | Shell::Zsh => "export PATH=\"$HOME/.kopi/shims:$PATH\"".to_string(),
            Shell::Fish => "set -gx PATH $HOME/.kopi/shims $PATH".to_string(),
            Shell::PowerShell => {
                "$env:Path = \"$env:USERPROFILE\\.kopi\\shims;$env:Path\"".to_string()
            }
            Shell::Cmd => "set PATH=%USERPROFILE%\\.kopi\\shims;%PATH%".to_string(),
            Shell::Unknown(_) => {
                // Default to bash/zsh style
                "export PATH=\"$HOME/.kopi/shims:$PATH\"".to_string()
            }
        }
    }
}

/// Check if a directory is in PATH
pub fn is_in_path(dir: &Path) -> bool {
    let Ok(paths) = env::var("PATH") else {
        return false;
    };

    let canonical_dir = if dir.exists() {
        dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())
    } else {
        dir.to_path_buf()
    };

    env::split_paths(&paths).any(|path| {
        // Direct comparison first
        if path == dir {
            return true;
        }

        // Try canonical comparison
        let canonical_path = if path.exists() {
            path.canonicalize().unwrap_or_else(|_| path.clone())
        } else {
            path.clone()
        };

        canonical_path == canonical_dir || canonical_path == dir || path == canonical_dir
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_path_config_commands() {
        // Test bash command
        let bash = Shell::Bash;
        assert_eq!(
            bash.get_path_config_command(),
            "export PATH=\"$HOME/.kopi/shims:$PATH\""
        );

        // Test zsh command (same as bash)
        let zsh = Shell::Zsh;
        assert_eq!(
            zsh.get_path_config_command(),
            "export PATH=\"$HOME/.kopi/shims:$PATH\""
        );

        // Test fish command
        let fish = Shell::Fish;
        assert_eq!(
            fish.get_path_config_command(),
            "set -gx PATH $HOME/.kopi/shims $PATH"
        );

        // Test PowerShell command
        let powershell = Shell::PowerShell;
        assert_eq!(
            powershell.get_path_config_command(),
            "$env:Path = \"$env:USERPROFILE\\.kopi\\shims;$env:Path\""
        );

        // Test cmd command
        let cmd = Shell::Cmd;
        assert_eq!(
            cmd.get_path_config_command(),
            "set PATH=%USERPROFILE%\\.kopi\\shims;%PATH%"
        );

        // Test unknown shell (defaults to bash style)
        let unknown = Shell::Unknown("mycustomshell".to_string());
        assert_eq!(
            unknown.get_path_config_command(),
            "export PATH=\"$HOME/.kopi/shims:$PATH\""
        );
    }

    #[test]
    #[serial]
    fn test_is_in_path_basic() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_kopi");
        fs::create_dir_all(&test_dir).unwrap();

        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Test with directory in PATH
        let separator = if cfg!(windows) { ";" } else { ":" };
        let new_path = format!("{}{}{}", test_dir.display(), separator, original_path);
        unsafe {
            env::set_var("PATH", &new_path);
        }

        assert!(is_in_path(&test_dir), "Directory should be found in PATH");

        // Test with directory not in PATH
        unsafe {
            env::set_var("PATH", &original_path);
        }
        assert!(
            !is_in_path(&test_dir),
            "Directory should not be found in PATH"
        );

        // Restore original PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_shell_detection() {
        // This test is environment-dependent, so we just verify it returns something
        let result = detect_shell();

        // On Windows without a shell parent, it should error
        #[cfg(windows)]
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(err.to_string().contains("shell"));
        }

        // On Unix, it should either succeed or fallback
        #[cfg(not(windows))]
        if let Ok((shell, path)) = result {
            assert!(path.exists());
            // Verify we got a known shell type
            assert!(matches!(
                shell,
                Shell::Bash | Shell::Zsh | Shell::Fish | Shell::PowerShell | Shell::Cmd
            ));
        }
    }

    #[test]
    #[serial]
    fn test_shell_config_files() {
        // Test with a mock HOME environment
        let original_home = env::var("HOME").ok();
        unsafe {
            env::set_var("HOME", "/home/testuser");
        }

        let bash = Shell::Bash;
        let config = bash.get_config_file();
        assert!(config.is_some());
        if let Some(config_path) = config {
            let path_str = config_path.to_string_lossy();
            assert!(path_str.contains(".bashrc") || path_str.contains(".bash_profile"));
        }

        let zsh = Shell::Zsh;
        let config = zsh.get_config_file();
        assert_eq!(config, Some(PathBuf::from("/home/testuser/.zshrc")));

        let fish = Shell::Fish;
        let config = fish.get_config_file();
        assert_eq!(
            config,
            Some(PathBuf::from("/home/testuser/.config/fish/config.fish"))
        );

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                env::set_var("HOME", home);
            } else {
                env::remove_var("HOME");
            }
        }
    }

    #[test]
    #[serial]
    fn test_is_in_path() {
        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Set a test PATH with platform-specific paths and separators
        let separator = if cfg!(windows) { ';' } else { ':' };
        let test_paths: Vec<&str>;
        let test_dir: &Path;
        let not_in_path_dir: &Path;

        #[cfg(windows)]
        {
            test_paths = vec![
                "C:\\Windows\\System32",
                "C:\\Program Files",
                "C:\\Users\\test\\.kopi\\shims",
            ];
            test_dir = Path::new("C:\\Windows\\System32");
            not_in_path_dir = Path::new("C:\\opt\\bin");
        }

        #[cfg(not(windows))]
        {
            test_paths = vec!["/usr/bin", "/usr/local/bin", "/home/user/.kopi/shims"];
            test_dir = Path::new("/usr/bin");
            not_in_path_dir = Path::new("/opt/bin");
        }

        let test_path_string = test_paths.join(&separator.to_string());
        unsafe {
            env::set_var("PATH", &test_path_string);
        }

        assert!(is_in_path(test_dir));
        assert!(is_in_path(Path::new(test_paths[2])));
        assert!(!is_in_path(not_in_path_dir));

        // Restore original PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_parse_shell_name() {
        assert_eq!(parse_shell_name("bash").unwrap(), Shell::Bash);
        assert_eq!(parse_shell_name("zsh").unwrap(), Shell::Zsh);
        assert_eq!(parse_shell_name("fish").unwrap(), Shell::Fish);
        assert_eq!(parse_shell_name("powershell").unwrap(), Shell::PowerShell);
        assert_eq!(parse_shell_name("pwsh").unwrap(), Shell::PowerShell);
        assert_eq!(parse_shell_name("cmd").unwrap(), Shell::Cmd);
        assert_eq!(parse_shell_name("BASH").unwrap(), Shell::Bash); // case insensitive

        assert!(parse_shell_name("tcsh").is_err());
        assert!(parse_shell_name("unknown").is_err());
    }

    #[test]
    fn test_find_shell_in_path() {
        // This test depends on the system having shells installed
        // We'll just test that the function doesn't panic

        // Test with a shell that's likely to exist
        #[cfg(unix)]
        {
            let result = find_shell_in_path(&Shell::Bash);
            // On most Unix systems, bash should be available
            // But in test environments it might not be, so we don't assert success
            if result.is_ok() {
                assert!(result.unwrap().exists());
            }
        }

        #[cfg(windows)]
        {
            let result = find_shell_in_path(&Shell::Cmd);
            // CMD should always be available on Windows
            if result.is_ok() {
                assert!(result.unwrap().exists());
            }
        }

        // Test with a shell that definitely doesn't exist
        let unknown_shell = Shell::Unknown("definitely_not_a_real_shell".to_string());
        assert!(find_shell_in_path(&unknown_shell).is_err());
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_detect_shell_from_env() {
        // Save original SHELL
        let original_shell = env::var("SHELL").ok();

        // Test bash detection
        unsafe {
            env::set_var("SHELL", "/bin/bash");
        }
        assert_eq!(detect_shell_from_env().unwrap(), Shell::Bash);

        // Test zsh detection
        unsafe {
            env::set_var("SHELL", "/usr/local/bin/zsh");
        }
        assert_eq!(detect_shell_from_env().unwrap(), Shell::Zsh);

        // Test fish detection
        unsafe {
            env::set_var("SHELL", "/usr/bin/fish");
        }
        assert_eq!(detect_shell_from_env().unwrap(), Shell::Fish);

        // Test unknown shell (should default to bash)
        unsafe {
            env::set_var("SHELL", "/usr/bin/tcsh");
        }
        assert_eq!(detect_shell_from_env().unwrap(), Shell::Bash);

        // Restore original SHELL
        unsafe {
            if let Some(shell) = original_shell {
                env::set_var("SHELL", shell);
            } else {
                env::remove_var("SHELL");
            }
        }
    }
}
