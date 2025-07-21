use std::fmt;
use std::time::{Duration, Instant};

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
    ) -> Self {
        Self {
            name: name.into(),
            category,
            status,
            message: message.into(),
            details: None,
            suggestion: None,
            duration: Duration::default(),
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

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
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
    fn category(&self) -> CheckCategory;
    fn run(&self) -> Result<CheckResult, Box<dyn std::error::Error>>;
}

pub struct DiagnosticEngine {
    checks: Vec<Box<dyn DiagnosticCheck>>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add_check(&mut self, check: Box<dyn DiagnosticCheck>) {
        self.checks.push(check);
    }

    pub fn run_checks(&self, categories: Option<Vec<CheckCategory>>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        for check in &self.checks {
            if let Some(ref cats) = categories {
                if !cats.contains(&check.category()) {
                    continue;
                }
            }

            let start = Instant::now();
            let result = match check.run() {
                Ok(mut result) => {
                    result.duration = start.elapsed();
                    result
                }
                Err(e) => CheckResult::new(
                    check.name(),
                    check.category(),
                    CheckStatus::Fail,
                    format!("Check failed: {e}"),
                )
                .with_duration(start.elapsed()),
            };

            results.push(result);
        }

        results
    }
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCheck {
        name: String,
        category: CheckCategory,
        status: CheckStatus,
        message: String,
    }

    impl DiagnosticCheck for MockCheck {
        fn name(&self) -> &str {
            &self.name
        }

        fn category(&self) -> CheckCategory {
            self.category
        }

        fn run(&self) -> Result<CheckResult, Box<dyn std::error::Error>> {
            Ok(CheckResult::new(
                &self.name,
                self.category,
                self.status,
                &self.message,
            ))
        }
    }

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
            ),
            CheckResult::new("test2", CheckCategory::Shell, CheckStatus::Fail, "Failed"),
            CheckResult::new(
                "test3",
                CheckCategory::Network,
                CheckStatus::Warning,
                "Warning",
            ),
            CheckResult::new("test4", CheckCategory::Cache, CheckStatus::Skip, "Skipped"),
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
        )];
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 0);

        results.push(CheckResult::new(
            "test2",
            CheckCategory::Shell,
            CheckStatus::Warning,
            "Warning",
        ));
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 2);

        results.push(CheckResult::new(
            "test3",
            CheckCategory::Network,
            CheckStatus::Fail,
            "Failed",
        ));
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));
        assert_eq!(summary.determine_exit_code(), 1);
    }

    #[test]
    fn test_diagnostic_engine() {
        let mut engine = DiagnosticEngine::new();

        engine.add_check(Box::new(MockCheck {
            name: "test1".to_string(),
            category: CheckCategory::Installation,
            status: CheckStatus::Pass,
            message: "Installation OK".to_string(),
        }));

        engine.add_check(Box::new(MockCheck {
            name: "test2".to_string(),
            category: CheckCategory::Shell,
            status: CheckStatus::Fail,
            message: "Shell config missing".to_string(),
        }));

        let results = engine.run_checks(None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, CheckStatus::Pass);
        assert_eq!(results[1].status, CheckStatus::Fail);
    }

    #[test]
    fn test_category_filtering() {
        let mut engine = DiagnosticEngine::new();

        engine.add_check(Box::new(MockCheck {
            name: "install_check".to_string(),
            category: CheckCategory::Installation,
            status: CheckStatus::Pass,
            message: "OK".to_string(),
        }));

        engine.add_check(Box::new(MockCheck {
            name: "shell_check".to_string(),
            category: CheckCategory::Shell,
            status: CheckStatus::Pass,
            message: "OK".to_string(),
        }));

        let results = engine.run_checks(Some(vec![CheckCategory::Installation]));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "install_check");
    }
}
