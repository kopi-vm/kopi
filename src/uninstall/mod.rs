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
use crate::platform;
use crate::storage::formatting::format_size;
use crate::storage::{InstalledJdk, JdkRepository};
use crate::uninstall::cleanup::UninstallCleanup;
use crate::uninstall::error_formatting::format_multiple_jdk_matches_error;
use crate::uninstall::progress::ProgressReporter;
use log::{debug, info, warn};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

pub mod batch;
pub mod cleanup;
pub mod error_formatting;
pub mod feedback;
pub mod post_check;
pub mod progress;
pub mod safety;
pub mod selection;

pub struct UninstallHandler<'a> {
    config: &'a KopiConfig,
    repository: &'a JdkRepository<'a>,
    no_progress: bool,
}

impl<'a> UninstallHandler<'a> {
    pub fn new(repository: &'a JdkRepository<'a>, no_progress: bool) -> Self {
        let config = repository.config();
        Self {
            config,
            repository,
            no_progress,
        }
    }

    /// Perform cleanup operations for failed uninstalls
    pub fn recover_from_failures(&self, force: bool) -> Result<()> {
        let reporter = StatusReporter::new(self.no_progress);
        let cleanup = UninstallCleanup::new(self.repository);

        let actions = cleanup.detect_and_cleanup_partial_removals()?;

        if actions.is_empty() {
            reporter.step("No recovery actions needed.");
            return Ok(());
        }

        reporter.operation(
            "Found recovery actions",
            &format!("{} actions", actions.len()),
        );
        for action in &actions {
            reporter.step(&format!("- {action:?}"));
        }

        let result = cleanup.execute_cleanup(actions, force)?;

        if result.is_success() {
            reporter.success("Recovery completed successfully");
            for success in result.successes {
                reporter.step(&format!("✓ {success}"));
            }
        } else {
            reporter.error("Recovery completed with errors");
            for success in result.successes {
                reporter.step(&format!("✓ {success}"));
            }
            for failure in result.failures {
                reporter.step(&format!("✗ {failure}"));
            }
        }

        Ok(())
    }

    pub fn uninstall_jdk(&self, version_spec: &str, force: bool, dry_run: bool) -> Result<()> {
        info!("Uninstalling JDK {version_spec}");
        let reporter = StatusReporter::new(self.no_progress);

        // Resolve JDKs to uninstall
        let jdks_to_remove = self.resolve_jdks_to_uninstall(version_spec)?;

        if jdks_to_remove.is_empty() {
            return Err(KopiError::JdkNotInstalled {
                jdk_spec: version_spec.to_string(),
                version: None,
                distribution: None,
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            });
        }

        // Handle multiple matches
        let jdk = if jdks_to_remove.len() > 1 {
            return Err(format_multiple_jdk_matches_error(
                version_spec,
                &jdks_to_remove,
            ));
        } else {
            jdks_to_remove.into_iter().next().unwrap()
        };
        let jdk_size = self.repository.get_jdk_size(&jdk.path)?;

        if dry_run {
            reporter.step(&format!(
                "Would remove {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(jdk_size)
            ));
            return Ok(());
        }

        let controller = LockController::with_default_inspector(
            self.config.kopi_home().to_path_buf(),
            &self.config.locking,
        );
        let scope_resolver = InstalledScopeResolver::new(self.repository);
        let lock_scope = scope_resolver.resolve(&jdk)?;
        let scope_label = lock_scope.label();

        reporter.step(&format!("Acquiring uninstall lock for {scope_label}"));
        let acquisition = controller.acquire_with_status_sink(lock_scope.clone(), &reporter)?;
        let uninstall_lock_guard = ScopedPackageLockGuard::new(&controller, acquisition);
        let backend_label = match uninstall_lock_guard.backend() {
            LockBackend::Advisory => "advisory",
            LockBackend::Fallback => "fallback",
        };
        info!("Uninstall lock acquired for {scope_label} using {backend_label} backend");
        reporter.step(&format!("Using {backend_label} backend for {scope_label}"));

        // Perform safety checks
        safety::perform_safety_checks(self.config, self.repository, &jdk, force)?;

        // Remove with progress
        match self.remove_jdk_with_progress(&jdk, jdk_size) {
            Ok(()) => {
                uninstall_lock_guard.release()?;
                reporter.success(&format!(
                    "Successfully uninstalled {}@{}",
                    jdk.distribution, jdk.version
                ));
                reporter.step(&format!("Freed {} of disk space", format_size(jdk_size)));
                Ok(())
            }
            Err(e) => {
                warn!("Uninstall failed: {e}");
                Err(e)
            }
        }
    }

    pub fn resolve_jdks_to_uninstall(&self, version_spec: &str) -> Result<Vec<InstalledJdk>> {
        debug!("Resolving JDKs to uninstall for spec: {version_spec}");

        // Use VersionRequest parser to handle JavaFX suffix and other special cases
        use crate::version::VersionRequest;
        let version_request = VersionRequest::from_str(version_spec)?;

        // Use repository's find_matching_jdks which handles JavaFX properly
        self.repository.find_matching_jdks(&version_request)
    }

    fn remove_jdk_with_progress(&self, jdk: &InstalledJdk, size: u64) -> Result<()> {
        info!("Removing JDK at {}", jdk.path.display());

        // Check for files in use before removal
        let files_in_use = platform::file_ops::check_files_in_use(&jdk.path)?;
        if !files_in_use.is_empty() {
            warn!("Files may be in use:");
            for file in &files_in_use {
                warn!("  {file}");
            }
            // Continue with removal but warn user
        }

        // Remove metadata file before atomic removal
        if let Some(parent) = jdk.path.parent()
            && let Some(jdk_dir_name) = jdk.path.file_name().and_then(|n| n.to_str())
        {
            let meta_file = parent.join(format!("{jdk_dir_name}.meta.json"));
            if meta_file.exists()
                && let Err(e) = std::fs::remove_file(&meta_file)
            {
                debug!(
                    "Failed to remove metadata file {}: {}",
                    meta_file.display(),
                    e
                );
                // Don't fail the operation if metadata removal fails
            }
        }

        // Create progress bar for large removals (> 100MB)
        let pb = if size > 100 * 1024 * 1024 {
            let mut progress_reporter = ProgressReporter::new(self.no_progress);
            let pb = progress_reporter
                .create_jdk_removal_spinner(&jdk.path.display().to_string(), &format_size(size));
            pb.enable_steady_tick(Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        // Prepare platform-specific removal
        platform::file_ops::prepare_for_removal(&jdk.path)?;

        // Atomic removal with rollback capability
        let temp_path = self.prepare_atomic_removal(&jdk.path)?;

        match self.finalize_removal(&temp_path) {
            Ok(()) => {
                // Platform-specific cleanup
                if let Err(e) = platform::file_ops::post_removal_cleanup(&jdk.path) {
                    debug!("Post-removal cleanup failed: {e}");
                }

                if let Some(pb) = pb {
                    pb.finish_and_clear();
                }
                Ok(())
            }
            Err(e) => {
                // Rollback on failure
                if let Err(rollback_err) = self.rollback_removal(&jdk.path, &temp_path) {
                    debug!("Failed to rollback removal: {rollback_err}");
                }
                if let Some(pb) = pb {
                    pb.finish_and_clear();
                }
                Err(e)
            }
        }
    }

    fn prepare_atomic_removal(&self, jdk_path: &PathBuf) -> Result<PathBuf> {
        let parent = jdk_path.parent().ok_or_else(|| {
            KopiError::SystemError("JDK path has no parent directory".to_string())
        })?;

        let temp_name = format!(
            ".{}.removing",
            jdk_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        );
        let temp_path = parent.join(temp_name);

        // Rename to temp location
        std::fs::rename(jdk_path, &temp_path)?;

        Ok(temp_path)
    }

    fn finalize_removal(&self, temp_path: &Path) -> Result<()> {
        // Use repository to ensure security checks
        self.repository.remove_jdk(temp_path)
    }

    fn rollback_removal(&self, original_path: &PathBuf, temp_path: &PathBuf) -> Result<()> {
        std::fs::rename(temp_path, original_path)?;
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
    use crate::version::Version;
    use std::fs;
    use std::str::FromStr;
    use tempfile::TempDir;

    struct TestSetup {
        _temp_dir: TempDir,
        config: KopiConfig,
    }

    impl TestSetup {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

            // Create jdks directory
            fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();

            TestSetup {
                _temp_dir: temp_dir,
                config,
            }
        }

        fn create_mock_jdk(&self, distribution: &str, version: &str) -> PathBuf {
            let jdk_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{distribution}-{version}"));
            fs::create_dir_all(&jdk_path).unwrap();

            // Create some mock files
            fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();
            let bin_dir = install::bin_directory(&jdk_path);
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho mock java").unwrap();

            jdk_path
        }
    }

    #[test]
    fn test_resolve_jdks_by_version() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, false);

        // Create test JDKs
        setup.create_mock_jdk("temurin", "21.0.5+11");
        setup.create_mock_jdk("temurin", "17.0.9+9");
        setup.create_mock_jdk("corretto", "21.0.1");

        // Test exact version match
        let matches = handler.resolve_jdks_to_uninstall("21").unwrap();
        assert_eq!(matches.len(), 2);

        // Test distribution@version
        let matches = handler.resolve_jdks_to_uninstall("temurin@21").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "temurin");
        assert_eq!(matches[0].version.to_string(), "21.0.5+11");

        // Test non-existent version
        let matches = handler.resolve_jdks_to_uninstall("11").unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_atomic_removal() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, false);

        let jdk_path = setup.create_mock_jdk("temurin", "21.0.5+11");
        assert!(jdk_path.exists());

        // Prepare atomic removal
        let temp_path = handler.prepare_atomic_removal(&jdk_path).unwrap();
        assert!(!jdk_path.exists());
        assert!(temp_path.exists());
        assert!(
            temp_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(".")
        );

        // Finalize removal
        handler.finalize_removal(&temp_path).unwrap();
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_rollback_removal() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, false);

        let jdk_path = setup.create_mock_jdk("temurin", "21.0.5+11");
        let original_exists = jdk_path.exists();

        // Prepare atomic removal
        let temp_path = handler.prepare_atomic_removal(&jdk_path).unwrap();
        assert!(!jdk_path.exists());

        // Rollback
        handler.rollback_removal(&jdk_path, &temp_path).unwrap();
        assert_eq!(jdk_path.exists(), original_exists);
        assert!(!temp_path.exists());
    }

    #[test]
    fn uninstall_releases_lock_on_success() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, true);

        let jdk_path = setup.create_mock_jdk("temurin", "21.0.5+11");

        handler
            .uninstall_jdk("temurin@21.0.5+11", false, false)
            .expect("uninstall should succeed");
        assert!(!jdk_path.exists());

        let controller = LockController::with_default_inspector(
            setup.config.kopi_home().to_path_buf(),
            &setup.config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.5+11").unwrap(),
            jdk_path.clone(),
            false,
        );
        let scope = resolver.resolve(&installed).unwrap();
        let reacquired = controller.try_acquire(scope).unwrap();
        assert!(reacquired.is_some());
        if let Some(handle) = reacquired {
            controller.release(handle).unwrap();
        }
    }

    #[test]
    fn uninstall_errors_when_lock_times_out() {
        let mut setup = TestSetup::new();
        setup
            .config
            .locking
            .set_timeout_value(LockTimeoutValue::from_secs(0));

        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, true);
        let jdk_path = setup.create_mock_jdk("temurin", "21.0.5+11");

        let controller = LockController::with_default_inspector(
            setup.config.kopi_home().to_path_buf(),
            &setup.config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.5+11").unwrap(),
            jdk_path.clone(),
            false,
        );
        let scope = resolver.resolve(&installed).unwrap();
        let acquisition = controller.acquire(scope).unwrap();
        let guard = ScopedPackageLockGuard::new(&controller, acquisition);

        let result = handler.uninstall_jdk("temurin@21.0.5+11", false, false);
        assert!(matches!(result, Err(KopiError::LockingTimeout { .. })));
        assert!(jdk_path.exists());

        guard.release().unwrap();
    }

    #[test]
    fn uninstall_releases_lock_on_failure() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository, true);

        let jdk_path = setup.create_mock_jdk("temurin", "21.0.5+11");
        let removing_path = jdk_path.parent().unwrap().join(format!(
            ".{}.removing",
            jdk_path.file_name().unwrap().to_str().unwrap()
        ));
        fs::create_dir_all(&removing_path).unwrap();
        fs::write(removing_path.join("marker"), "reserved").unwrap();

        let result = handler.uninstall_jdk("temurin@21.0.5+11", false, false);
        let err = result.expect_err("expected uninstall failure");
        assert!(matches!(err, KopiError::Io(_)), "unexpected error: {err:?}");
        assert!(jdk_path.exists());

        let controller = LockController::with_default_inspector(
            setup.config.kopi_home().to_path_buf(),
            &setup.config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.5+11").unwrap(),
            jdk_path,
            false,
        );
        let scope = resolver.resolve(&installed).unwrap();
        let reacquired = controller.try_acquire(scope).unwrap();
        assert!(reacquired.is_some());
        if let Some(handle) = reacquired {
            controller.release(handle).unwrap();
        }
    }
}
