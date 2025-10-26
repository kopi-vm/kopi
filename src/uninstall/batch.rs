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

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::indicator::StatusReporter;
use crate::locking::{InstalledScopeResolver, LockBackend, LockController, ScopedPackageLockGuard};
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

pub struct BatchUninstaller<'a> {
    config: &'a KopiConfig,
    repository: &'a JdkRepository<'a>,
    no_progress: bool,
}

impl<'a> BatchUninstaller<'a> {
    pub fn new(
        config: &'a KopiConfig,
        repository: &'a JdkRepository<'a>,
        no_progress: bool,
    ) -> Self {
        Self {
            config,
            repository,
            no_progress,
        }
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

        // Show summary only in dry-run mode
        if dry_run {
            self.display_batch_summary(&jdks, total_size)?;
            return Ok(());
        }

        // Confirm unless forced
        if !force && !self.confirm_batch_removal(&jdks)? {
            return Err(KopiError::SystemError(
                "User cancelled operation".to_string(),
            ));
        }

        // Add a newline after confirmation for cleaner output
        println!();

        // Perform batch removal with transaction-like behavior
        self.execute_batch_removal(jdks, total_size, force)
    }

    fn calculate_total_size(&self, jdks: &[InstalledJdk]) -> Result<u64> {
        let mut total = 0u64;
        for jdk in jdks {
            total += self.repository.get_jdk_size(&jdk.path)?;
        }
        Ok(total)
    }

    fn display_batch_summary(&self, jdks: &[InstalledJdk], total_size: u64) -> Result<()> {
        let reporter = StatusReporter::new(self.no_progress);

        reporter.operation("JDKs to be removed", "");

        for jdk in jdks {
            let size = self.repository.get_jdk_size(&jdk.path)?;
            reporter.step(&format!(
                "- {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(size)
            ));
        }

        reporter.step(&format!(
            "Total: {} JDKs, {}",
            jdks.len(),
            format_size(total_size)
        ));

        Ok(())
    }

    fn confirm_batch_removal(&self, jdks: &[InstalledJdk]) -> Result<bool> {
        let total_size = self.calculate_total_size(jdks)?;
        display_batch_uninstall_confirmation(jdks, total_size)
    }

    fn execute_batch_removal(
        &self,
        jdks: Vec<InstalledJdk>,
        total_size: u64,
        force: bool,
    ) -> Result<()> {
        let mut progress_reporter = ProgressReporter::new_batch(self.no_progress);
        let overall_pb = progress_reporter.create_batch_removal_bar(jdks.len() as u64);

        let mut removed_count = 0;
        let mut failed_jdks = Vec::new();
        let mut removed_jdks = Vec::new();
        let mut log_messages = Vec::new(); // Collect log messages to output after progress

        let reporter = StatusReporter::new(self.no_progress);
        let controller = LockController::with_default_inspector(
            self.config.kopi_home().to_path_buf(),
            &self.config.locking,
        );
        let scope_resolver = InstalledScopeResolver::new(self.repository);

        for jdk in &jdks {
            log_messages.push(format!("Removing {}@{}", jdk.distribution, jdk.version));

            // Update overall progress bar message for current JDK
            overall_pb.set_message(format!("Removing {}@{}...", jdk.distribution, jdk.version));

            let removal_result = (|| -> Result<()> {
                let scope = scope_resolver.resolve(jdk)?;
                let scope_label = scope.label().to_string();

                let mut acquisition_result = None;
                progress_reporter.suspend(|| {
                    reporter.step(&format!("Acquiring uninstall lock for {scope_label}"));
                    acquisition_result =
                        Some(controller.acquire_with_status_sink(scope.clone(), &reporter));
                });
                let acquisition =
                    acquisition_result.expect("lock acquisition attempt did not run")?;
                let uninstall_lock_guard = ScopedPackageLockGuard::new(&controller, acquisition);
                let backend_label = match uninstall_lock_guard.backend() {
                    LockBackend::Advisory => "advisory",
                    LockBackend::Fallback => "fallback",
                };

                progress_reporter.suspend(|| {
                    reporter.step(&format!("Using {backend_label} backend for {scope_label}"));
                });
                info!("Acquired uninstall lock for {scope_label} using {backend_label} backend");

                crate::uninstall::safety::perform_safety_checks(
                    self.config,
                    self.repository,
                    jdk,
                    force,
                )?;

                match self.repository.remove_jdk(&jdk.path) {
                    Ok(()) => uninstall_lock_guard.release(),
                    Err(err) => Err(err),
                }
            })();

            overall_pb.inc(1);

            match removal_result {
                Ok(()) => {
                    removed_count += 1;
                    removed_jdks.push(jdk.clone());
                }
                Err(err) => {
                    let err_string = err.to_string();
                    log_messages.push(format!(
                        "Failed to remove {}@{}: {err_string}",
                        jdk.distribution, jdk.version
                    ));
                    warn!(
                        "Failed to remove {}@{}: {err_string}",
                        jdk.distribution, jdk.version
                    );
                    failed_jdks.push((jdk.clone(), err));
                }
            }
        }

        overall_pb.finish_and_clear();

        // Show status messages after progress bar is done using StatusReporter
        for jdk in &removed_jdks {
            reporter.success(&format!("Removed {}@{}", jdk.distribution, jdk.version));
        }

        for (jdk, _) in &failed_jdks {
            reporter.error(&format!(
                "Failed to remove {}@{}",
                jdk.distribution, jdk.version
            ));
        }

        // Now output the collected log messages after all progress indicators are finished
        for msg in log_messages {
            if msg.starts_with("Removing ") {
                info!("{msg}");
            } else if msg.starts_with("Safety check failed") || msg.starts_with("Failed to remove")
            {
                warn!("{msg}");
            }
        }

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
    use crate::locking::{
        InstalledScopeResolver, LockController, LockTimeoutValue, ScopedPackageLockGuard,
    };
    use crate::paths::install;
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
        let batch_uninstaller = BatchUninstaller::new(&config, &repository, false);

        // Create some test directories
        install::ensure_installations_root(temp_dir.path()).unwrap();
        let jdk1_path = install::installation_directory(temp_dir.path(), "temurin-21.0.5+11");
        let jdk2_path = install::installation_directory(temp_dir.path(), "corretto-17.0.9");
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
        let batch_uninstaller = BatchUninstaller::new(&config, &repository, false);

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

    #[test]
    fn batch_uninstall_handles_lock_contention() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        config
            .locking
            .set_timeout_value(LockTimeoutValue::from_secs(0));
        let repository = JdkRepository::new(&config);
        install::ensure_installations_root(temp_dir.path()).unwrap();

        let locked_slug = "temurin-21.0.5+11";
        let other_slug = "corretto-17.0.9";
        let locked_path = install::installation_directory(temp_dir.path(), locked_slug);
        let other_path = install::installation_directory(temp_dir.path(), other_slug);
        std::fs::create_dir_all(&locked_path).unwrap();
        std::fs::create_dir_all(&other_path).unwrap();

        let locked_jdk =
            create_test_jdk_with_path("temurin", "21.0.5+11", locked_path.to_str().unwrap());
        let other_jdk =
            create_test_jdk_with_path("corretto", "17.0.9", other_path.to_str().unwrap());

        let controller = LockController::with_default_inspector(
            config.kopi_home().to_path_buf(),
            &config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let scope = resolver.resolve(&locked_jdk).unwrap();
        let acquisition = controller.acquire(scope.clone()).unwrap();
        let guard = ScopedPackageLockGuard::new(&controller, acquisition);

        let batch_uninstaller = BatchUninstaller::new(&config, &repository, true);
        let result = batch_uninstaller.uninstall_batch(
            vec![locked_jdk.clone(), other_jdk.clone()],
            true,
            false,
        );
        assert!(
            result.is_ok(),
            "batch uninstall should continue despite lock contention"
        );
        assert!(
            locked_jdk.path.exists(),
            "locked JDK should remain when lock acquisition times out"
        );
        assert!(
            !other_jdk.path.exists(),
            "unlocked JDK should be removed successfully"
        );

        guard.release().unwrap();
        if locked_jdk.path.exists() {
            std::fs::remove_dir_all(&locked_jdk.path).unwrap();
        }
    }

    #[test]
    fn batch_uninstall_releases_lock_after_success() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        install::ensure_installations_root(temp_dir.path()).unwrap();

        let slug = "temurin-21.0.5+11";
        let path = install::installation_directory(temp_dir.path(), slug);
        std::fs::create_dir_all(&path).unwrap();
        let jdk = create_test_jdk_with_path("temurin", "21.0.5+11", path.to_str().unwrap());

        let resolver = InstalledScopeResolver::new(&repository);
        let scope = resolver.resolve(&jdk).unwrap();

        let batch_uninstaller = BatchUninstaller::new(&config, &repository, true);
        batch_uninstaller
            .uninstall_batch(vec![jdk.clone()], true, false)
            .expect("batch uninstall should succeed");
        assert!(
            !jdk.path.exists(),
            "JDK directory should be removed after successful uninstall"
        );

        let controller = LockController::with_default_inspector(
            config.kopi_home().to_path_buf(),
            &config.locking,
        );
        let reacquired = controller.try_acquire(scope.clone()).unwrap();
        assert!(
            reacquired.is_some(),
            "lock scope should be re-acquirable after uninstall completes"
        );
        if let Some(handle) = reacquired {
            controller.release(handle).unwrap();
        }
    }
}
