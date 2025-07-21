use crate::config::KopiConfig;
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use crate::platform::shell::{detect_shell, is_in_path};
use crate::platform::{executable_extension, kopi_binary_name, path_separator};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use which::which;

/// Check if kopi binary is installed and in PATH
pub struct KopiBinaryCheck;

impl DiagnosticCheck for KopiBinaryCheck {
    fn name(&self) -> &str {
        "Kopi Binary in PATH"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let binary_name = kopi_binary_name();

        match which(binary_name) {
            Ok(path) => {
                // Check if the binary is executable
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let mode = metadata.permissions().mode();
                            if mode & 0o111 == 0 {
                                return CheckResult::new(
                                    self.name(),
                                    category,
                                    CheckStatus::Fail,
                                    "Kopi binary found but not executable",
                                    start.elapsed(),
                                )
                                .with_suggestion(format!("Run: chmod +x {}", path.display()));
                            }
                        }

                        CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Pass,
                            format!("Kopi binary found at {}", path.display()),
                            start.elapsed(),
                        )
                    }
                    Err(e) => CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        format!("Kopi binary found but cannot check permissions: {e}"),
                        start.elapsed(),
                    ),
                }
            }
            Err(_) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "Kopi binary not found in PATH",
                start.elapsed(),
            )
            .with_suggestion("Add kopi installation directory to your PATH environment variable"),
        }
    }
}

/// Check kopi version (compare with latest if network available)
pub struct VersionCheck;

impl DiagnosticCheck for VersionCheck {
    fn name(&self) -> &str {
        "Kopi Version"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let binary_name = kopi_binary_name();

        // First check if kopi is available
        let kopi_path = match which(binary_name) {
            Ok(path) => path,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check version - kopi not found in PATH",
                    start.elapsed(),
                );
            }
        };

        // Get current version
        match Command::new(&kopi_path).arg("--version").output() {
            Ok(output) => {
                if output.status.success() {
                    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

                    // TODO: In the future, we could check against the latest version from GitHub
                    // For now, just report the current version
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        format!("Kopi version: {version_str}"),
                        start.elapsed(),
                    )
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        "Could not determine kopi version",
                        start.elapsed(),
                    )
                    .with_details(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!("Failed to execute kopi --version: {e}"),
                start.elapsed(),
            ),
        }
    }
}

/// Check if installation directory exists and has proper structure
pub struct InstallationDirectoryCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> InstallationDirectoryCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl DiagnosticCheck for InstallationDirectoryCheck<'_> {
    fn name(&self) -> &str {
        "Installation Directory Structure"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let kopi_home = self.config.kopi_home();

        // Check if kopi home exists
        if !kopi_home.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Kopi home directory not found: {}", kopi_home.display()),
                start.elapsed(),
            )
            .with_suggestion("Run kopi installer or create the directory manually");
        }

        // Check subdirectories
        let mut missing_dirs = Vec::new();
        let subdirs = [
            ("jdks", self.config.jdks_dir()),
            ("shims", self.config.shims_dir()),
            ("cache", self.config.cache_dir()),
        ];

        for (name, dir_result) in subdirs {
            match dir_result {
                Ok(dir) => {
                    if !dir.exists() {
                        missing_dirs.push(name);
                    }
                }
                Err(e) => {
                    return CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Fail,
                        format!("Cannot determine {name} directory"),
                        start.elapsed(),
                    )
                    .with_details(e.to_string());
                }
            }
        }

        if missing_dirs.is_empty() {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                format!(
                    "Installation directory structure is valid: {}",
                    kopi_home.display()
                ),
                start.elapsed(),
            )
        } else {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!("Missing subdirectories: {}", missing_dirs.join(", ")),
                start.elapsed(),
            )
            .with_suggestion("These directories will be created automatically when needed")
        }
    }
}

/// Check if config file is valid
pub struct ConfigFileCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> ConfigFileCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl DiagnosticCheck for ConfigFileCheck<'_> {
    fn name(&self) -> &str {
        "Configuration File"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let config_path = self.config.config_path();

        if !config_path.exists() {
            // Config file is optional
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "No config file found (using defaults)",
                start.elapsed(),
            )
            .with_details(format!("Expected location: {}", config_path.display()));
        }

        // Try to parse the config file
        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(_) => CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Pass,
                    format!("Config file is valid: {}", config_path.display()),
                    start.elapsed(),
                ),
                Err(e) => CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Fail,
                    "Config file has invalid TOML syntax",
                    start.elapsed(),
                )
                .with_details(e.to_string())
                .with_suggestion(format!(
                    "Fix the syntax errors in {}",
                    config_path.display()
                )),
            },
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                "Cannot read config file",
                start.elapsed(),
            )
            .with_details(e.to_string()),
        }
    }
}

/// Check if shims directory is in PATH
pub struct ShimsInPathCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> ShimsInPathCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl DiagnosticCheck for ShimsInPathCheck<'_> {
    fn name(&self) -> &str {
        "Shims Directory in PATH"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let shims_dir = match self.config.shims_dir() {
            Ok(dir) => dir,
            Err(e) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Fail,
                    "Cannot determine shims directory",
                    start.elapsed(),
                )
                .with_details(e.to_string());
            }
        };

        if is_in_path(&shims_dir) {
            // Check PATH priority - shims should come before system Java
            let path_var = std::env::var("PATH").unwrap_or_default();
            let paths: Vec<&str> = path_var.split(path_separator()).collect();

            let shims_index = paths.iter().position(|p| Path::new(p) == shims_dir);
            let java_indices: Vec<usize> = paths
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let java_path = Path::new(p)
                        .join("java")
                        .with_extension(executable_extension());
                    if java_path.exists() && shims_index != Some(i) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            if let Some(shims_idx) = shims_index {
                if let Some(&first_java_idx) = java_indices.first() {
                    if shims_idx > first_java_idx {
                        return CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Warning,
                            "Shims directory is in PATH but appears after system Java",
                            start.elapsed(),
                        )
                        .with_details(format!(
                            "Shims at position {}, system Java at position {}",
                            shims_idx + 1,
                            first_java_idx + 1
                        ))
                        .with_suggestion(
                            "Move shims directory earlier in PATH to take precedence",
                        );
                    }
                }
            }

            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "Shims directory is in PATH with correct priority",
                start.elapsed(),
            )
        } else {
            let shell_result = detect_shell();
            let suggestion = match shell_result {
                Ok((shell, _)) => match shell {
                    crate::platform::shell::Shell::Bash => {
                        format!(
                            "Add to ~/.bashrc:\nexport PATH=\"{}:$PATH\"",
                            shims_dir.display()
                        )
                    }
                    crate::platform::shell::Shell::Zsh => {
                        format!(
                            "Add to ~/.zshrc:\nexport PATH=\"{}:$PATH\"",
                            shims_dir.display()
                        )
                    }
                    crate::platform::shell::Shell::Fish => {
                        format!(
                            "Add to ~/.config/fish/config.fish:\nset -gx PATH {} $PATH",
                            shims_dir.display()
                        )
                    }
                    crate::platform::shell::Shell::PowerShell => {
                        format!(
                            "Add to $PROFILE:\n$env:Path = \"{};$env:Path\"",
                            shims_dir.display()
                        )
                    }
                    _ => format!(
                        "Add {} to your PATH environment variable",
                        shims_dir.display()
                    ),
                },
                Err(_) => format!(
                    "Add {} to your PATH environment variable",
                    shims_dir.display()
                ),
            };

            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "Shims directory not found in PATH",
                start.elapsed(),
            )
            .with_details(format!("Expected: {}", shims_dir.display()))
            .with_suggestion(suggestion)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_kopi_binary_check_not_in_path() {
        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Set PATH to exclude kopi
        unsafe {
            env::set_var("PATH", "/usr/bin:/bin");
        }

        let check = KopiBinaryCheck;
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);

        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("not found in PATH"));
        assert!(result.suggestion.is_some());

        // Restore original PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_version_check_skip_when_not_found() {
        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Set PATH to exclude kopi
        unsafe {
            env::set_var("PATH", "/usr/bin:/bin");
        }

        let check = VersionCheck;
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);

        assert_eq!(result.status, CheckStatus::Skip);
        assert!(result.message.contains("kopi not found"));

        // Restore original PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_installation_directory_check() {
        let temp_dir = TempDir::new().unwrap();

        // Override kopi_home to use temp directory
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        let config = crate::config::new_kopi_config().unwrap();

        let check = InstallationDirectoryCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);

        // Directory and subdirs exist because config creation creates them
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(
            result
                .message
                .contains("Installation directory structure is valid")
        );

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_config_file_check_missing_is_ok() {
        let temp_dir = TempDir::new().unwrap();

        // Override kopi_home to use temp directory
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        let config = crate::config::new_kopi_config().unwrap();

        let check = ConfigFileCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);

        // Missing config file is OK (uses defaults)
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains("using defaults"));

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_config_file_check_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();

        // First create a valid config
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        let config = crate::config::new_kopi_config().unwrap();
        let config_path = config.config_path();

        // Now write invalid TOML to the config file
        fs::write(&config_path, "invalid = toml content [").unwrap();

        let check = ConfigFileCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);

        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("invalid TOML syntax"));
        assert!(result.suggestion.is_some());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_shims_in_path_check() {
        let temp_dir = TempDir::new().unwrap();
        let shims_dir = temp_dir.path().join("shims");
        fs::create_dir(&shims_dir).unwrap();

        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        let config = crate::config::new_kopi_config().unwrap();

        // Save original PATH
        let original_path = env::var("PATH").unwrap_or_default();

        // Test when shims not in PATH
        unsafe {
            env::set_var("PATH", "/usr/bin:/bin");
        }
        let check = ShimsInPathCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);
        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.suggestion.is_some());

        // Test when shims in PATH
        unsafe {
            env::set_var("PATH", format!("{}:/usr/bin", shims_dir.display()));
        }
        let check = ShimsInPathCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Installation);
        assert_eq!(result.status, CheckStatus::Pass);

        // Restore
        unsafe {
            env::set_var("PATH", original_path);
        }
        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }
}
