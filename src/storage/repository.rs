use crate::api::Package;
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::jdk::Distribution;
use crate::storage::disk_space::DiskSpaceChecker;
use crate::storage::installation::{InstallationContext, JdkInstaller};
use crate::storage::listing::{InstalledJdk, JdkLister};
use std::fs;
use std::path::{Path, PathBuf};

pub struct JdkRepository {
    config: KopiConfig,
}

impl JdkRepository {
    pub fn new(config: KopiConfig) -> Self {
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

    fn create_test_storage_manager() -> (JdkRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(config);
        (manager, temp_dir)
    }

    #[test]
    fn test_jdk_install_path() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let path = manager
            .jdk_install_path(&distribution, "21.0.1+35.1")
            .unwrap();
        assert!(path.ends_with("jdks/temurin-21.0.1+35.1"));
    }

    #[test]
    fn test_remove_jdk_security() {
        let (manager, _temp) = create_test_storage_manager();

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
        let manager = JdkRepository::new(config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 1024);
    }

    #[test]
    fn test_min_disk_space_default() {
        let temp_dir = TempDir::new().unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 500);
    }
}
