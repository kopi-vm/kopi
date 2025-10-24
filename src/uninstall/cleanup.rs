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

use crate::error::{KopiError, Result};
use crate::locking::{InstalledScopeResolver, LockBackend, LockController, ScopedPackageLockGuard};
use crate::paths::install;
use crate::platform;
use crate::storage::{JdkLister, JdkRepository};
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

/// Handles cleanup of failed uninstall operations
pub struct UninstallCleanup<'a> {
    repository: &'a JdkRepository<'a>,
}

impl<'a> UninstallCleanup<'a> {
    pub fn new(repository: &'a JdkRepository<'a>) -> Self {
        Self { repository }
    }

    /// Detect and cleanup partial removals
    pub fn detect_and_cleanup_partial_removals(&self) -> Result<Vec<CleanupAction>> {
        info!("Scanning for partial removals");

        let mut cleanup_actions = Vec::new();
        let jdks_dir = self.repository.jdks_dir()?;

        if !jdks_dir.exists() {
            return Ok(cleanup_actions);
        }

        // Look for temporary removal directories
        let temp_dirs = self.find_temp_removal_dirs(&jdks_dir)?;
        for temp_dir in temp_dirs {
            cleanup_actions.push(CleanupAction::CleanupTempDir(temp_dir));
        }

        // Look for partially removed JDKs
        let partial_removals = self.find_partial_removals(&jdks_dir)?;
        for partial in partial_removals {
            cleanup_actions.push(CleanupAction::CompleteRemoval(partial));
        }

        // Look for orphaned metadata files
        let orphaned_metadata = self.find_orphaned_metadata(&jdks_dir)?;
        for metadata in orphaned_metadata {
            cleanup_actions.push(CleanupAction::CleanupOrphanedMetadata(metadata));
        }

        Ok(cleanup_actions)
    }

    /// Execute cleanup actions
    pub fn execute_cleanup(
        &self,
        actions: Vec<CleanupAction>,
        force: bool,
    ) -> Result<CleanupResult> {
        let mut result = CleanupResult::default();
        let config = self.repository.config();
        let controller = LockController::with_default_inspector(
            config.kopi_home().to_path_buf(),
            &config.locking,
        );
        let resolver = InstalledScopeResolver::new(self.repository);

        for action in actions {
            match self.execute_cleanup_action(action, force, &controller, &resolver) {
                Ok(success_msg) => {
                    result.successes.push(success_msg);
                }
                Err(e) => {
                    result.failures.push(e.to_string());
                }
            }
        }

        Ok(result)
    }

    /// Force cleanup of stubborn JDKs
    pub fn force_cleanup_jdk(&self, jdk_path: &Path) -> Result<()> {
        info!("Performing force cleanup of {}", jdk_path.display());

        // Try platform-specific preparation
        if let Err(e) = platform::file_ops::prepare_for_removal(jdk_path) {
            warn!("Platform preparation failed: {e}");
        }

        // Try multiple removal strategies
        let strategies = [
            Self::strategy_standard_removal,
            Self::strategy_recursive_chmod,
            Self::strategy_individual_file_removal,
        ];

        for (i, strategy) in strategies.iter().enumerate() {
            debug!("Trying removal strategy {}", i + 1);
            match strategy(jdk_path) {
                Ok(()) => {
                    info!("Force cleanup successful with strategy {}", i + 1);
                    return Ok(());
                }
                Err(e) => {
                    debug!("Strategy {} failed: {}", i + 1, e);
                }
            }
        }

        Err(KopiError::SystemError(format!(
            "All force cleanup strategies failed for {}",
            jdk_path.display()
        )))
    }

    fn find_temp_removal_dirs(&self, jdks_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut temp_dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(jdks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with('.')
                    && name.ends_with(".removing")
                {
                    temp_dirs.push(path);
                }
            }
        }

        Ok(temp_dirs)
    }

    fn find_partial_removals(&self, jdks_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut partial_removals = Vec::new();

        if let Ok(entries) = fs::read_dir(jdks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip temporary removal directories and hidden directories
                    if let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && name.starts_with('.')
                    {
                        continue; // Skip temporary directories and hidden files
                    }

                    if self.is_partial_removal(&path)? {
                        partial_removals.push(path);
                    }
                }
            }
        }

        Ok(partial_removals)
    }

    fn find_orphaned_metadata(&self, jdks_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut orphaned_metadata = Vec::new();

        if let Ok(entries) = fs::read_dir(jdks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.ends_with(".meta.json")
                {
                    // Check if corresponding JDK directory exists
                    let jdk_name = name.replace(".meta.json", "");
                    let jdk_path = jdks_dir.join(&jdk_name);
                    if !jdk_path.exists() {
                        orphaned_metadata.push(path);
                    }
                }
            }
        }

        Ok(orphaned_metadata)
    }

    fn is_partial_removal(&self, path: &Path) -> Result<bool> {
        // Check if directory exists but is missing essential files
        if !path.is_dir() {
            return Ok(false);
        }

        let has_release_file = path.join("release").exists();
        let bin_dir = install::bin_directory(path);
        let has_bin_dir = bin_dir.exists();
        let has_java_executable =
            bin_dir.join("java").exists() || bin_dir.join("java.exe").exists();

        // If it's missing essential files, it might be a partial removal
        Ok(!has_release_file || !has_bin_dir || !has_java_executable)
    }

    fn execute_cleanup_action(
        &self,
        action: CleanupAction,
        force: bool,
        controller: &LockController,
        resolver: &InstalledScopeResolver,
    ) -> Result<String> {
        match action {
            CleanupAction::CleanupTempDir(path) => {
                info!("Cleaning up temporary directory: {}", path.display());
                if force {
                    // Force cleanup intentionally bypasses locking for emergency remediation.
                    self.force_cleanup_jdk(&path)?;
                    Ok(format!(
                        "Cleaned up temporary directory: {}",
                        path.display()
                    ))
                } else {
                    self.run_with_lock(&path, controller, resolver, || {
                        fs::remove_dir_all(&path)?;
                        Ok(format!(
                            "Cleaned up temporary directory: {}",
                            path.display()
                        ))
                    })
                }
            }
            CleanupAction::CompleteRemoval(path) => {
                info!("Completing partial removal: {}", path.display());
                if force {
                    // Force cleanup intentionally bypasses locking for emergency remediation.
                    self.force_cleanup_jdk(&path)?;
                    Ok(format!("Completed removal of: {}", path.display()))
                } else {
                    self.run_with_lock(&path, controller, resolver, || {
                        fs::remove_dir_all(&path)?;
                        Ok(format!("Completed removal of: {}", path.display()))
                    })
                }
            }
            CleanupAction::CleanupOrphanedMetadata(path) => {
                info!("Cleaning up orphaned metadata: {}", path.display());
                self.run_with_lock(&path, controller, resolver, || {
                    fs::remove_file(&path)?;
                    Ok(format!("Cleaned up orphaned metadata: {}", path.display()))
                })
            }
        }
    }

    fn run_with_lock<F>(
        &self,
        path: &Path,
        controller: &LockController,
        resolver: &InstalledScopeResolver,
        mut action: F,
    ) -> Result<String>
    where
        F: FnMut() -> Result<String>,
    {
        match self.cleanup_lock_scope(path, resolver)? {
            Some(scope) => {
                let scope_label = scope.label().to_string();
                info!("Acquiring cleanup lock for {scope_label}");
                let acquisition = controller.acquire(scope.clone())?;
                let guard = ScopedPackageLockGuard::new(controller, acquisition);
                let backend_label = match guard.backend() {
                    LockBackend::Advisory => "advisory",
                    LockBackend::Fallback => "fallback",
                };
                info!("Cleanup lock acquired for {scope_label} using {backend_label} backend");

                match action() {
                    Ok(message) => {
                        guard.release()?;
                        Ok(message)
                    }
                    Err(err) => {
                        // Guard releases on drop; propagate original error.
                        Err(err)
                    }
                }
            }
            None => {
                debug!(
                    "Skipping uninstall lock for cleanup target {}; scope could not be resolved",
                    path.display()
                );
                action()
            }
        }
    }

    fn cleanup_lock_scope(
        &self,
        path: &Path,
        resolver: &InstalledScopeResolver,
    ) -> Result<Option<crate::locking::LockScope>> {
        let jdks_dir = self.repository.jdks_dir()?;
        if !path.starts_with(&jdks_dir) {
            return Ok(None);
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return Ok(None),
        };

        let slug = if file_name.starts_with('.') && file_name.ends_with(".removing") {
            file_name
                .trim_start_matches('.')
                .trim_end_matches(".removing")
                .to_string()
        } else if file_name.ends_with(".meta.json") {
            file_name.trim_end_matches(".meta.json").to_string()
        } else if file_name.starts_with('.') {
            return Ok(None);
        } else {
            file_name.to_string()
        };

        if slug.is_empty() {
            return Ok(None);
        }

        let normalized_path = jdks_dir.join(&slug);
        if let Some(mut installed) = JdkLister::parse_jdk_dir_name(&normalized_path) {
            installed.path = normalized_path;
            match resolver.resolve(&installed) {
                Ok(scope) => Ok(Some(scope)),
                Err(err) => {
                    warn!(
                        "Failed to resolve lock scope for cleanup target {}: {}",
                        path.display(),
                        err
                    );
                    Ok(None)
                }
            }
        } else {
            debug!(
                "Unable to derive installation coordinate from cleanup target {}",
                path.display()
            );
            Ok(None)
        }
    }

    fn strategy_standard_removal(path: &Path) -> Result<()> {
        fs::remove_dir_all(path)?;
        Ok(())
    }

    fn strategy_recursive_chmod(path: &Path) -> Result<()> {
        // Make all files writable first
        walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .try_for_each(|entry| platform::file_ops::make_writable(entry.path()))?;

        fs::remove_dir_all(path)?;
        Ok(())
    }

    fn strategy_individual_file_removal(path: &Path) -> Result<()> {
        // Remove files one by one, then directories
        for entry in walkdir::WalkDir::new(path)
            .contents_first(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let _ = fs::remove_file(entry_path); // Ignore individual failures
            } else if entry_path.is_dir() && entry_path != path {
                let _ = fs::remove_dir(entry_path); // Ignore individual failures
            }
        }

        // Finally remove the root directory
        fs::remove_dir(path)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum CleanupAction {
    CleanupTempDir(PathBuf),
    CompleteRemoval(PathBuf),
    CleanupOrphanedMetadata(PathBuf),
}

#[derive(Debug, Default)]
pub struct CleanupResult {
    pub successes: Vec<String>,
    pub failures: Vec<String>,
}

impl CleanupResult {
    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Cleanup complete: {} successes, {} failures",
            self.successes.len(),
            self.failures.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::locking::{
        InstalledScopeResolver, LockController, LockTimeoutValue, ScopedPackageLockGuard,
    };
    use crate::test::fixtures::create_test_jdk_with_path;
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

        fn create_temp_removal_dir(&self, name: &str) -> PathBuf {
            let temp_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!(".{name}.removing"));
            fs::create_dir_all(&temp_path).unwrap();
            temp_path
        }

        fn create_partial_jdk(&self, name: &str) -> PathBuf {
            let jdk_path = self.config.jdks_dir().unwrap().join(name);
            fs::create_dir_all(&jdk_path).unwrap();

            // Create incomplete JDK structure (missing bin/java)
            fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();
            let bin_dir = install::bin_directory(&jdk_path);
            fs::create_dir_all(&bin_dir).unwrap();
            // Note: NOT creating bin/java to simulate partial removal

            jdk_path
        }

        fn create_orphaned_metadata(&self, name: &str) -> PathBuf {
            let metadata_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{name}.meta.json"));
            fs::write(
                &metadata_path,
                r#"{"version": "21.0.1", "distribution": "temurin"}"#,
            )
            .unwrap();
            metadata_path
        }
    }

    #[test]
    fn test_detect_temp_cleanup_dirs() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        // Create a temporary removal directory
        let temp_path = setup.create_temp_removal_dir("temurin-21.0.1");

        let actions = cleanup.detect_and_cleanup_partial_removals().unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CleanupAction::CleanupTempDir(path) => {
                assert_eq!(path, &temp_path);
            }
            _ => panic!("Expected CleanupTempDir action"),
        }
    }

    #[test]
    fn test_detect_partial_removals() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        // Create a partial JDK
        let partial_path = setup.create_partial_jdk("temurin-21.0.1");

        let actions = cleanup.detect_and_cleanup_partial_removals().unwrap();

        assert!(actions.iter().any(
            |action| matches!(action, CleanupAction::CompleteRemoval(path) if path == &partial_path)
        ));
    }

    #[test]
    fn test_detect_orphaned_metadata() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        // Create orphaned metadata
        let metadata_path = setup.create_orphaned_metadata("temurin-21.0.1");

        let actions = cleanup.detect_and_cleanup_partial_removals().unwrap();

        assert!(actions.iter().any(|action| matches!(action, CleanupAction::CleanupOrphanedMetadata(path) if path == &metadata_path)));
    }

    #[test]
    fn test_execute_cleanup_actions() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        // Create test scenarios
        let temp_path = setup.create_temp_removal_dir("temurin-21.0.1");
        let metadata_path = setup.create_orphaned_metadata("corretto-17.0.1");

        let actions = vec![
            CleanupAction::CleanupTempDir(temp_path.clone()),
            CleanupAction::CleanupOrphanedMetadata(metadata_path.clone()),
        ];

        let result = cleanup.execute_cleanup(actions, false).unwrap();

        assert_eq!(result.successes.len(), 2);
        assert_eq!(result.failures.len(), 0);
        assert!(!temp_path.exists());
        assert!(!metadata_path.exists());
    }

    #[test]
    fn cleanup_respects_lock_contention() {
        let mut setup = TestSetup::new();
        setup
            .config
            .locking
            .set_timeout_value(LockTimeoutValue::from_secs(0));
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        let temp_path = setup.create_temp_removal_dir("temurin-21.0.1");
        let slug = "temurin-21.0.1";
        let install_path = setup.config.jdks_dir().unwrap().join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let locked_jdk =
            create_test_jdk_with_path("temurin", "21.0.1", install_path.to_str().unwrap());
        let controller = LockController::with_default_inspector(
            setup.config.kopi_home().to_path_buf(),
            &setup.config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let scope = resolver.resolve(&locked_jdk).unwrap();
        let guard = ScopedPackageLockGuard::new(&controller, controller.acquire(scope).unwrap());

        let actions = vec![CleanupAction::CleanupTempDir(temp_path.clone())];
        let result = cleanup.execute_cleanup(actions, false).unwrap();

        assert!(
            result.failures.len() == 1,
            "cleanup should record failure when lock acquisition times out"
        );
        assert!(
            temp_path.exists(),
            "temporary directory should remain when cleanup fails to acquire lock"
        );

        guard.release().unwrap();
        if temp_path.exists() {
            fs::remove_dir_all(&temp_path).unwrap();
        }
    }

    #[test]
    fn cleanup_force_bypasses_locking() {
        let mut setup = TestSetup::new();
        setup
            .config
            .locking
            .set_timeout_value(LockTimeoutValue::from_secs(0));
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        let slug = "temurin-21.0.2";
        let install_path = setup.config.jdks_dir().unwrap().join(slug);
        fs::create_dir_all(&install_path).unwrap();
        fs::write(install_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();

        let locked_jdk =
            create_test_jdk_with_path("temurin", "21.0.2", install_path.to_str().unwrap());
        let controller = LockController::with_default_inspector(
            setup.config.kopi_home().to_path_buf(),
            &setup.config.locking,
        );
        let resolver = InstalledScopeResolver::new(&repository);
        let scope = resolver.resolve(&locked_jdk).unwrap();
        let guard = ScopedPackageLockGuard::new(&controller, controller.acquire(scope).unwrap());

        let actions = vec![CleanupAction::CompleteRemoval(install_path.clone())];
        let result = cleanup.execute_cleanup(actions, true).unwrap();
        assert!(
            result.failures.is_empty(),
            "force cleanup should not fail when a lock is held"
        );
        assert!(
            result.successes.len() == 1,
            "force cleanup should report the completed removal"
        );
        assert!(
            !install_path.exists(),
            "force cleanup should remove the directory even under lock contention"
        );

        guard.release().unwrap();
    }
}
