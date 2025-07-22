use std::fmt;
use std::time::{Duration, Instant};

pub mod checks;
pub mod formatters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    Skip,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "pass"),
            CheckStatus::Fail => write!(f, "fail"),
            CheckStatus::Warning => write!(f, "warning"),
            CheckStatus::Skip => write!(f, "skip"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckCategory {
    Installation,
    Shell,
    Jdks,
    Permissions,
    Network,
    Cache,
}

impl fmt::Display for CheckCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckCategory::Installation => write!(f, "Installation"),
            CheckCategory::Shell => write!(f, "Shell"),
            CheckCategory::Jdks => write!(f, "JDKs"),
            CheckCategory::Permissions => write!(f, "Permissions"),
            CheckCategory::Network => write!(f, "Network"),
            CheckCategory::Cache => write!(f, "Cache"),
        }
    }
}

impl CheckCategory {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "installation" => Some(CheckCategory::Installation),
            "shell" => Some(CheckCategory::Shell),
            "jdks" => Some(CheckCategory::Jdks),
            "permissions" => Some(CheckCategory::Permissions),
            "network" => Some(CheckCategory::Network),
            "cache" => Some(CheckCategory::Cache),
            _ => None,
        }
    }

    pub fn all() -> Vec<CheckCategory> {
        vec![
            CheckCategory::Installation,
            CheckCategory::Shell,
            CheckCategory::Jdks,
            CheckCategory::Permissions,
            CheckCategory::Network,
            CheckCategory::Cache,
        ]
    }

    /// Create all diagnostic checks for this category
    pub fn create_checks<'a>(
        &self,
        config: &'a crate::config::KopiConfig,
    ) -> Vec<Box<dyn DiagnosticCheck + 'a>> {
        use crate::doctor::checks::{
            BinaryPermissionsCheck, ConfigFileCheck, DirectoryPermissionsCheck,
            InstallationDirectoryCheck, JdkDiskSpaceCheck, JdkInstallationCheck, JdkIntegrityCheck,
            JdkVersionConsistencyCheck, KopiBinaryCheck, OwnershipCheck, PathCheck,
            ShellConfigurationCheck, ShellDetectionCheck, ShimFunctionalityCheck, ShimsInPathCheck,
            VersionCheck,
        };

        match self {
            CheckCategory::Installation => vec![
                Box::new(KopiBinaryCheck) as Box<dyn DiagnosticCheck + 'a>,
                Box::new(VersionCheck),
                Box::new(InstallationDirectoryCheck::new(config)),
                Box::new(ConfigFileCheck::new(config)),
                Box::new(ShimsInPathCheck::new(config)),
            ],
            CheckCategory::Permissions => vec![
                Box::new(DirectoryPermissionsCheck::new(config)),
                Box::new(BinaryPermissionsCheck::new(config)),
                Box::new(OwnershipCheck::new(config)),
            ],
            CheckCategory::Shell => vec![
                Box::new(ShellDetectionCheck) as Box<dyn DiagnosticCheck + 'a>,
                Box::new(PathCheck::new(config)),
                Box::new(ShellConfigurationCheck),
                Box::new(ShimFunctionalityCheck::new(config)),
            ],
            CheckCategory::Jdks => vec![
                Box::new(JdkInstallationCheck::new(config)) as Box<dyn DiagnosticCheck + 'a>,
                Box::new(JdkIntegrityCheck::new(config)),
                Box::new(JdkDiskSpaceCheck::new(config)),
                Box::new(JdkVersionConsistencyCheck::new(config)),
            ],
            CheckCategory::Network => vec![
                // Phase 3: Network connectivity checks will go here
            ],
            CheckCategory::Cache => vec![
                // Phase 3: Cache validation checks will go here
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
    pub duration: Duration,
}

impl CheckResult {
    pub fn new(
        name: impl Into<String>,
        category: CheckCategory,
        status: CheckStatus,
        message: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            category,
            status,
            message: message.into(),
            details: None,
            suggestion: None,
            duration,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

pub struct DiagnosticSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub skipped: usize,
    pub total_duration: Duration,
}

impl DiagnosticSummary {
    pub fn from_results(results: &[CheckResult], total_duration: Duration) -> Self {
        let mut passed = 0;
        let mut failed = 0;
        let mut warnings = 0;
        let mut skipped = 0;

        for result in results {
            match result.status {
                CheckStatus::Pass => passed += 1,
                CheckStatus::Fail => failed += 1,
                CheckStatus::Warning => warnings += 1,
                CheckStatus::Skip => skipped += 1,
            }
        }

        Self {
            total_checks: results.len(),
            passed,
            failed,
            warnings,
            skipped,
            total_duration,
        }
    }

    pub fn determine_exit_code(&self) -> i32 {
        if self.failed > 0 {
            1
        } else if self.warnings > 0 {
            2
        } else {
            0
        }
    }
}

pub trait DiagnosticCheck: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult;
}

pub struct DiagnosticEngine<'a> {
    config: &'a crate::config::KopiConfig,
}

impl<'a> DiagnosticEngine<'a> {
    pub fn new(config: &'a crate::config::KopiConfig) -> Self {
        Self { config }
    }

    pub fn run_checks(&self, categories: Option<Vec<CheckCategory>>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        // Determine which categories to run
        let categories_to_run = categories.unwrap_or_else(CheckCategory::all);

        // Create checks for each category and run them
        for category in categories_to_run {
            let checks = category.create_checks(self.config);

            for check in checks {
                let start = Instant::now();
                let result = check.run(start, category);
                results.push(result);
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_display() {
        assert_eq!(CheckStatus::Pass.to_string(), "pass");
        assert_eq!(CheckStatus::Fail.to_string(), "fail");
        assert_eq!(CheckStatus::Warning.to_string(), "warning");
        assert_eq!(CheckStatus::Skip.to_string(), "skip");
    }

    #[test]
    fn test_check_category_display() {
        assert_eq!(CheckCategory::Installation.to_string(), "Installation");
        assert_eq!(CheckCategory::Shell.to_string(), "Shell");
        assert_eq!(CheckCategory::Jdks.to_string(), "JDKs");
    }

    #[test]
    fn test_check_category_parse() {
        assert_eq!(
            CheckCategory::parse("installation"),
            Some(CheckCategory::Installation)
        );
        assert_eq!(CheckCategory::parse("SHELL"), Some(CheckCategory::Shell));
        assert_eq!(CheckCategory::parse("invalid"), None);
    }

    #[test]
    fn test_diagnostic_summary() {
        let results = vec![
            CheckResult::new(
                "test1",
                CheckCategory::Installation,
                CheckStatus::Pass,
                "OK",
                Duration::from_millis(100),
            ),
            CheckResult::new(
                "test2",
                CheckCategory::Shell,
                CheckStatus::Fail,
                "Failed",
                Duration::from_millis(200),
            ),
            CheckResult::new(
                "test3",
                CheckCategory::Network,
                CheckStatus::Warning,
                "Warning",
                Duration::from_millis(150),
            ),
            CheckResult::new(
                "test4",
                CheckCategory::Cache,
                CheckStatus::Skip,
                "Skipped",
                Duration::from_millis(50),
            ),
        ];

        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));

        assert_eq!(summary.total_checks, 4);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn test_exit_code_determination() {
        let mut results = vec![CheckResult::new(
            "test1",
            CheckCategory::Installation,
            CheckStatus::Pass,
            "OK",
            Duration::from_millis(100),
        )];
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 0);

        results.push(CheckResult::new(
            "test2",
            CheckCategory::Shell,
            CheckStatus::Warning,
            "Warning",
            Duration::from_millis(200),
        ));
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 2);

        results.push(CheckResult::new(
            "test3",
            CheckCategory::Network,
            CheckStatus::Fail,
            "Failed",
            Duration::from_millis(150),
        ));
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 1);
    }

    // Note: DiagnosticEngine tests are now integration tests since it requires
    // a real KopiConfig and initializes all checks internally
}
