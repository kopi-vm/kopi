use crate::error::{KopiError, Result};
use dirs::home_dir;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_MIN_DISK_SPACE_MB: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KopiConfig {
    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub default_distribution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_min_disk_space_mb")]
    pub min_disk_space_mb: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            min_disk_space_mb: DEFAULT_MIN_DISK_SPACE_MB,
        }
    }
}

fn default_min_disk_space_mb() -> u64 {
    DEFAULT_MIN_DISK_SPACE_MB
}

impl KopiConfig {
    pub fn load(kopi_home: &Path) -> Result<Self> {
        let config_path = kopi_home.join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            log::debug!("Config file not found at {:?}, using defaults", config_path);
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: KopiConfig = toml::from_str(&contents)
            .map_err(|e| KopiError::ConfigError(format!("Failed to parse config.toml: {}", e)))?;

        log::debug!("Loaded config from {:?}", config_path);
        Ok(config)
    }

    pub fn save(&self, kopi_home: &Path) -> Result<()> {
        let config_path = kopi_home.join(CONFIG_FILE_NAME);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| KopiError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, contents)?;
        log::debug!("Saved config to {:?}", config_path);
        Ok(())
    }
}

/// Get the KOPI home directory from environment variable or default location
///
/// This function checks the KOPI_HOME environment variable first.
/// If it's set but not an absolute path, a warning is logged and the default path is used.
/// If not set or invalid, falls back to ~/.kopi
pub fn get_kopi_home() -> Result<PathBuf> {
    // Check KOPI_HOME environment variable first
    if let Ok(kopi_home) = std::env::var("KOPI_HOME") {
        let path = PathBuf::from(&kopi_home);
        if path.is_absolute() {
            return Ok(path);
        } else {
            let default_path = home_dir().map(|home| home.join(".kopi")).ok_or_else(|| {
                KopiError::ConfigError("Unable to determine home directory".to_string())
            })?;
            warn!(
                "KOPI_HOME environment variable '{}' is not an absolute path. Ignoring and using default path: {}",
                kopi_home,
                default_path.display()
            );
            return Ok(default_path);
        }
    }

    // Fall back to ~/.kopi
    home_dir()
        .map(|home| home.join(".kopi"))
        .ok_or_else(|| KopiError::ConfigError("Unable to determine home directory".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = KopiConfig::default();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(config.default_distribution, None);
    }

    #[test]
    fn test_load_missing_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::load(temp_dir.path()).unwrap();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = KopiConfig::default();
        config.storage.min_disk_space_mb = 1024;
        config.default_distribution = Some("temurin".to_string());

        config.save(temp_dir.path()).unwrap();

        let loaded = KopiConfig::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, 1024);
        assert_eq!(loaded.default_distribution, Some("temurin".to_string()));
    }

    #[test]
    fn test_partial_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        // Write partial config with only default_distribution
        fs::write(&config_path, r#"default_distribution = "corretto""#).unwrap();

        let loaded = KopiConfig::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(loaded.default_distribution, Some("corretto".to_string()));
    }

    #[test]
    fn test_config_with_storage_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        fs::write(
            &config_path,
            r#"
default_distribution = "zulu"

[storage]
min_disk_space_mb = 2048
"#,
        )
        .unwrap();

        let loaded = KopiConfig::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, 2048);
        assert_eq!(loaded.default_distribution, Some("zulu".to_string()));
    }

    #[test]
    fn test_get_kopi_home_from_env() {
        let temp_dir = TempDir::new().unwrap();
        let abs_path = temp_dir.path().to_path_buf();

        unsafe {
            env::set_var("KOPI_HOME", &abs_path);
        }
        let result = get_kopi_home().unwrap();
        assert_eq!(result, abs_path);

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_get_kopi_home_relative_path() {
        // Set a relative path
        unsafe {
            env::set_var("KOPI_HOME", "relative/path");
        }

        let result = get_kopi_home().unwrap();
        // Should fall back to default home
        assert!(result.ends_with(".kopi"));
        assert!(result.is_absolute());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_get_kopi_home_default() {
        unsafe {
            env::remove_var("KOPI_HOME");
        }

        let result = get_kopi_home().unwrap();
        assert!(result.ends_with(".kopi"));
        assert!(result.is_absolute());
    }
}
