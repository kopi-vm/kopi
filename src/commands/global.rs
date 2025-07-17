use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::installation::auto::{AutoInstaller, InstallationResult};
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use log::{debug, info};
use std::path::PathBuf;
use std::str::FromStr;

pub struct GlobalCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> GlobalCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, version_spec: &str) -> Result<()> {
        info!("Setting global JDK version to {version_spec}");

        // Use configuration

        // Parse version specification using lenient parsing
        let version_request = VersionRequest::from_str(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Create storage repository
        let repository = JdkRepository::new(self.config);

        // Check if matching JDK is installed
        let mut matching_jdks = repository.find_matching_jdks(&version_request)?;

        if matching_jdks.is_empty() {
            // Auto-installation for global command
            info!("JDK {} is not installed.", version_request.version_pattern);

            let auto_installer = AutoInstaller::new(self.config);

            match auto_installer.prompt_and_install(&version_request)? {
                InstallationResult::Installed => {
                    info!(
                        "JDK {} installed successfully",
                        version_request.version_pattern
                    );
                    // Re-fetch matching JDKs after installation
                    matching_jdks = repository.find_matching_jdks(&version_request)?;
                }
                InstallationResult::UserDeclined => {
                    return Err(KopiError::JdkNotInstalled {
                        jdk_spec: version_request.version_pattern.clone(),
                        version: Some(version_request.version_pattern.clone()),
                        distribution: version_request.distribution.clone(),
                        auto_install_enabled: true,
                        auto_install_failed: None,
                        user_declined: true,
                        install_in_progress: false,
                    });
                }
                InstallationResult::AutoInstallDisabled => {
                    return Err(KopiError::JdkNotInstalled {
                        jdk_spec: version_request.version_pattern.clone(),
                        version: Some(version_request.version_pattern.clone()),
                        distribution: version_request.distribution.clone(),
                        auto_install_enabled: false,
                        auto_install_failed: None,
                        user_declined: false,
                        install_in_progress: false,
                    });
                }
            }
        }

        // Get the last (latest) matching JDK
        let selected_jdk = matching_jdks
            .last()
            .ok_or_else(|| KopiError::JdkNotInstalled {
                jdk_spec: version_request.version_pattern.clone(),
                version: Some(version_request.version_pattern.clone()),
                distribution: version_request.distribution.clone(),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            })?;

        // Write version file using the selected JDK
        let version_file = self.global_version_path(self.config)?;
        selected_jdk.write_to(&version_file)?;

        println!(
            "Global JDK version set to {}@{}",
            selected_jdk.distribution, selected_jdk.version
        );

        Ok(())
    }

    fn global_version_path(&self, config: &crate::config::KopiConfig) -> Result<PathBuf> {
        Ok(config.kopi_home().join("version"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_command_creation() {
        let command = GlobalCommand::new().unwrap();
        assert!(!std::ptr::addr_of!(command).is_null());
    }

    #[test]
    fn test_global_version_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let command = GlobalCommand::new().unwrap();

        let version_path = command.global_version_path(command.config).unwrap();
        assert_eq!(version_path, temp_dir.path().join("version"));
    }
}
