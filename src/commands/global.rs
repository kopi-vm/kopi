use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::installation::auto::AutoInstaller;
use crate::models::distribution::Distribution;
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use crate::version::parser::VersionParser;
use log::{debug, info};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

pub struct GlobalCommand;

impl GlobalCommand {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute(&self, version_spec: &str) -> Result<()> {
        info!("Setting global JDK version to {version_spec}");

        // Load configuration
        let config = new_kopi_config()?;
        let parser = VersionParser::new(&config);

        // Parse version specification
        let version_request = parser.parse(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Global command requires a specific version
        let version = version_request.version.as_ref().ok_or_else(|| {
            KopiError::InvalidVersionFormat(
                "Global command requires a specific version (e.g., '21' or 'temurin@21')"
                    .to_string(),
            )
        })?;

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
        let is_installed = self.check_installation(&repository, &distribution, version)?;

        if !is_installed {
            // Auto-installation is mandatory for global command
            println!("JDK {} {} is not installed.", distribution.name(), version);

            let auto_installer = AutoInstaller::new(&config);

            // For global command, we always install if not present
            if auto_installer.should_auto_install() {
                // Build version request for installation
                let install_request = VersionRequest {
                    distribution: Some(distribution.id().to_string()),
                    version: version.clone(),
                    package_type: None,
                };

                println!("Installing JDK...");
                auto_installer.install_jdk(&install_request)?;
            } else {
                return Err(KopiError::JdkNotInstalled {
                    jdk_spec: format!("{} {}", distribution.name(), version),
                    version: Some(version.to_string()),
                    distribution: Some(distribution.id().to_string()),
                    auto_install_enabled: false,
                    auto_install_failed: None,
                    user_declined: false,
                    install_in_progress: false,
                });
            }
        }

        // Write version file
        let version_file = self.global_version_path(&config)?;
        self.write_version_file(&version_file, &version_request)?;

        println!(
            "Global JDK version set to {} {}",
            distribution.name(),
            version
        );

        Ok(())
    }

    fn check_installation(
        &self,
        repository: &JdkRepository,
        distribution: &Distribution,
        version: &crate::version::Version,
    ) -> Result<bool> {
        // Get list of installed JDKs
        if let Ok(installed_jdks) = repository.list_installed_jdks() {
            // Look for an exact match
            for jdk in installed_jdks {
                if jdk.distribution == distribution.id() {
                    // Check if the version matches
                    if version.matches_pattern(&jdk.version) {
                        debug!(
                            "Found installed JDK: {} {}",
                            distribution.name(),
                            jdk.version
                        );
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn global_version_path(&self, config: &crate::config::KopiConfig) -> Result<PathBuf> {
        Ok(config.kopi_home().join("version"))
    }

    fn write_version_file(
        &self,
        path: &PathBuf,
        version_request: &crate::version::parser::ParsedVersionRequest,
    ) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KopiError::SystemError(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Format version string - use minimal representation
        let version = version_request.version.as_ref().unwrap();
        let version_str = if version.minor() == Some(0) && version.patch() == Some(0) {
            // Just major version (e.g., "21" instead of "21.0.0")
            version.major().to_string()
        } else if version.patch() == Some(0) {
            // Major.minor (e.g., "21.1" instead of "21.1.0")
            format!("{}.{}", version.major(), version.minor().unwrap())
        } else {
            // Full version
            version.to_string()
        };

        let version_string = if let Some(dist) = &version_request.distribution {
            format!("{}@{}", dist.id(), version_str)
        } else {
            version_str
        };

        // Write atomically using a temporary file
        let temp_path = path.with_extension("tmp");

        {
            let mut file = fs::File::create(&temp_path).map_err(|e| {
                KopiError::SystemError(format!("Failed to create {}: {}", temp_path.display(), e))
            })?;

            file.write_all(version_string.as_bytes()).map_err(|e| {
                KopiError::SystemError(format!("Failed to write to {}: {}", temp_path.display(), e))
            })?;

            file.flush().map_err(|e| {
                KopiError::SystemError(format!("Failed to flush {}: {}", temp_path.display(), e))
            })?;
        }

        // Rename temp file to final location
        fs::rename(&temp_path, path).map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to rename {} to {}: {}",
                temp_path.display(),
                path.display(),
                e
            ))
        })?;

        debug!("Wrote global version file: {path:?}");
        Ok(())
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

        let version_path = command.global_version_path(&config).unwrap();
        assert_eq!(version_path, temp_dir.path().join("version"));
    }

    #[test]
    fn test_write_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let command = GlobalCommand::new().unwrap();
        let version_file = temp_dir.path().join("version");

        // Test with distribution
        let version_request = crate::version::parser::ParsedVersionRequest {
            distribution: Some(Distribution::Temurin),
            version: Some(crate::version::Version::new(21, 0, 0)),
            package_type: None,
            latest: false,
        };

        command
            .write_version_file(&version_file, &version_request)
            .unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "temurin@21");

        // Test without distribution
        let version_request2 = crate::version::parser::ParsedVersionRequest {
            distribution: None,
            version: Some(crate::version::Version::new(17, 0, 0)),
            package_type: None,
            latest: false,
        };

        command
            .write_version_file(&version_file, &version_request2)
            .unwrap();

        let content2 = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "17");
    }
}
