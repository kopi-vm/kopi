use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::installation::auto::{AutoInstaller, InstallationResult};
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use log::{debug, info};
use std::path::PathBuf;
use std::str::FromStr;

pub struct LocalCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> LocalCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, version_spec: &str) -> Result<()> {
        info!("Setting local JDK version to {version_spec}");

        // Parse version specification using lenient parsing
        let version_request = VersionRequest::from_str(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Create storage repository
        let repository = JdkRepository::new(self.config);

        // Check if matching JDK is installed
        let mut matching_jdks = repository.find_matching_jdks(&version_request)?;

        if matching_jdks.is_empty() {
            // Auto-installation is optional for local command
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
                        version: None,
                        distribution: version_request.distribution.clone(),
                        auto_install_enabled: true,
                        auto_install_failed: None,
                        user_declined: true,
                        install_in_progress: false,
                    });
                }
                InstallationResult::AutoInstallDisabled => {
                    // When auto-install is disabled, still create the .kopi-version file
                    // but show a warning about the JDK not being installed
                    let version_file = self.local_version_path()?;
                    std::fs::write(&version_file, version_request.to_string())?;

                    println!("Created .kopi-version file for {version_request}");
                    println!(
                        "Warning: JDK {} is not installed",
                        version_request.version_pattern
                    );
                    println!(
                        "Run 'kopi install {}' to install this JDK",
                        version_request.version_pattern
                    );

                    return Ok(());
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
    use tempfile::TempDir;

    #[test]
    fn test_local_command_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let command = LocalCommand::new(&config).unwrap();
        assert!(!std::ptr::addr_of!(command).is_null());
    }

    #[test]
    fn test_local_version_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let command = LocalCommand::new(&config).unwrap();
        let path = command.local_version_path().unwrap();
        assert!(path.ends_with(".kopi-version"));
    }
}
