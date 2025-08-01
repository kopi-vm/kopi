use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution;
use crate::storage::formatting::format_size;
use crate::storage::{InstalledJdk, JdkRepository};
use crate::uninstall::feedback::{
    display_batch_uninstall_confirmation, display_batch_uninstall_summary,
};
use crate::uninstall::progress::ProgressReporter;
use crate::version::VersionRequest;
use log::{debug, info, warn};
use std::str::FromStr;
use std::time::Duration;

pub struct BatchUninstaller<'a> {
    config: &'a KopiConfig,
    repository: &'a JdkRepository<'a>,
}

impl<'a> BatchUninstaller<'a> {
    pub fn new(config: &'a KopiConfig, repository: &'a JdkRepository<'a>) -> Self {
        Self { config, repository }
    }

    pub fn uninstall_all(&self, spec: Option<&str>, force: bool, dry_run: bool) -> Result<()> {
        let jdks = if let Some(spec_str) = spec {
            // Build the list of all known distributions (built-in + additional)
            let mut all_distributions: Vec<String> = Distribution::known_distributions()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            all_distributions.extend(self.config.additional_distributions.clone());

            // Check if spec is a distribution name (case-insensitive)
            let is_distribution = all_distributions
                .iter()
                .any(|dist| dist.eq_ignore_ascii_case(spec_str));

            if is_distribution {
                // Filter installed JDKs by distribution
                debug!("Filtering JDKs by distribution: {spec_str}");
                let all_jdks = self.repository.list_installed_jdks()?;
                all_jdks
                    .into_iter()
                    .filter(|jdk| jdk.distribution.eq_ignore_ascii_case(spec_str))
                    .collect()
            } else {
                // Parse as version request
                let version_request = VersionRequest::from_str(spec_str)?;
                debug!("Parsed version request: {version_request:?}");
                self.repository.find_matching_jdks(&version_request)?
            }
        } else {
            // List all installed JDKs if no spec provided
            self.repository.list_installed_jdks()?
        };

        if jdks.is_empty() {
            return Err(KopiError::JdkNotInstalled {
                jdk_spec: spec.unwrap_or("all").to_string(),
                version: None,
                distribution: None,
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            });
        }

        self.uninstall_batch(jdks, force, dry_run)
    }

    pub fn uninstall_batch(
        &self,
        jdks: Vec<InstalledJdk>,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        if jdks.is_empty() {
            return Ok(());
        }

        // Calculate total size
        let total_size = self.calculate_total_size(&jdks)?;

        // Show summary
        self.display_batch_summary(&jdks, total_size)?;

        if dry_run {
            return Ok(());
        }

        // Confirm unless forced
        if !force && !self.confirm_batch_removal(&jdks)? {
            return Err(KopiError::SystemError(
                "User cancelled operation".to_string(),
            ));
        }

        // Perform batch removal with transaction-like behavior
        self.execute_batch_removal(jdks, total_size)
    }

    fn calculate_total_size(&self, jdks: &[InstalledJdk]) -> Result<u64> {
        let mut total = 0u64;
        for jdk in jdks {
            total += self.repository.get_jdk_size(&jdk.path)?;
        }
        Ok(total)
    }

    fn display_batch_summary(&self, jdks: &[InstalledJdk], total_size: u64) -> Result<()> {
        println!("JDKs to be removed:");
        println!();

        for jdk in jdks {
            let size = self.repository.get_jdk_size(&jdk.path)?;
            println!(
                "  - {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(size)
            );
        }

        println!();
        println!("Total: {} JDKs, {}", jdks.len(), format_size(total_size));

        Ok(())
    }

    fn confirm_batch_removal(&self, jdks: &[InstalledJdk]) -> Result<bool> {
        let total_size = self.calculate_total_size(jdks)?;
        display_batch_uninstall_confirmation(jdks, total_size)
    }

    fn execute_batch_removal(&self, jdks: Vec<InstalledJdk>, total_size: u64) -> Result<()> {
        let progress_reporter = ProgressReporter::new_batch();
        let overall_pb = progress_reporter.create_batch_removal_bar(jdks.len() as u64);

        let mut removed_count = 0;
        let mut failed_jdks = Vec::new();
        let mut removed_jdks = Vec::new();

        for jdk in &jdks {
            info!("Removing {}@{}", jdk.distribution, jdk.version);

            // Create spinner for current JDK
            let spinner = progress_reporter
                .create_spinner(&format!("Removing {}@{}...", jdk.distribution, jdk.version));
            spinner.enable_steady_tick(Duration::from_millis(100));

            // Perform safety checks
            match crate::uninstall::safety::perform_safety_checks(
                &jdk.distribution.to_string(),
                &jdk.version.to_string(),
            ) {
                Ok(()) => {}
                Err(e) => {
                    warn!(
                        "Safety check failed for {}@{}: {}",
                        jdk.distribution, jdk.version, e
                    );
                    spinner.finish_and_clear();
                    failed_jdks.push((jdk.clone(), e));
                    continue;
                }
            }

            // Attempt removal
            match self.repository.remove_jdk(&jdk.path) {
                Ok(()) => {
                    removed_count += 1;
                    removed_jdks.push(jdk.clone());
                    spinner.finish_with_message(format!(
                        "✓ Removed {}@{}",
                        jdk.distribution, jdk.version
                    ));
                    overall_pb.inc(1);
                }
                Err(e) => {
                    warn!(
                        "Failed to remove {}@{}: {}",
                        jdk.distribution, jdk.version, e
                    );
                    spinner.finish_with_message(format!(
                        "✗ Failed to remove {}@{}",
                        jdk.distribution, jdk.version
                    ));
                    failed_jdks.push((jdk.clone(), e));
                }
            }
        }

        overall_pb.finish_and_clear();

        // Report results
        let failed_with_messages: Vec<(InstalledJdk, String)> = failed_jdks
            .into_iter()
            .map(|(jdk, err)| (jdk, err.to_string()))
            .collect();

        display_batch_uninstall_summary(&removed_jdks, &failed_with_messages, total_size);

        // Return error if all removals failed
        if removed_count == 0 {
            return Err(KopiError::SystemError(
                "All JDK removals failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use mockall::mock;
    use tempfile::TempDir;

    // Import shared test fixtures
    use crate::test::fixtures::create_test_jdk_with_path;

    // Mock JdkRepository trait
    mock! {
        JdkRepo {
            fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>>;
            fn get_jdk_size(&self, path: &std::path::Path) -> Result<u64>;
            fn remove_jdk(&self, path: &std::path::Path) -> Result<()>;
        }
    }

    #[test]
    fn test_calculate_total_size() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let batch_uninstaller = BatchUninstaller::new(&config, &repository);

        // Create some test directories
        let jdk1_path = temp_dir.path().join("jdks").join("temurin-21.0.5+11");
        let jdk2_path = temp_dir.path().join("jdks").join("corretto-17.0.9");
        std::fs::create_dir_all(&jdk1_path).unwrap();
        std::fs::create_dir_all(&jdk2_path).unwrap();

        // Write some test files
        std::fs::write(jdk1_path.join("test1.txt"), vec![0u8; 1024]).unwrap();
        std::fs::write(jdk2_path.join("test2.txt"), vec![0u8; 2048]).unwrap();

        let jdks = vec![
            create_test_jdk_with_path("temurin", "21.0.5+11", jdk1_path.to_str().unwrap()),
            create_test_jdk_with_path("corretto", "17.0.9", jdk2_path.to_str().unwrap()),
        ];

        let total_size = batch_uninstaller.calculate_total_size(&jdks).unwrap();
        assert_eq!(total_size, 3072);
    }

    #[test]
    fn test_uninstall_all_invalid_spec() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let batch_uninstaller = BatchUninstaller::new(&config, &repository);

        // Test invalid version spec
        let result = batch_uninstaller.uninstall_all(Some("invalid.version"), false, false);
        assert!(result.is_err());
        match result {
            Err(KopiError::InvalidVersionFormat(_)) => {
                // Good, got the expected error type
            }
            _ => panic!("Expected InvalidVersionFormat error"),
        }
    }
}
