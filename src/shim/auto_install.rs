use crate::commands::install::InstallCommand;
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::jdk::VersionRequest;
use log::{debug, info, warn};
use std::fs::File;
use std::io::{Write, stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const LOCK_FILE_NAME: &str = ".kopi-install.lock";

pub struct AutoInstaller<'a> {
    config: &'a KopiConfig,
    enabled: bool,
    enable_prompts: bool,
}

impl<'a> AutoInstaller<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        // Configuration values already include environment variable overrides
        let enabled = config.auto_install.enabled;
        let enable_prompts = config.auto_install.prompt && enabled;

        Self {
            config,
            enabled,
            enable_prompts,
        }
    }

    /// Check if auto-install is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Attempt to auto-install a missing JDK
    pub fn auto_install(&self, version_request: &VersionRequest) -> Result<PathBuf> {
        if !self.is_enabled() {
            return Err(KopiError::JdkNotInstalled(format!(
                "{}@{}",
                version_request
                    .distribution
                    .as_ref()
                    .unwrap_or(&"temurin".to_string()),
                version_request.version_pattern
            )));
        }

        let version_spec = if let Some(dist) = &version_request.distribution {
            format!("{}@{}", dist, version_request.version_pattern)
        } else {
            version_request.version_pattern.clone()
        };

        info!(
            "Auto-install: JDK {} not found, attempting to install",
            version_spec
        );

        // Check for concurrent installations
        if let Err(e) = self.acquire_install_lock(&version_spec) {
            warn!("Auto-install: Failed to acquire lock: {}", e);
            return Err(KopiError::JdkNotInstalled(format!(
                "{} (auto-install in progress by another process)",
                version_spec
            )));
        }

        // Prompt user if configured
        if self.enable_prompts && !self.prompt_user(&version_spec)? {
            self.release_install_lock(&version_spec)?;
            return Err(KopiError::JdkNotInstalled(format!(
                "{} (user declined auto-install)",
                version_spec
            )));
        }

        // Perform installation with timeout
        let result = self.install_with_timeout(&version_spec);

        // Always release lock
        let _ = self.release_install_lock(&version_spec);

        match result {
            Ok(path) => {
                info!("Auto-install: Successfully installed {}", version_spec);
                Ok(path)
            }
            Err(e) => {
                warn!("Auto-install: Failed to install {}: {}", version_spec, e);
                Err(e)
            }
        }
    }

    fn prompt_user(&self, version_spec: &str) -> Result<bool> {
        print!(
            "\nJDK {} is not installed. Would you like to install it now? [Y/n] ",
            version_spec
        );
        stdout().flush()?;

        // Read user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let response = input.trim().to_lowercase();
        Ok(response.is_empty() || response == "y" || response == "yes")
    }

    fn install_with_timeout(&self, version_spec: &str) -> Result<PathBuf> {
        // Configuration already includes environment variable overrides
        let timeout_secs = self.config.auto_install.timeout_secs;

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let install_complete = Arc::new(AtomicBool::new(false));
        let install_complete_clone = install_complete.clone();

        // Create install command
        let install_cmd = InstallCommand::new()?;

        // Spawn installation in a separate thread
        let version_spec_clone = version_spec.to_string();
        let handle = std::thread::spawn(move || {
            let result = install_cmd.execute(
                &version_spec_clone,
                false, // force
                false, // dry_run
                false, // no_progress
                Some(timeout_secs),
                false, // javafx_bundled
            );
            install_complete_clone.store(true, Ordering::SeqCst);
            result
        });

        // Monitor timeout
        loop {
            if install_complete.load(Ordering::SeqCst) {
                break;
            }

            if start.elapsed() > timeout {
                warn!(
                    "Auto-install: Installation timed out after {} seconds",
                    timeout_secs
                );
                // Note: We can't reliably cancel the installation thread in Rust
                // The thread will continue but we'll return an error
                return Err(KopiError::SystemError(format!(
                    "Auto-installation timed out after {} seconds",
                    timeout_secs
                )));
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        // Wait for installation to complete
        match handle.join() {
            Ok(Ok(())) => {
                // Installation successful, find the JDK path
                let repository = crate::storage::JdkRepository::new(self.config);

                // Parse version request to get distribution
                let parser = crate::version::parser::VersionParser::new(self.config);
                let parsed_request = parser.parse(version_spec)?;
                let distribution = parsed_request
                    .distribution
                    .unwrap_or(crate::models::jdk::Distribution::Temurin);

                // Find the installed JDK
                let installed_jdks = repository.list_installed_jdks()?;

                // Extract version string from parsed request
                let version_str = parsed_request
                    .version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "".to_string());

                for jdk in installed_jdks {
                    if jdk.distribution.to_lowercase() == distribution.id()
                        && (version_str.is_empty() || jdk.version.starts_with(&version_str))
                    {
                        return Ok(jdk.path);
                    }
                }

                Err(KopiError::SystemError(
                    "JDK installed but path not found".to_string(),
                ))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(KopiError::SystemError(
                "Installation thread panicked".to_string(),
            )),
        }
    }

    pub fn acquire_install_lock(&self, version_spec: &str) -> Result<()> {
        let lock_dir = self.config.cache_dir()?;
        let safe_name = version_spec.replace('@', "-").replace('/', "-");
        let lock_file = lock_dir.join(format!("{}-{}", safe_name, LOCK_FILE_NAME));

        debug!(
            "Auto-install: Attempting to acquire lock at {:?}",
            lock_file
        );

        // Try to create lock file exclusively
        match File::create_new(&lock_file) {
            Ok(mut file) => {
                // Write PID to lock file
                let pid = std::process::id();
                file.write_all(pid.to_string().as_bytes())?;
                debug!("Auto-install: Lock acquired by PID {}", pid);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Check if the process that created the lock is still running
                if let Ok(contents) = std::fs::read_to_string(&lock_file) {
                    if let Ok(pid) = contents.trim().parse::<u32>() {
                        debug!("Auto-install: Lock held by PID {}", pid);
                        // For now, we'll assume the process is still running
                        // In a production system, we'd check if the PID is actually running
                    }
                }
                Err(KopiError::SystemError(
                    "Another installation is in progress".to_string(),
                ))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn release_install_lock(&self, version_spec: &str) -> Result<()> {
        let lock_dir = self.config.cache_dir()?;
        let safe_name = version_spec.replace('@', "-").replace('/', "-");
        let lock_file = lock_dir.join(format!("{}-{}", safe_name, LOCK_FILE_NAME));

        if lock_file.exists() {
            debug!("Auto-install: Releasing lock at {:?}", lock_file);
            std::fs::remove_file(&lock_file)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_config() -> (TempDir, KopiConfig) {
        // Clear any auto-install environment variables
        unsafe {
            std::env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            std::env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            std::env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        (temp_dir, config)
    }

    #[test]
    #[serial]
    fn test_auto_installer_enabled_by_default() {
        let (_temp_dir, config) = setup_test_config();

        // Clear environment variable
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }

        let installer = AutoInstaller::new(&config);
        assert!(installer.is_enabled());
    }

    #[test]
    #[serial]
    fn test_auto_installer_enabled_via_env() {
        let (temp_dir, _config) = setup_test_config();

        // Set environment variable before creating config
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "true");
        }

        // Create new config to pick up environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let installer = AutoInstaller::new(&config);
        assert!(installer.is_enabled());

        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }
    }

    #[test]
    #[serial]
    fn test_auto_installer_prompts_enabled_by_default() {
        let (temp_dir, _config) = setup_test_config();

        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "true");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
        }

        // Create new config to pick up environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let installer = AutoInstaller::new(&config);
        assert!(installer.enable_prompts);

        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }
    }

    #[test]
    #[serial]
    fn test_auto_installer_prompts_disabled_via_env() {
        let (temp_dir, _config) = setup_test_config();

        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "true");
            env::set_var("KOPI_AUTO_INSTALL__PROMPT", "false");
        }

        // Create new config to pick up environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let installer = AutoInstaller::new(&config);
        assert!(!installer.enable_prompts);

        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
        }
    }

    #[test]
    fn test_lock_file_creation() {
        let (_temp_dir, config) = setup_test_config();
        let installer = AutoInstaller::new(&config);

        let version_spec = "temurin@21";

        // First lock should succeed
        assert!(installer.acquire_install_lock(version_spec).is_ok());

        // Second lock should fail
        assert!(installer.acquire_install_lock(version_spec).is_err());

        // Release lock
        assert!(installer.release_install_lock(version_spec).is_ok());

        // Now lock should succeed again
        assert!(installer.acquire_install_lock(version_spec).is_ok());

        // Cleanup
        installer.release_install_lock(version_spec).unwrap();
    }

    #[test]
    fn test_lock_file_name_sanitization() {
        let (_temp_dir, config) = setup_test_config();
        let cache_dir = config.cache_dir().unwrap();
        let installer = AutoInstaller::new(&config);

        // Test with special characters
        let version_spec = "custom/jdk@21.0.1";
        assert!(installer.acquire_install_lock(version_spec).is_ok());

        // Check that lock file was created with sanitized name
        let expected_lock_file = cache_dir.join("custom-jdk-21.0.1-.kopi-install.lock");
        assert!(expected_lock_file.exists());

        // Cleanup
        installer.release_install_lock(version_spec).unwrap();
    }

    #[test]
    #[serial]
    fn test_auto_install_disabled_returns_error() {
        let (temp_dir, _config) = setup_test_config();

        // Explicitly disable auto-install
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "false");
        }

        // Create new config to pick up environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let installer = AutoInstaller::new(&config);
        assert!(!installer.is_enabled(), "Auto-install should be disabled");

        let version_request =
            VersionRequest::new("21".to_string()).with_distribution("temurin".to_string());

        let result = installer.auto_install(&version_request);
        assert!(matches!(result, Err(KopiError::JdkNotInstalled(_))));

        // Cleanup
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }
    }

    // Mock-based tests for installation logic would go here
    // Since we can't easily mock the InstallCommand in the current architecture,
    // these would be integration tests
}
