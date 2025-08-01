use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution;
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
    repository: &'a JdkRepository<'a>,
}

impl<'a> UninstallHandler<'a> {
    pub fn new(repository: &'a JdkRepository<'a>) -> Self {
        Self { repository }
    }

    /// Perform cleanup operations for failed uninstalls
    pub fn recover_from_failures(&self, force: bool) -> Result<()> {
        let cleanup = UninstallCleanup::new(self.repository);

        let actions = cleanup.detect_and_cleanup_partial_removals()?;

        if actions.is_empty() {
            println!("No recovery actions needed.");
            return Ok(());
        }

        println!("Found {} recovery actions:", actions.len());
        for action in &actions {
            println!("  - {action:?}");
        }

        let result = cleanup.execute_cleanup(actions, force)?;

        if result.is_success() {
            println!("✓ Recovery completed successfully");
            for success in result.successes {
                println!("  ✓ {success}");
            }
        } else {
            println!("⚠ Recovery completed with errors");
            for success in result.successes {
                println!("  ✓ {success}");
            }
            for failure in result.failures {
                println!("  ✗ {failure}");
            }
        }

        Ok(())
    }

    /// Get cleanup suggestions for a failed uninstall
    pub fn get_cleanup_suggestions(&self, error: &KopiError) -> Vec<String> {
        let cleanup = UninstallCleanup::new(self.repository);
        cleanup.suggest_cleanup_actions(error)
    }

    pub fn uninstall_jdk(&self, version_spec: &str, dry_run: bool) -> Result<()> {
        info!("Uninstalling JDK {version_spec}");

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
            println!(
                "Would remove {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(jdk_size)
            );
            return Ok(());
        }

        // Perform safety checks
        safety::perform_safety_checks(&jdk.distribution, &jdk.version.to_string())?;

        // Remove with progress
        match self.remove_jdk_with_progress(&jdk, jdk_size) {
            Ok(()) => {
                println!(
                    "✓ Successfully uninstalled {}@{}",
                    jdk.distribution, jdk.version
                );
                println!("  Freed {} of disk space", format_size(jdk_size));
                Ok(())
            }
            Err(e) => {
                warn!("Uninstall failed: {e}");

                // Provide cleanup suggestions
                let suggestions = self.get_cleanup_suggestions(&e);
                if !suggestions.is_empty() {
                    println!("\nRecovery suggestions:");
                    for suggestion in suggestions {
                        println!("  • {suggestion}");
                    }
                }

                Err(e)
            }
        }
    }

    pub fn resolve_jdks_to_uninstall(&self, version_spec: &str) -> Result<Vec<InstalledJdk>> {
        debug!("Resolving JDKs to uninstall for spec: {version_spec}");

        // List all installed JDKs
        let installed_jdks = self.repository.list_installed_jdks()?;
        if installed_jdks.is_empty() {
            return Ok(Vec::new());
        }

        // Parse version specification
        let (distribution_str, version_str) = if version_spec.contains('@') {
            let parts: Vec<&str> = version_spec.split('@').collect();
            if parts.len() != 2 {
                return Err(KopiError::InvalidVersionFormat(version_spec.to_string()));
            }
            (Some(parts[0]), Some(parts[1]))
        } else {
            (None, Some(version_spec))
        };

        // Filter matching JDKs
        let matches: Vec<InstalledJdk> = installed_jdks
            .into_iter()
            .filter(|jdk| {
                // Check distribution match
                if let Some(dist_str) = distribution_str {
                    if let Ok(req_dist) = Distribution::from_str(dist_str) {
                        if let Ok(jdk_dist) = Distribution::from_str(&jdk.distribution) {
                            if req_dist != jdk_dist {
                                return false;
                            }
                        }
                    }
                }

                // Check version match
                if let Some(ver_str) = version_str {
                    return jdk.version.matches_pattern(ver_str);
                }

                // If no version specified but distribution matches, include it
                version_str.is_none()
            })
            .collect();

        debug!("Found {} matching JDKs", matches.len());
        Ok(matches)
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

        // Create progress bar for large removals (> 100MB)
        let pb = if size > 100 * 1024 * 1024 {
            let progress_reporter = ProgressReporter::new();
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
    use std::fs;
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
            fs::create_dir_all(jdk_path.join("bin")).unwrap();
            fs::write(jdk_path.join("bin/java"), "#!/bin/sh\necho mock java").unwrap();

            jdk_path
        }
    }

    #[test]
    fn test_resolve_jdks_by_version() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let handler = UninstallHandler::new(&repository);

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
        let handler = UninstallHandler::new(&repository);

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
        let handler = UninstallHandler::new(&repository);

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
}
