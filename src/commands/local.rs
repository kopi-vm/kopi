use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::installation::auto::{AutoInstaller, InstallationResult};
use crate::models::distribution::Distribution;
use crate::storage::JdkRepository;
use crate::version::{
    build_install_request, file::write_version_file, parser::VersionParser,
    validate_version_for_command,
};
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
        let parser = VersionParser::new(&config);

        // Parse version specification
        let version_request = parser.parse(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Local command requires a specific version
        let version = validate_version_for_command(&version_request.version, "Local")?;

        // Validate version semantics
        VersionParser::validate_version_semantics(version)?;

        // Use default distribution from config if not specified
        let distribution = if let Some(dist) = &version_request.distribution {
            dist.clone()
        } else {
            Distribution::from_str(&config.default_distribution).unwrap_or(Distribution::Temurin)
        };

        // Create storage repository
        let repository = JdkRepository::new(&config);

        // Check if JDK is installed
        let is_installed = repository.check_installation(&distribution, version)?;

        if !is_installed {
            // Auto-installation is optional for local command
            println!("JDK {} {} is not installed.", distribution.name(), version);

            let auto_installer = AutoInstaller::new(&config);
            let version_spec = format!("{}@{}", distribution.id(), version);
            let install_request = build_install_request(&distribution, version);

            match auto_installer.prompt_and_install(&version_spec, &install_request) {
                Ok(InstallationResult::Installed) => {
                    info!(
                        "JDK {} {} installed successfully",
                        distribution.name(),
                        version
                    );
                }
                Ok(InstallationResult::UserDeclined | InstallationResult::AutoInstallDisabled) => {
                    // For local command, we continue even if installation is skipped
                    eprintln!("The .kopi-version file will still be created.");
                }
                Err(e) => {
                    eprintln!("Warning: Failed to install JDK: {e}");
                    eprintln!("The .kopi-version file will still be created.");
                    eprintln!("You can install the JDK later with:");
                    eprintln!("  kopi install {version_spec}");
                }
            }
        }

        // Always create the version file, regardless of installation status
        let version_file = self.local_version_path()?;
        write_version_file(&version_file, &version_request)?;

        println!(
            "Created .kopi-version file for {} {}",
            distribution.name(),
            version
        );

        if !is_installed && !repository.check_installation(&distribution, version)? {
            println!();
            println!("Note: The JDK is not installed yet. Run the following to install it:");
            println!("  kopi install {}@{}", distribution.id(), version);
        }

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
        let command = LocalCommand::new().unwrap();
        assert!(!std::ptr::addr_of!(command).is_null());
    }

    #[test]
    fn test_local_version_path() {
        let command = LocalCommand::new().unwrap();
        let path = command.local_version_path().unwrap();
        assert!(path.ends_with(".kopi-version"));
    }

    #[test]
    fn test_write_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join(".kopi-version");

        // Test with distribution
        let version_request = crate::version::parser::ParsedVersionRequest {
            distribution: Some(Distribution::Temurin),
            version: Some(crate::version::Version::new(21, 0, 0)),
            package_type: None,
            latest: false,
        };

        crate::version::file::write_version_file(&version_file, &version_request).unwrap();

        let content = std::fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "temurin@21");

        // Test without distribution
        let version_request2 = crate::version::parser::ParsedVersionRequest {
            distribution: None,
            version: Some(crate::version::Version::new(17, 0, 0)),
            package_type: None,
            latest: false,
        };

        crate::version::file::write_version_file(&version_file, &version_request2).unwrap();

        let content2 = std::fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "17");

        // Test with full version
        let version_request3 = crate::version::parser::ParsedVersionRequest {
            distribution: Some(Distribution::Corretto),
            version: Some(crate::version::Version::new(11, 0, 21)),
            package_type: None,
            latest: false,
        };

        crate::version::file::write_version_file(&version_file, &version_request3).unwrap();

        let content3 = std::fs::read_to_string(&version_file).unwrap();
        assert_eq!(content3, "corretto@11.0.21");
    }
}
