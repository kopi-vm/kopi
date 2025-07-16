use crate::error::{KopiError, Result};
use crate::platform;
use crate::storage::JdkRepository;
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

        for action in actions {
            match self.execute_cleanup_action(action, force) {
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

    /// Provide cleanup suggestions for common errors
    pub fn suggest_cleanup_actions(&self, error: &KopiError) -> Vec<String> {
        let mut suggestions = Vec::new();

        match error {
            KopiError::SystemError(msg) if msg.contains("permission") => {
                suggestions.push("Try running with administrator/root privileges".to_string());
                suggestions.push("Check file permissions in the JDK directory".to_string());
            }
            KopiError::SystemError(msg) if msg.contains("in use") => {
                suggestions.push("Close any applications using the JDK".to_string());
                suggestions.push("Restart your system and try again".to_string());
                suggestions.push("Use --force flag to attempt forced removal".to_string());
            }
            KopiError::SystemError(msg) if msg.contains("antivirus") => {
                suggestions.push("Temporarily disable real-time antivirus protection".to_string());
                suggestions.push("Add JDK directory to antivirus exclusions".to_string());
            }
            KopiError::Io(io_err) => match io_err.kind() {
                std::io::ErrorKind::NotFound => {
                    suggestions.push("JDK may have been partially removed".to_string());
                    suggestions.push("Run 'kopi doctor' to check for issues".to_string());
                }
                std::io::ErrorKind::PermissionDenied => {
                    suggestions.push("Check file permissions".to_string());
                    suggestions.push("Try running as administrator".to_string());
                }
                _ => {
                    suggestions.push("Check disk space and file system health".to_string());
                }
            },
            _ => {
                suggestions.push("Run 'kopi doctor' to diagnose issues".to_string());
            }
        }

        suggestions
    }

    /// Force cleanup of stubborn JDKs
    pub fn force_cleanup_jdk(&self, jdk_path: &Path) -> Result<()> {
        info!("Performing force cleanup of {}", jdk_path.display());

        // Try platform-specific preparation
        if let Err(e) = platform::uninstall::prepare_for_removal(jdk_path) {
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
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') && name.ends_with(".removing") {
                        temp_dirs.push(path);
                    }
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
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('.') {
                            continue; // Skip temporary directories and hidden files
                        }
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
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".meta.json") {
                        // Check if corresponding JDK directory exists
                        let jdk_name = name.replace(".meta.json", "");
                        let jdk_path = jdks_dir.join(&jdk_name);
                        if !jdk_path.exists() {
                            orphaned_metadata.push(path);
                        }
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
        let has_bin_dir = path.join("bin").exists();
        let has_java_executable =
            path.join("bin").join("java").exists() || path.join("bin").join("java.exe").exists();

        // If it's missing essential files, it might be a partial removal
        Ok(!has_release_file || !has_bin_dir || !has_java_executable)
    }

    fn execute_cleanup_action(&self, action: CleanupAction, force: bool) -> Result<String> {
        match action {
            CleanupAction::CleanupTempDir(path) => {
                info!("Cleaning up temporary directory: {}", path.display());
                if force {
                    self.force_cleanup_jdk(&path)?;
                } else {
                    fs::remove_dir_all(&path)?;
                }
                Ok(format!(
                    "Cleaned up temporary directory: {}",
                    path.display()
                ))
            }
            CleanupAction::CompleteRemoval(path) => {
                info!("Completing partial removal: {}", path.display());
                if force {
                    self.force_cleanup_jdk(&path)?;
                } else {
                    fs::remove_dir_all(&path)?;
                }
                Ok(format!("Completed removal of: {}", path.display()))
            }
            CleanupAction::CleanupOrphanedMetadata(path) => {
                info!("Cleaning up orphaned metadata: {}", path.display());
                fs::remove_file(&path)?;
                Ok(format!("Cleaned up orphaned metadata: {}", path.display()))
            }
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
            .try_for_each(|entry| {
                let mut perms = entry.metadata()?.permissions();
                perms.set_readonly(false);
                fs::set_permissions(entry.path(), perms)
            })?;

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
                .join(format!(".{}.removing", name));
            fs::create_dir_all(&temp_path).unwrap();
            temp_path
        }

        fn create_partial_jdk(&self, name: &str) -> PathBuf {
            let jdk_path = self.config.jdks_dir().unwrap().join(name);
            fs::create_dir_all(&jdk_path).unwrap();

            // Create incomplete JDK structure (missing bin/java)
            fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();
            fs::create_dir_all(jdk_path.join("bin")).unwrap();
            // Note: NOT creating bin/java to simulate partial removal

            jdk_path
        }

        fn create_orphaned_metadata(&self, name: &str) -> PathBuf {
            let metadata_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{}.meta.json", name));
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
    fn test_suggest_cleanup_actions() {
        let setup = TestSetup::new();
        let repository = JdkRepository::new(&setup.config);
        let cleanup = UninstallCleanup::new(&repository);

        // Test permission error suggestions
        let permission_error = KopiError::SystemError("permission denied".to_string());
        let suggestions = cleanup.suggest_cleanup_actions(&permission_error);

        assert!(suggestions.iter().any(|s| s.contains("administrator")));
        assert!(suggestions.iter().any(|s| s.contains("permission")));
    }
}
