use crate::error::{KopiError, Result};
use config::{Config, ConfigError, Environment, File};
use dirs::home_dir;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_MIN_DISK_SPACE_MB: u64 = 500;

// Directory names
const JDKS_DIR_NAME: &str = "jdks";
const CACHE_DIR_NAME: &str = "cache";
const BIN_DIR_NAME: &str = "bin";
const SHIMS_DIR_NAME: &str = "shims";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KopiConfig {
    #[serde(skip)]
    kopi_home: PathBuf,

    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default = "default_distribution")]
    pub default_distribution: String,

    #[serde(default)]
    pub additional_distributions: Vec<String>,

    #[serde(default)]
    pub auto_install: AutoInstallConfig,

    #[serde(default)]
    pub shims: ShimsConfig,

    #[serde(default)]
    pub cache: CacheConfig,

    #[serde(default)]
    pub metadata: MetadataConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    #[serde(default)]
    pub cache: MetadataCacheConfig,

    #[serde(default = "default_metadata_sources")]
    pub sources: Vec<SourceConfig>,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            cache: MetadataCacheConfig::default(),
            sources: default_metadata_sources(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCacheConfig {
    #[serde(default = "default_metadata_cache_max_age_hours")]
    pub max_age_hours: u64,

    #[serde(default = "default_true")]
    pub auto_refresh: bool,
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        Self {
            max_age_hours: default_metadata_cache_max_age_hours(),
            auto_refresh: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SourceConfig {
    #[serde(rename = "http")]
    Http {
        name: String,
        #[serde(default = "default_true")]
        enabled: bool,
        base_url: String,
        #[serde(default = "default_true")]
        cache_locally: bool,
        #[serde(default = "default_timeout_secs")]
        timeout_secs: u64,
    },
    #[serde(rename = "local")]
    Local {
        name: String,
        #[serde(default = "default_true")]
        enabled: bool,
        directory: String,
        #[serde(default = "default_archive_pattern")]
        archive_pattern: String,
        #[serde(default = "default_true")]
        cache_extracted: bool,
    },
    #[serde(rename = "foojay")]
    Foojay {
        name: String,
        #[serde(default = "default_false")]
        enabled: bool,
        base_url: String,
        #[serde(default = "default_timeout_secs")]
        timeout_secs: u64,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoInstallConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_true")]
    pub prompt: bool,

    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for AutoInstallConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prompt: true,
            timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShimsConfig {
    #[serde(default = "default_true")]
    pub auto_create_shims: bool,
    #[serde(default)]
    pub additional_tools: Vec<String>,
    #[serde(default)]
    pub exclude_tools: Vec<String>,
    #[serde(default = "default_false")]
    pub auto_install: bool,
    #[serde(default = "default_true")]
    pub auto_install_prompt: bool,
    #[serde(default = "default_shim_install_timeout")]
    pub install_timeout: u64,
}

impl Default for ShimsConfig {
    fn default() -> Self {
        Self {
            auto_create_shims: true,
            additional_tools: Vec::new(),
            exclude_tools: Vec::new(),
            auto_install: false,
            auto_install_prompt: true,
            install_timeout: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_max_age_hours")]
    pub max_age_hours: u64,

    #[serde(default = "default_true")]
    pub auto_refresh: bool,

    #[serde(default = "default_true")]
    pub refresh_on_miss: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_age_hours: 720, // 30 days
            auto_refresh: true,
            refresh_on_miss: true,
        }
    }
}

// Default value functions
fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_timeout_secs() -> u64 {
    300
}

fn default_shim_install_timeout() -> u64 {
    600 // 10 minutes for shim-specific installations
}

fn default_min_disk_space_mb() -> u64 {
    DEFAULT_MIN_DISK_SPACE_MB
}

fn default_distribution() -> String {
    "temurin".to_string()
}

fn default_cache_max_age_hours() -> u64 {
    720 // 30 days
}

fn default_metadata_sources() -> Vec<SourceConfig> {
    vec![
        SourceConfig::Http {
            name: "primary-http".to_string(),
            enabled: true,
            base_url: default_http_base_url(),
            cache_locally: true,
            timeout_secs: 30,
        },
        SourceConfig::Local {
            name: "local-fallback".to_string(),
            enabled: true,
            directory: default_local_directory(),
            archive_pattern: default_archive_pattern(),
            cache_extracted: true,
        },
        SourceConfig::Foojay {
            name: "foojay-api".to_string(),
            enabled: false,
            base_url: default_foojay_base_url(),
            timeout_secs: 30,
        },
    ]
}

fn default_metadata_cache_max_age_hours() -> u64 {
    720 // 30 days
}

fn default_http_base_url() -> String {
    "https://kopi-vm.github.io/metadata".to_string()
}

fn default_local_directory() -> String {
    "${KOPI_HOME}/bundled-metadata".to_string()
}

fn default_archive_pattern() -> String {
    "*.tar.gz".to_string()
}

fn default_foojay_base_url() -> String {
    "https://api.foojay.io/disco".to_string()
}

/// Create a new KopiConfig with automatic home directory resolution
pub fn new_kopi_config() -> Result<KopiConfig> {
    let kopi_home = resolve_kopi_home()?;
    KopiConfig::new(kopi_home)
}

/// Resolve the KOPI home directory from environment variable or default location
fn resolve_kopi_home() -> Result<PathBuf> {
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
                "KOPI_HOME environment variable '{}' is not an absolute path. Ignoring and using \
                 default path: {}",
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

impl KopiConfig {
    /// Create a new KopiConfig from the specified home directory
    pub fn new(kopi_home: PathBuf) -> Result<Self> {
        let config_path = kopi_home.join(CONFIG_FILE_NAME);

        // Build the configuration using the config crate
        let mut builder = Config::builder()
            // Set defaults
            .set_default("storage.min_disk_space_mb", DEFAULT_MIN_DISK_SPACE_MB)?
            .set_default("default_distribution", "temurin")?
            .set_default("additional_distributions", Vec::<String>::new())?
            .set_default("auto_install.enabled", true)?
            .set_default("auto_install.prompt", true)?
            .set_default("auto_install.timeout_secs", 300)?
            .set_default("shims.auto_create_shims", true)?
            .set_default("shims.additional_tools", Vec::<String>::new())?
            .set_default("shims.exclude_tools", Vec::<String>::new())?
            .set_default("shims.auto_install", false)?
            .set_default("shims.auto_install_prompt", true)?
            .set_default("shims.install_timeout", 600)?
            .set_default("cache.max_age_hours", 720)?
            .set_default("cache.auto_refresh", true)?
            .set_default("cache.refresh_on_miss", true)?;

        // Add the config file if it exists
        if config_path.exists() {
            log::debug!("Loading config from {config_path:?}");
            builder = builder.add_source(File::from(config_path.clone()).required(false));
        } else {
            log::debug!("Config file not found at {config_path:?}, using defaults");
        }

        // Add environment variables with KOPI_ prefix
        // The config crate will automatically map environment variables to config fields
        // Double underscore (__) is used for nested fields to avoid ambiguity
        // For example: KOPI_AUTO_INSTALL__ENABLED -> auto_install.enabled
        builder = builder.add_source(
            Environment::with_prefix("KOPI")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true),
        );

        // Build and deserialize the configuration
        let settings = builder
            .build()
            .map_err(|e| KopiError::ConfigError(format!("Failed to build config: {e}")))?;

        let mut config: KopiConfig = settings
            .try_deserialize()
            .map_err(|e| KopiError::ConfigError(format!("Failed to deserialize config: {e}")))?;

        // Set the kopi_home path
        config.kopi_home = kopi_home;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = self.kopi_home.join(CONFIG_FILE_NAME);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| KopiError::ConfigError(format!("Failed to serialize config: {e}")))?;

        fs::write(&config_path, contents)?;
        log::debug!("Saved config to {config_path:?}");
        Ok(())
    }

    /// Get the KOPI home directory
    pub fn kopi_home(&self) -> &Path {
        &self.kopi_home
    }

    /// Get the JDKs directory path and create it if it doesn't exist
    pub fn jdks_dir(&self) -> Result<PathBuf> {
        let dir = self.kopi_home.join(JDKS_DIR_NAME);
        fs::create_dir_all(&dir)
            .map_err(|e| KopiError::ConfigError(format!("Failed to create jdks directory: {e}")))?;
        Ok(dir)
    }

    /// Get the cache directory path and create it if it doesn't exist
    pub fn cache_dir(&self) -> Result<PathBuf> {
        let dir = self.kopi_home.join(CACHE_DIR_NAME);
        fs::create_dir_all(&dir).map_err(|e| {
            KopiError::ConfigError(format!("Failed to create cache directory: {e}"))
        })?;
        Ok(dir)
    }

    /// Get the bin directory path for kopi binary and create it if it doesn't exist
    pub fn bin_dir(&self) -> Result<PathBuf> {
        let dir = self.kopi_home.join(BIN_DIR_NAME);
        fs::create_dir_all(&dir)
            .map_err(|e| KopiError::ConfigError(format!("Failed to create bin directory: {e}")))?;
        Ok(dir)
    }

    /// Get the shims directory path and create it if it doesn't exist
    pub fn shims_dir(&self) -> Result<PathBuf> {
        let dir = self.kopi_home.join(SHIMS_DIR_NAME);
        fs::create_dir_all(&dir).map_err(|e| {
            KopiError::ConfigError(format!("Failed to create shims directory: {e}"))
        })?;
        Ok(dir)
    }

    /// Get the path to the metadata cache file (ensures cache directory exists)
    pub fn metadata_cache_path(&self) -> Result<PathBuf> {
        let cache_dir = self.cache_dir()?;
        Ok(cache_dir.join("metadata.json"))
    }

    /// Get the path to the config file
    pub fn config_path(&self) -> PathBuf {
        self.kopi_home.join(CONFIG_FILE_NAME)
    }
}

// Custom error conversion from config::ConfigError
impl From<ConfigError> for KopiError {
    fn from(err: ConfigError) -> Self {
        KopiError::ConfigError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_default_config() {
        // Clear KOPI_HOME to ensure we get the default behavior
        unsafe {
            env::remove_var("KOPI_HOME");
        }

        let kopi_home = resolve_kopi_home().unwrap();
        let config = KopiConfig::new(kopi_home).unwrap();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(config.default_distribution, "temurin");
        // The path should contain .kopi - it could be absolute or relative
        let path_str = config.kopi_home.to_string_lossy();
        assert!(
            path_str.contains(".kopi"),
            "Expected path to contain '.kopi', but got: {path_str}"
        );
    }

    #[test]
    #[serial]
    fn test_load_missing_config() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(config.default_distribution, "temurin");
        assert_eq!(config.kopi_home, temp_dir.path());
    }

    #[test]
    #[serial]
    fn test_save_and_load_config() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();

        let mut config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        config.storage.min_disk_space_mb = 1024;
        config.default_distribution = "temurin".to_string();
        config.additional_distributions = vec!["mycustom".to_string(), "private-jdk".to_string()];
        config.auto_install.enabled = true;
        config.auto_install.prompt = false;
        config.auto_install.timeout_secs = 600;

        config.save().unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, 1024);
        assert_eq!(loaded.default_distribution, "temurin");
        assert_eq!(
            loaded.additional_distributions,
            vec!["mycustom", "private-jdk"]
        );
        assert!(loaded.auto_install.enabled);
        assert!(!loaded.auto_install.prompt);
        assert_eq!(loaded.auto_install.timeout_secs, 600);
    }

    #[test]
    #[serial]
    fn test_partial_config() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        // Write partial config with only default_distribution
        fs::write(&config_path, r#"default_distribution = "corretto""#).unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(loaded.default_distribution, "corretto");
        assert!(loaded.additional_distributions.is_empty());
    }

    #[test]
    #[serial]
    fn test_config_with_storage_section() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        fs::write(
            &config_path,
            r#"
default_distribution = "zulu"
additional_distributions = ["custom1", "custom2"]

[storage]
min_disk_space_mb = 2048
"#,
        )
        .unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, 2048);
        assert_eq!(loaded.default_distribution, "zulu");
        assert_eq!(loaded.additional_distributions, vec!["custom1", "custom2"]);
    }

    #[test]
    #[serial]
    fn test_resolve_kopi_home_from_env() {
        // Clear any existing KOPI_HOME first
        unsafe {
            env::remove_var("KOPI_HOME");
        }

        let temp_dir = TempDir::new().unwrap();
        // Ensure we have a canonicalized absolute path on all platforms
        let abs_path = temp_dir.path().canonicalize().unwrap();

        unsafe {
            env::set_var("KOPI_HOME", &abs_path);
        }
        let result = resolve_kopi_home().unwrap();
        assert_eq!(result, abs_path);

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_kopi_home_relative_path() {
        // Set a relative path
        unsafe {
            env::set_var("KOPI_HOME", "relative/path");
        }

        let result = resolve_kopi_home().unwrap();
        // Should fall back to default home
        assert!(result.ends_with(".kopi"));
        assert!(result.is_absolute());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_kopi_home_default() {
        unsafe {
            env::remove_var("KOPI_HOME");
        }

        let result = resolve_kopi_home().unwrap();
        assert!(result.ends_with(".kopi"));
        assert!(result.is_absolute());
    }

    #[test]
    #[serial]
    fn test_directory_paths() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();
        let config = KopiConfig::new(kopi_home.to_path_buf()).unwrap();

        // Test JDKs directory
        let jdks_dir = config.jdks_dir().unwrap();
        assert_eq!(jdks_dir, kopi_home.join("jdks"));
        assert!(jdks_dir.exists());

        // Test cache directory
        let cache_dir = config.cache_dir().unwrap();
        assert_eq!(cache_dir, kopi_home.join("cache"));
        assert!(cache_dir.exists());

        // Test bin directory
        let bin_dir = config.bin_dir().unwrap();
        assert_eq!(bin_dir, kopi_home.join("bin"));
        assert!(bin_dir.exists());

        // Test metadata cache path
        let cache_path = config.metadata_cache_path().unwrap();
        assert_eq!(cache_path, kopi_home.join("cache").join("metadata.json"));

        // Test config path
        let config_path = config.config_path();
        assert_eq!(config_path, kopi_home.join(CONFIG_FILE_NAME));
    }

    #[test]
    #[serial]
    fn test_directory_creation_on_access() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();
        let config = KopiConfig::new(kopi_home.to_path_buf()).unwrap();

        // Verify directories don't exist initially
        assert!(!kopi_home.join("jdks").exists());
        assert!(!kopi_home.join("cache").exists());
        assert!(!kopi_home.join("bin").exists());

        // Access directories - they should be created
        config.jdks_dir().unwrap();
        assert!(kopi_home.join("jdks").exists());

        config.cache_dir().unwrap();
        assert!(kopi_home.join("cache").exists());

        config.bin_dir().unwrap();
        assert!(kopi_home.join("bin").exists());
    }

    #[test]
    #[serial]
    fn test_auto_install_config_defaults() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Test default auto-install settings
        assert!(config.auto_install.enabled);
        assert!(config.auto_install.prompt);
        assert_eq!(config.auto_install.timeout_secs, 300);
    }

    #[test]
    #[serial]
    fn test_shims_config_defaults() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_SHIMS__AUTO_CREATE_SHIMS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Test default shims settings
        assert!(config.shims.auto_create_shims);
    }

    #[test]
    #[serial]
    fn test_cache_config_defaults() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_CACHE__MAX_AGE_HOURS");
            env::remove_var("KOPI_CACHE__AUTO_REFRESH");
            env::remove_var("KOPI_CACHE__REFRESH_ON_MISS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Test default cache settings
        assert_eq!(config.cache.max_age_hours, 720); // 30 days
        assert!(config.cache.auto_refresh);
        assert!(config.cache.refresh_on_miss);
    }

    #[test]
    #[serial]
    fn test_partial_config_with_auto_install() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        // Write partial config with only auto_install section
        fs::write(
            &config_path,
            r#"
[auto_install]
enabled = true
timeout_secs = 120
"#,
        )
        .unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(loaded.auto_install.enabled);
        assert!(loaded.auto_install.prompt); // Should use default
        assert_eq!(loaded.auto_install.timeout_secs, 120);
        assert_eq!(loaded.default_distribution, "temurin"); // Should use default
    }

    #[test]
    #[serial]
    fn test_env_var_overrides() {
        // Clear any existing environment variables first
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        // Write config with specific values
        fs::write(
            &config_path,
            r#"
default_distribution = "temurin"

[auto_install]
enabled = true
prompt = true
timeout_secs = 300

[storage]
min_disk_space_mb = 500
"#,
        )
        .unwrap();

        // Set environment variables to override config
        // With separator "__", nested fields are separated by double underscores
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "false");
            env::set_var("KOPI_AUTO_INSTALL__PROMPT", "false");
            env::set_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS", "600");
            env::set_var("KOPI_STORAGE__MIN_DISK_SPACE_MB", "1024");
            env::set_var("KOPI_DEFAULT_DISTRIBUTION", "corretto");
        }

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Verify environment overrides
        assert!(!loaded.auto_install.enabled);
        assert!(!loaded.auto_install.prompt);
        assert_eq!(loaded.auto_install.timeout_secs, 600);
        assert_eq!(loaded.storage.min_disk_space_mb, 1024);
        assert_eq!(loaded.default_distribution, "corretto");

        // Cleanup
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }
    }

    #[test]
    #[serial]
    fn test_env_var_invalid_values() {
        let temp_dir = TempDir::new().unwrap();

        // Test invalid timeout value
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS", "not_a_number");
        }
        // The config crate with try_parsing(true) will fail on invalid values
        assert!(KopiConfig::new(temp_dir.path().to_path_buf()).is_err());
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        // Test invalid storage value
        unsafe {
            env::set_var("KOPI_STORAGE__MIN_DISK_SPACE_MB", "invalid");
        }
        // Invalid values cause parsing errors
        assert!(KopiConfig::new(temp_dir.path().to_path_buf()).is_err());
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
        }

        // Test empty distribution value
        unsafe {
            env::set_var("KOPI_DEFAULT_DISTRIBUTION", ""); // Empty value
        }
        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        // Empty string is accepted
        assert_eq!(loaded.default_distribution, "");
        unsafe {
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        // Clear all potential leftover environment variables
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
        }

        // Test invalid boolean value
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "not_a_bool");
        }
        // Invalid boolean values cause parsing errors
        assert!(KopiConfig::new(temp_dir.path().to_path_buf()).is_err());
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }
    }
}
