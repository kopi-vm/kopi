use crate::config::KopiConfig;
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use crate::platform::with_executable_extension;
use crate::storage::formatting::format_size;
use crate::storage::{InstalledJdk, JdkLister};
use std::time::Instant;

/// Check if any JDKs are installed
pub struct JdkInstallationCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkInstallationCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl<'a> DiagnosticCheck for JdkInstallationCheck<'a> {
    fn name(&self) -> &str {
        "JDK Installation Enumeration"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let jdks_dir = match self.config.jdks_dir() {
            Ok(dir) => dir,
            Err(e) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Fail,
                    format!("Cannot determine JDKs directory: {e}"),
                    start.elapsed(),
                )
                .with_suggestion("Ensure KOPI_HOME is set correctly or use default ~/.kopi");
            }
        };

        match JdkLister::list_installed_jdks(&jdks_dir) {
            Ok(jdks) => {
                if jdks.is_empty() {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        "No JDKs installed",
                        start.elapsed(),
                    )
                    .with_suggestion("Install a JDK with: kopi install <version>")
                } else {
                    let summary = format!(
                        "{} JDK{} installed",
                        jdks.len(),
                        if jdks.len() == 1 { "" } else { "s" }
                    );
                    let details = jdks
                        .iter()
                        .map(|jdk| format!("  - {}-{}", jdk.distribution, jdk.version))
                        .collect::<Vec<_>>()
                        .join("\n");

                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        summary,
                        start.elapsed(),
                    )
                    .with_details(details)
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Failed to list installed JDKs: {e}"),
                start.elapsed(),
            ),
        }
    }
}

/// Check JDK installation integrity
pub struct JdkIntegrityCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkIntegrityCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn check_jdk_structure(jdk: &InstalledJdk) -> Result<(bool, Vec<String>), std::io::Error> {
        let mut issues = Vec::new();
        let bin_dir = jdk.path.join("bin");

        // Check if bin directory exists
        if !bin_dir.exists() {
            issues.push("Missing bin directory".to_string());
            return Ok((false, issues));
        }

        // List of required executables
        let required_executables = vec!["java", "javac"];
        let optional_executables = vec!["jar", "javadoc", "jlink", "jmod"];

        // Check required executables
        for exe in required_executables {
            let exe_name = with_executable_extension(exe);
            let exe_path = bin_dir.join(&exe_name);

            if !exe_path.exists() {
                issues.push(format!("Missing required executable: {exe_name}"));
                continue;
            }

            // Check if executable permissions are correct
            match crate::platform::file_ops::is_executable(&exe_path) {
                Ok(is_exec) => {
                    if !is_exec {
                        issues.push(format!("{exe_name} is not executable"));
                    }
                }
                Err(e) => {
                    issues.push(format!("Cannot check {exe_name} permissions: {e}"));
                }
            }
        }

        // Check optional executables (just note if missing, don't fail)
        let mut missing_optional = Vec::new();
        for exe in optional_executables {
            let exe_name = with_executable_extension(exe);
            let exe_path = bin_dir.join(&exe_name);
            if !exe_path.exists() {
                missing_optional.push(exe);
            }
        }

        if !missing_optional.is_empty() {
            log::debug!(
                "JDK {} missing optional executables: {}",
                jdk.path.display(),
                missing_optional.join(", ")
            );
        }

        Ok((issues.is_empty(), issues))
    }
}

impl<'a> DiagnosticCheck for JdkIntegrityCheck<'a> {
    fn name(&self) -> &str {
        "JDK Installation Integrity"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let jdks_dir = match self.config.jdks_dir() {
            Ok(dir) => dir,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check JDK integrity - JDKs directory not accessible",
                    start.elapsed(),
                );
            }
        };

        let jdks = match JdkLister::list_installed_jdks(&jdks_dir) {
            Ok(jdks) => jdks,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check JDK integrity - failed to list JDKs",
                    start.elapsed(),
                );
            }
        };

        if jdks.is_empty() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "No JDKs installed to check",
                start.elapsed(),
            );
        }

        let mut all_issues = Vec::new();
        let mut corrupted_count = 0;

        for jdk in &jdks {
            match Self::check_jdk_structure(jdk) {
                Ok((is_valid, issues)) => {
                    if !is_valid {
                        corrupted_count += 1;
                        all_issues.push(format!(
                            "{}-{}:\n{}",
                            jdk.distribution,
                            jdk.version,
                            issues
                                .iter()
                                .map(|i| format!("    - {i}"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ));
                    }
                }
                Err(e) => {
                    corrupted_count += 1;
                    all_issues.push(format!(
                        "{}-{}: Failed to check structure: {e}",
                        jdk.distribution, jdk.version
                    ));
                }
            }
        }

        if corrupted_count == 0 {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                format!("All {} JDK installations are intact", jdks.len()),
                start.elapsed(),
            )
        } else {
            let message = format!(
                "{} of {} JDK installations have issues",
                corrupted_count,
                jdks.len()
            );
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                message,
                start.elapsed(),
            )
            .with_details(all_issues.join("\n\n"))
            .with_suggestion("Reinstall corrupted JDKs with: kopi install <distribution>@<version>")
        }
    }
}

/// Check available disk space for JDK installations
pub struct JdkDiskSpaceCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkDiskSpaceCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl<'a> DiagnosticCheck for JdkDiskSpaceCheck<'a> {
    fn name(&self) -> &str {
        "JDK Disk Space Analysis"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let jdks_dir = match self.config.jdks_dir() {
            Ok(dir) => dir,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot analyze disk space - JDKs directory not accessible",
                    start.elapsed(),
                );
            }
        };

        let jdks = match JdkLister::list_installed_jdks(&jdks_dir) {
            Ok(jdks) => jdks,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot analyze disk space - failed to list JDKs",
                    start.elapsed(),
                );
            }
        };

        if jdks.is_empty() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "No JDKs installed",
                start.elapsed(),
            );
        }

        // Calculate total size of all JDKs
        let mut total_size = 0u64;
        let mut jdk_sizes = Vec::new();

        for jdk in &jdks {
            match JdkLister::get_jdk_size(&jdk.path) {
                Ok(size) => {
                    total_size += size;
                    jdk_sizes.push((jdk, size));
                }
                Err(e) => {
                    log::warn!(
                        "Failed to calculate size for {}-{}: {}",
                        jdk.distribution,
                        jdk.version,
                        e
                    );
                }
            }
        }

        // Check available disk space
        let available_space = match fs2::available_space(&jdks_dir) {
            Ok(space) => space,
            Err(e) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Warning,
                    format!("Cannot check available disk space: {e}"),
                    start.elapsed(),
                );
            }
        };

        // Sort JDKs by size (largest first) for details
        jdk_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        let details = jdk_sizes
            .iter()
            .map(|(jdk, size)| {
                format!(
                    "  - {}-{}: {}",
                    jdk.distribution,
                    jdk.version,
                    format_size(*size)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let total_size_str = format_size(total_size);
        let available_str = format_size(available_space);

        // Warn if less than 1GB available
        let status = if available_space < 1024 * 1024 * 1024 {
            CheckStatus::Warning
        } else {
            CheckStatus::Pass
        };

        let message = format!("JDKs using {total_size_str}, {available_str} available");

        let mut result = CheckResult::new(self.name(), category, status, message, start.elapsed())
            .with_details(format!("JDK sizes:\n{details}"));

        if status == CheckStatus::Warning {
            result = result.with_suggestion(
                "Low disk space. Consider removing unused JDKs with: kopi uninstall <version>",
            );
        }

        result
    }
}

/// Check JDK version consistency between directory name and actual version
pub struct JdkVersionConsistencyCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkVersionConsistencyCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn check_java_version(jdk: &InstalledJdk) -> Result<(bool, String), std::io::Error> {
        let java_path = jdk.path.join("bin").join(with_executable_extension("java"));

        if !java_path.exists() {
            return Ok((false, "Java executable not found".to_string()));
        }

        // Run java -version to get actual version
        let output = std::process::Command::new(&java_path)
            .arg("-version")
            .output()?;

        // Java outputs version info to stderr
        let version_output = String::from_utf8_lossy(&output.stderr);

        // Extract version from output (usually in format: openjdk version "21.0.1" or java version "17.0.9")
        let version_line = version_output
            .lines()
            .find(|line| line.contains("version"))
            .unwrap_or("");

        Ok((true, version_line.to_string()))
    }
}

impl<'a> DiagnosticCheck for JdkVersionConsistencyCheck<'a> {
    fn name(&self) -> &str {
        "JDK Version Consistency"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let jdks_dir = match self.config.jdks_dir() {
            Ok(dir) => dir,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check version consistency - JDKs directory not accessible",
                    start.elapsed(),
                );
            }
        };

        let jdks = match JdkLister::list_installed_jdks(&jdks_dir) {
            Ok(jdks) => jdks,
            Err(_) => {
                return CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot check version consistency - failed to list JDKs",
                    start.elapsed(),
                );
            }
        };

        if jdks.is_empty() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "No JDKs installed to check",
                start.elapsed(),
            );
        }

        let mut inconsistencies = Vec::new();

        for jdk in &jdks {
            match Self::check_java_version(jdk) {
                Ok((found, version_output)) => {
                    if found {
                        // Simple check: see if the directory version appears in the output
                        let dir_version = jdk.version.to_string();
                        if !version_output.contains(&dir_version) {
                            inconsistencies.push(format!(
                                "{}-{}: {}",
                                jdk.distribution,
                                jdk.version,
                                version_output.trim()
                            ));
                        }
                    } else {
                        inconsistencies.push(format!(
                            "{}-{}: Java executable not found",
                            jdk.distribution, jdk.version
                        ));
                    }
                }
                Err(e) => {
                    log::debug!(
                        "Failed to check version for {}-{}: {}",
                        jdk.distribution,
                        jdk.version,
                        e
                    );
                }
            }
        }

        if inconsistencies.is_empty() {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "All JDK versions are consistent",
                start.elapsed(),
            )
        } else {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!(
                    "{} JDK{} may have version inconsistencies",
                    inconsistencies.len(),
                    if inconsistencies.len() == 1 { "" } else { "s" }
                ),
                start.elapsed(),
            )
            .with_details(inconsistencies.join("\n"))
            .with_suggestion("Version mismatch may indicate corrupted installations")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    struct TestSetup {
        config: KopiConfig,
        _temp_dir: TempDir,
    }

    impl TestSetup {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

            // Create jdks directory
            fs::create_dir_all(temp_dir.path().join("jdks")).unwrap();

            TestSetup {
                config,
                _temp_dir: temp_dir,
            }
        }

        fn create_mock_jdk(&self, name: &str) {
            let jdk_path = self.config.jdks_dir().unwrap().join(name);
            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();

            // Create mock executables
            for exe in &["java", "javac"] {
                let exe_name = with_executable_extension(exe);
                let exe_path = bin_dir.join(exe_name);
                fs::write(&exe_path, "mock executable").unwrap();

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&exe_path).unwrap().permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&exe_path, perms).unwrap();
                }
            }
        }
    }

    #[test]
    fn test_jdk_installation_check_empty() {
        let setup = TestSetup::new();
        let check = JdkInstallationCheck::new(&setup.config);
        let result = check.run(Instant::now(), CheckCategory::Jdks);

        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.message.contains("No JDKs installed"));
        assert!(result.suggestion.is_some());
    }

    #[test]
    fn test_jdk_installation_check_with_jdks() {
        let setup = TestSetup::new();
        setup.create_mock_jdk("temurin-21.0.1");
        setup.create_mock_jdk("corretto-17.0.9");

        let check = JdkInstallationCheck::new(&setup.config);
        let result = check.run(Instant::now(), CheckCategory::Jdks);

        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains("2 JDKs installed"));
        assert!(result.details.is_some());
    }

    #[test]
    fn test_jdk_integrity_check() {
        let setup = TestSetup::new();

        // Create a valid JDK
        setup.create_mock_jdk("temurin-21.0.1");

        // Create a corrupted JDK (missing bin directory)
        let corrupted_path = setup.config.jdks_dir().unwrap().join("corretto-17.0.9");
        fs::create_dir_all(&corrupted_path).unwrap();

        let check = JdkIntegrityCheck::new(&setup.config);
        let result = check.run(Instant::now(), CheckCategory::Jdks);

        assert_eq!(result.status, CheckStatus::Fail);
        assert!(
            result
                .message
                .contains("1 of 2 JDK installations have issues")
        );
    }

    #[test]
    fn test_jdk_disk_space_check() {
        let setup = TestSetup::new();
        setup.create_mock_jdk("temurin-21.0.1");

        let check = JdkDiskSpaceCheck::new(&setup.config);
        let result = check.run(Instant::now(), CheckCategory::Jdks);

        // Should at least run without errors
        assert!(
            result.status == CheckStatus::Pass || result.status == CheckStatus::Warning,
            "Unexpected status: {:?}",
            result.status
        );
        assert!(result.message.contains("JDKs using"));
        assert!(result.message.contains("available"));
    }
}
