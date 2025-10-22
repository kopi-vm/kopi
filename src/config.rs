// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::{KopiError, Result};
use crate::locking::timeout::{
    LockTimeoutParseError, LockTimeoutResolution, LockTimeoutResolver, LockTimeoutSource,
    LockTimeoutValue, parse_timeout_override,
};
use crate::paths::{cache, home};
use config::{Config, ConfigError, Environment, File};
use dirs::home_dir;
use log::warn;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_MIN_DISK_SPACE_MB: u64 = 500;
const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 600;

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
    pub metadata: MetadataConfig,

    #[serde(default)]
    pub locking: LockingConfig,
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

    #[serde(default = "default_true")]
    pub refresh_on_miss: bool,
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        Self {
            max_age_hours: default_metadata_cache_max_age_hours(),
            auto_refresh: true,
            refresh_on_miss: true,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockingConfig {
    #[serde(default = "default_locking_mode")]
    pub mode: LockingMode,

    #[serde(
        default = "default_lock_timeout_value",
        rename = "timeout",
        deserialize_with = "deserialize_lock_timeout",
        serialize_with = "serialize_lock_timeout"
    )]
    configured_timeout: LockTimeoutValue,

    #[serde(skip, default = "default_lock_timeout_value")]
    effective_timeout: LockTimeoutValue,

    #[serde(skip, default)]
    timeout_source: LockTimeoutSource,
}

impl LockingConfig {
    /// Returns the configured timeout as a `Duration` for convenience.
    pub fn timeout(&self) -> Duration {
        self.effective_timeout.as_duration()
    }

    pub fn timeout_value(&self) -> LockTimeoutValue {
        self.effective_timeout
    }

    pub fn set_timeout_value(&mut self, value: LockTimeoutValue) {
        self.configured_timeout = value;
        self.initialize_effective_timeout();
    }

    pub fn configured_timeout_value(&self) -> LockTimeoutValue {
        self.configured_timeout
    }

    pub fn timeout_source(&self) -> LockTimeoutSource {
        self.timeout_source
    }

    pub fn resolve_timeout(
        &mut self,
        cli_override: Option<&str>,
        env_override: Option<&str>,
    ) -> std::result::Result<LockTimeoutResolution, LockTimeoutParseError> {
        let default = default_lock_timeout_value();
        let resolution =
            LockTimeoutResolver::new(cli_override, env_override, self.configured_timeout, default)
                .resolve()?;
        self.effective_timeout = resolution.value;
        self.timeout_source = resolution.source;
        Ok(resolution)
    }

    fn initialize_effective_timeout(&mut self) {
        let default = default_lock_timeout_value();
        if self.configured_timeout == default {
            self.timeout_source = LockTimeoutSource::Default;
        } else {
            self.timeout_source = LockTimeoutSource::Config;
        }
        self.effective_timeout = self.configured_timeout;
    }
}

impl Default for LockingConfig {
    fn default() -> Self {
        let default_timeout = default_lock_timeout_value();
        Self {
            mode: default_locking_mode(),
            configured_timeout: default_timeout,
            effective_timeout: default_timeout,
            timeout_source: LockTimeoutSource::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LockingMode {
    #[default]
    Auto,
    Advisory,
    Fallback,
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

fn default_locking_mode() -> LockingMode {
    LockingMode::Auto
}

fn default_lock_timeout_value() -> LockTimeoutValue {
    LockTimeoutValue::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
}

fn deserialize_lock_timeout<'de, D>(
    deserializer: D,
) -> std::result::Result<LockTimeoutValue, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawTimeout<'a> {
        Number(u64),
        BorrowedStr(&'a str),
        OwnedString(String),
    }

    match RawTimeout::deserialize(deserializer)? {
        RawTimeout::Number(seconds) => Ok(LockTimeoutValue::from_secs(seconds)),
        RawTimeout::BorrowedStr(value) => {
            parse_timeout_override(value).map_err(|err| de::Error::custom(err.to_string()))
        }
        RawTimeout::OwnedString(value) => {
            parse_timeout_override(&value).map_err(|err| de::Error::custom(err.to_string()))
        }
    }
}

fn serialize_lock_timeout<S>(
    value: &LockTimeoutValue,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        LockTimeoutValue::Finite(duration) => serializer.serialize_u64(duration.as_secs()),
        LockTimeoutValue::Infinite => serializer.serialize_str("infinite"),
    }
}

fn default_distribution() -> String {
    "temurin".to_string()
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
        SourceConfig::Foojay {
            name: "foojay-api".to_string(),
            enabled: true,
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
            .set_default("locking.mode", "auto")?
            .set_default("locking.timeout", DEFAULT_LOCK_TIMEOUT_SECS)?
            .set_default("metadata.cache.max_age_hours", 720)?
            .set_default("metadata.cache.auto_refresh", true)?
            .set_default("metadata.cache.refresh_on_miss", true)?;

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
        config.locking.initialize_effective_timeout();
        let _ = config.apply_lock_timeout_overrides(None)?;

        Ok(config)
    }

    pub fn apply_lock_timeout_overrides(
        &mut self,
        cli_override: Option<&str>,
    ) -> Result<LockTimeoutResolution> {
        let env_override = std::env::var("KOPI_LOCK_TIMEOUT").ok();
        self.locking
            .resolve_timeout(cli_override, env_override.as_deref())
            .map_err(|err| KopiError::InvalidConfig(err.to_string()))
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
        home::ensure_jdks_dir(&self.kopi_home).map_err(|error| {
            KopiError::ConfigError(format!("Failed to create jdks directory: {error}"))
        })
    }

    /// Get the cache directory path and create it if it doesn't exist
    pub fn cache_dir(&self) -> Result<PathBuf> {
        home::ensure_cache_dir(&self.kopi_home).map_err(|error| {
            KopiError::ConfigError(format!("Failed to create cache directory: {error}"))
        })
    }

    /// Get the bin directory path for kopi binary and create it if it doesn't exist
    pub fn bin_dir(&self) -> Result<PathBuf> {
        home::ensure_bin_dir(&self.kopi_home).map_err(|error| {
            KopiError::ConfigError(format!("Failed to create bin directory: {error}"))
        })
    }

    /// Get the shims directory path and create it if it doesn't exist
    pub fn shims_dir(&self) -> Result<PathBuf> {
        home::ensure_shims_dir(&self.kopi_home).map_err(|error| {
            KopiError::ConfigError(format!("Failed to create shims directory: {error}"))
        })
    }

    /// Get the path to the metadata cache file (ensures cache directory exists)
    pub fn metadata_cache_path(&self) -> Result<PathBuf> {
        cache::ensure_cache_root(&self.kopi_home).map_err(|error| {
            KopiError::ConfigError(format!("Failed to create cache directory: {error}"))
        })?;
        Ok(cache::metadata_cache_file(&self.kopi_home))
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
    use crate::paths::{cache, home};
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_default_config() {
        // Clear KOPI_HOME to ensure we get the default behavior
        unsafe {
            env::remove_var("KOPI_HOME");
            env::remove_var("KOPI_LOCKING__MODE");
            env::remove_var("KOPI_LOCKING__TIMEOUT");
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let kopi_home = resolve_kopi_home().unwrap();
        let config = KopiConfig::new(kopi_home).unwrap();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(config.default_distribution, "temurin");
        assert_eq!(config.locking.mode, LockingMode::Auto);
        assert_eq!(
            config.locking.timeout_value(),
            LockTimeoutValue::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
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
            env::remove_var("KOPI_LOCKING__MODE");
            env::remove_var("KOPI_LOCKING__TIMEOUT");
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(config.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(config.default_distribution, "temurin");
        assert_eq!(config.kopi_home, temp_dir.path());
        assert_eq!(config.locking.mode, LockingMode::Auto);
        assert_eq!(
            config.locking.timeout_value(),
            LockTimeoutValue::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
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
            env::remove_var("KOPI_LOCKING__MODE");
            env::remove_var("KOPI_LOCKING__TIMEOUT");
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let temp_dir = TempDir::new().unwrap();

        let mut config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        config.storage.min_disk_space_mb = 1024;
        config.default_distribution = "temurin".to_string();
        config.additional_distributions = vec!["mycustom".to_string(), "private-jdk".to_string()];
        config.auto_install.enabled = true;
        config.auto_install.prompt = false;
        config.auto_install.timeout_secs = 600;
        config.locking.mode = LockingMode::Fallback;
        config
            .locking
            .set_timeout_value(LockTimeoutValue::from_secs(900));

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
        assert_eq!(loaded.locking.mode, LockingMode::Fallback);
        assert_eq!(
            loaded.locking.timeout_value(),
            LockTimeoutValue::from_secs(900)
        );
    }

    #[test]
    #[serial]
    fn test_partial_config() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_STORAGE__MIN_DISK_SPACE_MB");
            env::remove_var("KOPI_DEFAULT_DISTRIBUTION");
            env::remove_var("KOPI_LOCKING__MODE");
            env::remove_var("KOPI_LOCKING__TIMEOUT");
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        // Write partial config with only default_distribution
        fs::write(&config_path, r#"default_distribution = "corretto""#).unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.storage.min_disk_space_mb, DEFAULT_MIN_DISK_SPACE_MB);
        assert_eq!(loaded.default_distribution, "corretto");
        assert_eq!(loaded.locking.mode, LockingMode::Auto);
        assert_eq!(
            loaded.locking.timeout_value(),
            LockTimeoutValue::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
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
    fn test_infinite_lock_timeout_from_config() {
        unsafe {
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);

        fs::write(
            &config_path,
            r#"
[locking]
timeout = "infinite"
"#,
        )
        .unwrap();

        let loaded = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(loaded.locking.timeout_value().is_infinite());
    }

    #[test]
    #[serial]
    fn test_lock_timeout_environment_override() {
        unsafe {
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }
        unsafe {
            env::set_var("KOPI_LOCK_TIMEOUT", "45");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(
            config.locking.timeout_value(),
            LockTimeoutValue::from_secs(45)
        );
        assert_eq!(
            config.locking.timeout_source(),
            LockTimeoutSource::Environment
        );

        unsafe {
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }
    }

    #[test]
    #[serial]
    fn test_lock_timeout_cli_override() {
        unsafe {
            env::remove_var("KOPI_LOCK_TIMEOUT");
        }

        let temp_dir = TempDir::new().unwrap();
        let mut config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolution = config
            .apply_lock_timeout_overrides(Some("infinite"))
            .unwrap();

        assert!(config.locking.timeout_value().is_infinite());
        assert_eq!(resolution.source, LockTimeoutSource::Cli);
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
        assert_eq!(jdks_dir, home::jdks_dir(kopi_home));
        assert!(jdks_dir.exists());

        // Test cache directory
        let cache_dir = config.cache_dir().unwrap();
        assert_eq!(cache_dir, home::cache_dir(kopi_home));
        assert!(cache_dir.exists());

        // Test bin directory
        let bin_dir = config.bin_dir().unwrap();
        assert_eq!(bin_dir, home::bin_dir(kopi_home));
        assert!(bin_dir.exists());

        // Test metadata cache path
        let cache_path = config.metadata_cache_path().unwrap();
        assert_eq!(cache_path, cache::metadata_cache_file(kopi_home));

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
        assert!(!home::jdks_dir(kopi_home).exists());
        assert!(!home::cache_dir(kopi_home).exists());
        assert!(!home::bin_dir(kopi_home).exists());

        // Access directories - they should be created
        config.jdks_dir().unwrap();
        assert!(home::jdks_dir(kopi_home).exists());

        config.cache_dir().unwrap();
        assert!(home::cache_dir(kopi_home).exists());

        config.bin_dir().unwrap();
        assert!(home::bin_dir(kopi_home).exists());
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
    fn test_metadata_cache_config_defaults() {
        // Clear any environment variables that might affect the test
        unsafe {
            env::remove_var("KOPI_METADATA__CACHE__MAX_AGE_HOURS");
            env::remove_var("KOPI_METADATA__CACHE__AUTO_REFRESH");
            env::remove_var("KOPI_METADATA__CACHE__REFRESH_ON_MISS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Test default metadata cache settings
        assert_eq!(config.metadata.cache.max_age_hours, 720); // 30 days
        assert!(config.metadata.cache.auto_refresh);
        assert!(config.metadata.cache.refresh_on_miss);
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
