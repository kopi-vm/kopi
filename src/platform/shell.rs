use crate::platform;
use std::env;
use std::path::{Path, PathBuf};

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

/// Detect the current shell
pub fn detect_shell() -> Shell {
    // First try SHELL environment variable (Unix)
    #[cfg(unix)]
    if let Ok(shell_path) = env::var("SHELL") {
        if let Some(shell_name) = PathBuf::from(&shell_path).file_name() {
            let shell_name = shell_name.to_string_lossy();
            match shell_name.as_ref() {
                "bash" => return Shell::Bash,
                "zsh" => return Shell::Zsh,
                "fish" => return Shell::Fish,
                _ => {}
            }
        }
    }

    // On Windows, check COMSPEC or PSModulePath
    #[cfg(windows)]
    {
        if env::var("PSModulePath").is_ok() {
            return Shell::PowerShell;
        }
        if let Ok(comspec) = env::var("COMSPEC") {
            if comspec.to_lowercase().contains("cmd.exe") {
                return Shell::Cmd;
            }
        }
    }

    // Try parent process name
    if let Ok(parent) = env::var("SHELL") {
        return Shell::Unknown(parent);
    }

    // Default fallback
    #[cfg(unix)]
    return Shell::Bash;

    #[cfg(windows)]
    return Shell::PowerShell;
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

    /// Generate PATH update instructions for this shell
    pub fn generate_path_instructions(&self, shims_dir: &Path) -> String {
        let shims_path = shims_dir.to_string_lossy();

        match self {
            Shell::Bash | Shell::Zsh => {
                format!(
                    r#"# Add Kopi shims to PATH
export PATH="{}:$PATH"

# To make this permanent, add the above line to your {}
"#,
                    shims_path,
                    self.get_config_file()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| format!("~/.{}", self.get_shell_name()))
                )
            }
            Shell::Fish => {
                format!(
                    r#"# Add Kopi shims to PATH
set -gx PATH {shims_path} $PATH

# To make this permanent, add the above line to ~/.config/fish/config.fish
"#
                )
            }
            Shell::PowerShell => {
                format!(
                    r#"# Add Kopi shims to PATH
$env:Path = "{shims_path};$env:Path"

# To make this permanent, add the following to your PowerShell profile:
# $env:Path = "{shims_path};$env:Path"
#
# You can edit your profile by running:
# notepad $PROFILE
"#
                )
            }
            Shell::Cmd => {
                format!(
                    r#"REM Add Kopi shims to PATH temporarily
set PATH={shims_path};%PATH%

REM To make this permanent, use the System Properties dialog:
REM 1. Right-click "This PC" or "My Computer"
REM 2. Click "Properties" -> "Advanced system settings"
REM 3. Click "Environment Variables"
REM 4. Edit the PATH variable and add: {shims_path}
"#
                )
            }
            Shell::Unknown(_) => {
                format!(
                    r#"# Add Kopi shims to PATH
# Add this line to your shell configuration file:
export PATH="{shims_path}:$PATH"
"#
                )
            }
        }
    }
}

/// Check if a directory is in PATH
pub fn is_in_path(dir: &Path) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        let separator = platform::path_separator();
        let paths = path_var.split(separator);

        for path in paths {
            if Path::new(path) == dir {
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
        let shell = detect_shell();
        // The shell might be Unknown in some test environments, so we just verify we got a result
        assert!(matches!(
            shell,
            Shell::Bash | Shell::Zsh | Shell::Fish | Shell::PowerShell | Shell::Unknown(_)
        ));
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
    fn test_path_instructions() {
        let shims_dir = Path::new("/home/user/.kopi/shims");

        let bash_instructions = Shell::Bash.generate_path_instructions(shims_dir);
        assert!(bash_instructions.contains("export PATH="));
        assert!(bash_instructions.contains("/home/user/.kopi/shims"));

        let fish_instructions = Shell::Fish.generate_path_instructions(shims_dir);
        assert!(fish_instructions.contains("set -gx PATH"));

        let ps_instructions = Shell::PowerShell.generate_path_instructions(shims_dir);
        assert!(ps_instructions.contains("$env:Path"));
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
}
