use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::api::Package;
use crate::models::distribution::Distribution;
use crate::storage::disk_space::DiskSpaceChecker;
use crate::storage::installation::{InstallationContext, JdkInstaller};
use crate::storage::listing::{InstalledJdk, JdkLister};
use crate::version::Version;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

pub struct JdkRepository<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkRepository<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    pub fn jdk_install_path(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
    ) -> Result<PathBuf> {
        let dir_name = format!("{}-{distribution_version}", distribution.id());
        Ok(self.config.jdks_dir()?.join(dir_name))
    }

    pub fn prepare_jdk_installation(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
    ) -> Result<InstallationContext> {
        let install_path = self.jdk_install_path(distribution, distribution_version)?;

        let disk_checker = DiskSpaceChecker::new(self.config.storage.min_disk_space_mb);
        disk_checker.check_disk_space(&install_path, self.config.kopi_home())?;

        let jdks_dir = self.config.jdks_dir()?;
        JdkInstaller::prepare_installation(&jdks_dir, &install_path)
    }

    pub fn finalize_installation(&self, context: InstallationContext) -> Result<PathBuf> {
        JdkInstaller::finalize_installation(context)
    }

    pub fn cleanup_failed_installation(&self, context: &InstallationContext) -> Result<()> {
        JdkInstaller::cleanup_failed_installation(context)
    }

    pub fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>> {
        let jdks_dir = self.config.jdks_dir()?;
        JdkLister::list_installed_jdks(&jdks_dir)
    }

    /// Check if a specific JDK version is installed
    pub fn check_installation(
        &self,
        distribution: &Distribution,
        version: &Version,
    ) -> Result<bool> {
        debug!(
            "Checking installation for {} version {}",
            distribution.id(),
            version
        );

        // Get list of installed JDKs
        if let Ok(installed_jdks) = self.list_installed_jdks() {
            debug!("Found {} installed JDKs", installed_jdks.len());

            // Look for an exact match
            for jdk in installed_jdks {
                debug!(
                    "Checking installed JDK: {} {} at {:?}",
                    jdk.distribution, jdk.version, jdk.path
                );

                if jdk.distribution == distribution.id() {
                    debug!(
                        "Distribution matches. Checking if search version {} matches installed version {}",
                        version, jdk.version
                    );

                    // Check if the installed version matches the search pattern
                    // For example: installed "17.0.15" matches search pattern "17"
                    if jdk.version.matches_pattern(&version.to_string()) {
                        debug!(
                            "Found matching JDK: {} {} (matched pattern {})",
                            distribution.name(),
                            jdk.version,
                            version
                        );
                        return Ok(true);
                    } else {
                        debug!(
                            "Version mismatch: installed version {} does not match search pattern {}",
                            jdk.version, version
                        );
                    }
                }
            }
        }

        debug!(
            "No matching JDK found for {} version {}",
            distribution.id(),
            version
        );
        Ok(false)
    }

    pub fn get_jdk_size(&self, path: &Path) -> Result<u64> {
        JdkLister::get_jdk_size(path)
    }

    pub fn remove_jdk(&self, path: &Path) -> Result<()> {
        let jdks_dir = self.config.jdks_dir()?;
        if !path.starts_with(&jdks_dir) {
            return Err(KopiError::SecurityError(format!(
                "Refusing to remove directory outside of JDKs directory: {path:?}"
            )));
        }

        fs::remove_dir_all(path)?;
        Ok(())
    }

    pub fn save_jdk_metadata(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
        metadata: &Package,
    ) -> Result<()> {
        let jdks_dir = self.config.jdks_dir()?;
        super::save_jdk_metadata(&jdks_dir, distribution, distribution_version, metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestStorage {
        config: KopiConfig,
        _temp_dir: TempDir,
    }

    impl TestStorage {
        fn new() -> Self {
            // Clear any leftover environment variables
            unsafe {
                std::env::remove_var("KOPI_AUTO_INSTALL");
                std::env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            }

            let temp_dir = TempDir::new().unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
            TestStorage {
                config,
                _temp_dir: temp_dir,
            }
        }

        fn manager(&self) -> JdkRepository {
            JdkRepository::new(&self.config)
        }
    }

    #[test]
    fn test_jdk_install_path() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let distribution = Distribution::Temurin;

        let path = manager
            .jdk_install_path(&distribution, "21.0.1+35.1")
            .unwrap();
        assert!(path.ends_with("jdks/temurin-21.0.1+35.1"));
    }

    #[test]
    fn test_remove_jdk_security() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();

        let result = manager.remove_jdk(Path::new("/etc/passwd"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::SecurityError(_)));
    }

    #[test]
    fn test_min_disk_space_from_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[storage]
min_disk_space_mb = 1024
"#,
        )
        .unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(&config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 1024);
    }

    #[test]
    fn test_min_disk_space_default() {
        let temp_dir = TempDir::new().unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(&config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 500);
    }

    #[test]
    fn test_check_installation_empty_repository() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let distribution = Distribution::Temurin;
        let version = Version::new(21, 0, 0);

        let result = manager.check_installation(&distribution, &version).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_installation_with_partial_version() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        // Create a JDK directory with a full version
        fs::create_dir_all(jdks_dir.join("temurin-17.0.15")).unwrap();

        // Search with just major version "17"
        let distribution = Distribution::Temurin;
        let search_version = Version::from_components(17, None, None);

        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for version 17"
        );

        // Search with major.minor "17.0"
        let search_version = Version::from_components(17, Some(0), None);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for version 17.0"
        );

        // Search with full version "17.0.15"
        let search_version = Version::new(17, 0, 15);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for exact version"
        );

        // Search with different patch version should not match
        let search_version = Version::new(17, 0, 14);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            !result,
            "Should not find temurin-17.0.15 when searching for 17.0.14"
        );

        // Search with different minor version should not match
        let search_version = Version::new(17, 1, 0);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            !result,
            "Should not find temurin-17.0.15 when searching for 17.1.0"
        );
    }
}
