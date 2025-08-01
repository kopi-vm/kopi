use crate::error::{KopiError, Result};
use crate::platform;
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

    // Find parent process
    if let Some(current_process) = system.process(Pid::from_u32(current_pid)) {
        if let Some(parent_pid) = current_process.parent() {
            if let Some(parent_process) = system.process(parent_pid) {
                // Get the executable path
                if let Some(exe_path) = parent_process.exe() {
                    log::debug!("Parent process executable: {exe_path:?}");

                    // Get just the file name from the path
                    if let Some(file_name) = exe_path.file_name() {
                        let file_str = file_name.to_string_lossy();
                        log::debug!("Parent process file name: {file_str}");

                        // Check executable file name and return both type and path
                        match file_str.as_ref() {
                            "bash" | "bash.exe" => {
                                return Ok((Shell::Bash, exe_path.to_path_buf()));
                            }
                            "zsh" | "zsh.exe" => return Ok((Shell::Zsh, exe_path.to_path_buf())),
                            "fish" | "fish.exe" => {
                                return Ok((Shell::Fish, exe_path.to_path_buf()));
                            }
                            "powershell" | "powershell.exe" => {
                                return Ok((Shell::PowerShell, exe_path.to_path_buf()));
                            }
                            "pwsh" | "pwsh.exe" => {
                                return Ok((Shell::PowerShell, exe_path.to_path_buf()));
                            }
                            "cmd" | "cmd.exe" => return Ok((Shell::Cmd, exe_path.to_path_buf())),
                            _ => {
                                log::debug!("Parent process is not a recognized shell: {file_str}");
                                #[cfg(windows)]
                                {
                                    return Err(KopiError::ShellDetectionError(format!(
                                        "Parent process '{file_str}' is not a recognized shell. \
                                         Please specify shell type with --shell option"
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
                        "Cannot determine parent shell executable path. Please specify shell type \
                         with --shell option"
                            .to_string(),
                    ));
                }
            }
        }
    }

    // On Windows, we cannot proceed without parent process detection
    #[cfg(windows)]
    {
        Err(KopiError::ShellDetectionError(
            "Cannot detect parent shell on Windows. Please specify shell type with --shell option"
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

#[cfg(test)]
mod shell_tests {
    use super::*;

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
}

/// Check if a directory is in PATH
pub fn is_in_path(dir: &Path) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        let separator = platform::path_separator();
        let paths = path_var.split(separator);

        // Try to canonicalize the target directory for comparison
        let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());

        for path in paths {
            let path_buf = PathBuf::from(path);

            // Try to canonicalize the PATH entry
            let canonical_path = path_buf.canonicalize().unwrap_or_else(|_| path_buf.clone());

            // Compare both original and canonical paths
            if path_buf == dir
                || canonical_path == canonical_dir
                || path_buf == canonical_dir
                || canonical_path == dir
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_is_in_path() {
        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Set a test PATH with platform-specific paths and separators
        let separator = platform::path_separator();
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
