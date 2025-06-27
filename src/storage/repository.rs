use crate::api::Package;
use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::jdk::Distribution;
use crate::storage::disk_space::DiskSpaceChecker;
use crate::storage::installation::{InstallationContext, JdkInstaller};
use crate::storage::listing::{InstalledJdk, JdkLister};
use dirs::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

const KOPI_DIR_NAME: &str = ".kopi";
const JDKS_DIR_NAME: &str = "jdks";

pub struct JdkRepository {
    kopi_home: PathBuf,
    min_disk_space_mb: u64,
}

impl JdkRepository {
    pub fn new() -> Result<Self> {
        let kopi_home = Self::get_kopi_home()?;
        let config = KopiConfig::load(&kopi_home)?;
        Ok(Self {
            kopi_home,
            min_disk_space_mb: config.storage.min_disk_space_mb,
        })
    }

    pub fn with_home(kopi_home: PathBuf) -> Self {
        let config = KopiConfig::load(&kopi_home).unwrap_or_default();
        Self {
            kopi_home,
            min_disk_space_mb: config.storage.min_disk_space_mb,
        }
    }

    fn get_kopi_home() -> Result<PathBuf> {
        if let Ok(kopi_home) = std::env::var("KOPI_HOME") {
            let path = PathBuf::from(kopi_home);
            if path.is_absolute() {
                return Ok(path);
            }
        }

        home_dir()
            .map(|home| home.join(KOPI_DIR_NAME))
            .ok_or_else(|| KopiError::ConfigError("Unable to determine home directory".to_string()))
    }

    pub fn kopi_home(&self) -> &Path {
        &self.kopi_home
    }

    pub fn jdks_dir(&self) -> PathBuf {
        self.kopi_home.join(JDKS_DIR_NAME)
    }

    pub fn jdk_install_path(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
    ) -> PathBuf {
        let dir_name = format!("{}-{}", distribution.id(), distribution_version);
        self.jdks_dir().join(dir_name)
    }

    pub fn prepare_jdk_installation(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
    ) -> Result<InstallationContext> {
        let install_path = self.jdk_install_path(distribution, distribution_version);

        let disk_checker = DiskSpaceChecker::new(self.min_disk_space_mb);
        disk_checker.check_disk_space(&install_path, &self.kopi_home)?;

        JdkInstaller::prepare_installation(&self.jdks_dir(), &install_path)
    }

    pub fn finalize_installation(&self, context: InstallationContext) -> Result<PathBuf> {
        JdkInstaller::finalize_installation(context)
    }

    pub fn cleanup_failed_installation(&self, context: &InstallationContext) -> Result<()> {
        JdkInstaller::cleanup_failed_installation(context)
    }

    pub fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>> {
        JdkLister::list_installed_jdks(&self.jdks_dir())
    }

    pub fn get_jdk_size(&self, path: &Path) -> Result<u64> {
        JdkLister::get_jdk_size(path)
    }

    pub fn remove_jdk(&self, path: &Path) -> Result<()> {
        if !path.starts_with(self.jdks_dir()) {
            return Err(KopiError::SecurityError(format!(
                "Refusing to remove directory outside of JDKs directory: {:?}",
                path
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
        super::save_jdk_metadata(
            &self.jdks_dir(),
            distribution,
            distribution_version,
            metadata,
        )
    }
}

impl Default for JdkRepository {
    fn default() -> Self {
        Self::new().expect("Failed to initialize JdkRepository")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage_manager() -> (JdkRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = JdkRepository::with_home(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_kopi_home_from_env() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("KOPI_HOME", temp_dir.path());
        }

        let home = JdkRepository::get_kopi_home().unwrap();
        assert_eq!(home, temp_dir.path());

        unsafe {
            std::env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_jdk_install_path() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let path = manager.jdk_install_path(&distribution, "21.0.1+35.1");
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

        let manager = JdkRepository::with_home(temp_dir.path().to_path_buf());
        assert_eq!(manager.min_disk_space_mb, 1024);
    }

    #[test]
    fn test_min_disk_space_default() {
        let temp_dir = TempDir::new().unwrap();

        let manager = JdkRepository::with_home(temp_dir.path().to_path_buf());
        assert_eq!(manager.min_disk_space_mb, 500);
    }
}
