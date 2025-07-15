use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::installation::auto::{AutoInstaller, InstallationResult};
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use log::{debug, info};
use std::path::PathBuf;
use std::str::FromStr;

pub struct LocalCommand;

impl LocalCommand {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute(&self, version_spec: &str) -> Result<()> {
        info!("Setting local JDK version to {version_spec}");

        // Load configuration
        let config = new_kopi_config()?;

        // Parse version specification using lenient parsing
        let version_request = VersionRequest::from_str(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Create storage repository
        let repository = JdkRepository::new(&config);

        // Check if matching JDK is installed
        let mut matching_jdks = repository.find_matching_jdks(&version_request)?;

        if matching_jdks.is_empty() {
            // Auto-installation is optional for local command
            info!("JDK {} is not installed.", version_request.version_pattern);

            let auto_installer = AutoInstaller::new(&config);

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
                        version: None,
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
                        version: None,
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
                version: None,
                distribution: version_request.distribution.clone(),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            })?;

        // Write version file using the selected JDK
        let version_file = self.local_version_path()?;
        selected_jdk.write_to(&version_file)?;

        println!(
            "Created .kopi-version file for {}@{}",
            selected_jdk.distribution, selected_jdk.version
        );

        Ok(())
    }

    fn local_version_path(&self) -> Result<PathBuf> {
        let current_dir = std::env::current_dir()
            .map_err(|e| KopiError::SystemError(format!("Failed to get current directory: {e}")))?;
        Ok(current_dir.join(".kopi-version"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_command_creation() {
        let command = LocalCommand::new().unwrap();
        assert!(!std::ptr::addr_of!(command).is_null());
    }

    #[test]
    fn test_local_version_path() {
        let command = LocalCommand::new().unwrap();
        let path = command.local_version_path().unwrap();
        assert!(path.ends_with(".kopi-version"));
    }
}
