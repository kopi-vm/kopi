use crate::config::KopiConfig;
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use crate::platform::shell::{detect_shell, is_in_path};
use crate::platform::{path_separator, with_executable_extension};
use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Check if ~/.kopi/shims is in PATH and has correct priority
pub struct PathCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> PathCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn check_path_priority(&self, shims_dir: &Path) -> Option<String> {
        if let Ok(path_var) = env::var("PATH") {
            let separator = path_separator();
            let paths: Vec<&str> = path_var.split(&separator.to_string()).collect();

            let mut shims_index = None;
            let mut system_java_index = None;

            for (index, path) in paths.iter().enumerate() {
                if Path::new(path) == shims_dir {
                    shims_index = Some(index);
                }

                // Check for system Java installations
                if (path.contains("/usr/bin")
                    || path.contains("\\System32")
                    || path.contains("Java")
                    || path.contains("java"))
                    && system_java_index.is_none()
                {
                    // Check if this directory contains java executable
                    let java_path = Path::new(path).join(with_executable_extension("java"));
                    if java_path.exists() {
                        system_java_index = Some(index);
                    }
                }
            }

            if let (Some(shims), Some(system)) = (shims_index, system_java_index) {
                if shims > system {
                    return Some(format!(
                        "Kopi shims directory is in PATH but comes after system Java (position {} \
                         vs {})",
                        shims + 1,
                        system + 1
                    ));
                }
            }
        }
        None
    }
}

impl<'a> DiagnosticCheck for PathCheck<'a> {
    fn name(&self) -> &str {
        "PATH Configuration"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let shims_dir = self.config.kopi_home().join("shims");

        if !is_in_path(&shims_dir) {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "~/.kopi/shims not found in PATH",
                start.elapsed(),
            )
            .with_details("Kopi shims directory must be in your PATH for automatic JDK switching")
            .with_suggestion({
                let shell_cmd = if let Ok((shell, _)) = detect_shell() {
                    shell.get_path_config_command()
                } else {
                    // Default to bash/zsh style if detection fails
                    format!("export PATH=\"{}:$PATH\"", shims_dir.display())
                };
                format!("Add this line to your shell configuration:\n{shell_cmd}")
            });
        }

        // Check PATH priority
        if let Some(priority_issue) = self.check_path_priority(&shims_dir) {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                priority_issue,
                start.elapsed(),
            )
            .with_details("Kopi shims should appear before system Java in PATH")
            .with_suggestion("Reorder your PATH to ensure ~/.kopi/shims comes first");
        }

        CheckResult::new(
            self.name(),
            category,
            CheckStatus::Pass,
            "PATH correctly configured with ~/.kopi/shims",
            start.elapsed(),
        )
    }
}

/// Check shell detection and configuration
pub struct ShellDetectionCheck;

impl DiagnosticCheck for ShellDetectionCheck {
    fn name(&self) -> &str {
        "Shell Detection"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        match detect_shell() {
            Ok((shell, path)) => {
                let details = format!(
                    "Detected {} shell at {}",
                    shell.get_shell_name(),
                    path.display()
                );

                CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Pass,
                    format!("Detected shell: {}", shell.get_shell_name()),
                    start.elapsed(),
                )
                .with_details(details)
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                "Unable to detect parent shell",
                start.elapsed(),
            )
            .with_details(format!("Error: {e}"))
            .with_suggestion("Specify your shell type when running 'kopi shell' command"),
        }
    }
}

/// Check shell configuration files for kopi setup
pub struct ShellConfigurationCheck;

impl DiagnosticCheck for ShellConfigurationCheck {
    fn name(&self) -> &str {
        "Shell Configuration"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        // Try to detect shell and check its config
        let (shell, config_file) = match detect_shell() {
            Ok((shell, _)) => {
                let config = shell.get_config_file();
                (shell, config)
            }
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check shell configuration - shell detection failed",
                    start.elapsed(),
                );
            }
        };

        let config_file = match config_file {
            Some(file) => file,
            None => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    format!(
                        "{} shell has no standard configuration file",
                        shell.get_shell_name()
                    ),
                    start.elapsed(),
                );
            }
        };

        // Check if config file exists
        if !config_file.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!(
                    "Shell configuration file not found: {}",
                    config_file.display()
                ),
                start.elapsed(),
            )
            .with_suggestion(format!(
                "Create {} and add:\n{}",
                config_file.display(),
                shell.get_path_config_command()
            ));
        }

        // Check if file contains kopi setup
        match fs::read_to_string(&config_file) {
            Ok(content) => {
                let has_kopi_path =
                    content.contains("/.kopi/shims") || content.contains("\\.kopi\\shims");
                let has_kopi_export = content.contains("export PATH") && has_kopi_path;

                if has_kopi_export || has_kopi_path {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        format!("Kopi setup found in {}", config_file.display()),
                        start.elapsed(),
                    )
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        format!("Kopi setup not found in {}", config_file.display()),
                        start.elapsed(),
                    )
                    .with_suggestion(format!(
                        "Add to {}:\n{}",
                        config_file.display(),
                        shell.get_path_config_command()
                    ))
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!("Cannot read shell configuration file: {e}"),
                start.elapsed(),
            ),
        }
    }
}

/// Check if shims directory exists and contains executables
pub struct ShimFunctionalityCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> ShimFunctionalityCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn check_shim_executable(&self, shim_path: &Path) -> bool {
        crate::platform::file_ops::is_executable(shim_path).unwrap_or(false)
    }
}

impl<'a> DiagnosticCheck for ShimFunctionalityCheck<'a> {
    fn name(&self) -> &str {
        "Shim Functionality"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let shims_dir = self.config.kopi_home().join("shims");

        // Check if shims directory exists
        if !shims_dir.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "Shims directory does not exist",
                start.elapsed(),
            )
            .with_details(format!("Expected directory: {}", shims_dir.display()))
            .with_suggestion("Run 'kopi use <version>' to create shims for an installed JDK");
        }

        // Check if directory is readable
        match fs::read_dir(&shims_dir) {
            Ok(entries) => {
                let mut shim_count = 0;
                let mut executable_count = 0;
                let mut non_executable_shims = Vec::new();

                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        shim_count += 1;

                        // Check if it's a common Java executable
                        if let Some(name) = path.file_stem() {
                            let name_str = name.to_string_lossy();
                            if matches!(name_str.as_ref(), "java" | "javac" | "jar" | "jshell") {
                                if self.check_shim_executable(&path) {
                                    executable_count += 1;
                                } else {
                                    non_executable_shims.push(name_str.to_string());
                                }
                            }
                        }
                    }
                }

                if shim_count == 0 {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        "Shims directory exists but contains no shims",
                        start.elapsed(),
                    )
                    .with_suggestion("Run 'kopi use <version>' to activate a JDK and create shims")
                } else if !non_executable_shims.is_empty() {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Fail,
                        format!(
                            "Some shims are not executable: {}",
                            non_executable_shims.join(", ")
                        ),
                        start.elapsed(),
                    )
                    .with_suggestion(
                        "Fix permissions on shim files or recreate them with 'kopi use <version>'",
                    )
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        format!("Shims directory contains {executable_count} executable shims"),
                        start.elapsed(),
                    )
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Cannot read shims directory: {e}"),
                start.elapsed(),
            )
            .with_suggestion("Check directory permissions or recreate with 'kopi use <version>'"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> (TempDir, KopiConfig) {
        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path().join(".kopi");
        fs::create_dir_all(&kopi_home).unwrap();

        let config = KopiConfig::new(kopi_home).unwrap();

        (temp_dir, config)
    }

    #[test]
    fn test_path_check_missing() {
        let (_temp, config) = create_test_config();
        let check = PathCheck::new(&config);
        let result = check.run(Instant::now(), CheckCategory::Shell);

        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("not found in PATH"));
        assert!(result.suggestion.is_some());
    }

    #[test]
    fn test_path_check_present() {
        let (_temp, config) = create_test_config();
        let shims_dir = config.kopi_home().join("shims");
        fs::create_dir_all(&shims_dir).unwrap();

        // Temporarily modify PATH for test
        let original_path = env::var("PATH").unwrap_or_default();
        let separator = path_separator();
        let new_path = format!("{}{}{}", shims_dir.display(), separator, original_path);
        unsafe {
            env::set_var("PATH", new_path);
        }

        let check = PathCheck::new(&config);
        let result = check.run(Instant::now(), CheckCategory::Shell);

        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains("correctly configured"));

        // Restore PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_shell_detection_check() {
        let check = ShellDetectionCheck;
        let result = check.run(Instant::now(), CheckCategory::Shell);

        // Result depends on environment, but should not panic
        assert!(matches!(
            result.status,
            CheckStatus::Pass | CheckStatus::Warning
        ));
    }

    #[test]
    fn test_shim_functionality_no_dir() {
        let (_temp, config) = create_test_config();
        let check = ShimFunctionalityCheck::new(&config);
        let result = check.run(Instant::now(), CheckCategory::Shell);

        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("does not exist"));
    }

    #[test]
    fn test_shim_functionality_empty_dir() {
        let (_temp, config) = create_test_config();
        let shims_dir = config.kopi_home().join("shims");
        fs::create_dir_all(&shims_dir).unwrap();

        let check = ShimFunctionalityCheck::new(&config);
        let result = check.run(Instant::now(), CheckCategory::Shell);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.message.contains("contains no shims"));
    }

    #[test]
    fn test_shim_functionality_with_shims() {
        let (_temp, config) = create_test_config();
        let shims_dir = config.kopi_home().join("shims");
        fs::create_dir_all(&shims_dir).unwrap();

        // Create mock shim files
        let java_shim = if cfg!(windows) {
            shims_dir.join("java.exe")
        } else {
            shims_dir.join("java")
        };
        fs::write(&java_shim, "#!/bin/sh\necho mock").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&java_shim).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&java_shim, perms).unwrap();
        }

        let check = ShimFunctionalityCheck::new(&config);
        let result = check.run(Instant::now(), CheckCategory::Shell);

        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains("1 executable shims"));
    }

    #[test]
    fn test_path_priority_check() {
        let (_temp, config) = create_test_config();
        let shims_dir = config.kopi_home().join("shims");
        fs::create_dir_all(&shims_dir).unwrap();

        let check = PathCheck::new(&config);

        // Test with system Java before shims
        let original_path = env::var("PATH").unwrap_or_default();
        let separator = path_separator();

        // Create a mock system java directory
        let sys_java_dir = _temp.path().join("system_java");
        fs::create_dir_all(&sys_java_dir).unwrap();
        let java_exe = sys_java_dir.join("java");
        #[cfg(windows)]
        let java_exe = java_exe.with_extension("exe");
        fs::write(&java_exe, "mock").unwrap();

        let new_path = format!(
            "{}{}{}{}{}",
            sys_java_dir.display(),
            separator,
            shims_dir.display(),
            separator,
            original_path
        );
        unsafe {
            env::set_var("PATH", &new_path);
        }

        let priority_issue = check.check_path_priority(&shims_dir);
        assert!(priority_issue.is_some());
        assert!(priority_issue.unwrap().contains("comes after system Java"));

        // Restore PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }
}
