use crate::error::{KopiError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
