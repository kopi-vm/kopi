use crate::error::{KopiError, Result};
use crate::metadata::{GeneratorConfig, Platform};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

/// Configuration file structure for metadata generator
#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataGenConfigFile {
    pub generator: Option<GeneratorSettings>,
    pub api: Option<ApiSettings>,
    pub output: Option<OutputSettings>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneratorSettings {
    /// Distributions to include (leave empty for all)
    pub distributions: Option<Vec<String>>,
    /// Platforms to generate (leave empty for all)
    pub platforms: Option<Vec<PlatformConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlatformConfig {
    pub os: String,
    pub arch: String,
    pub libc: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiSettings {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_parallel_requests")]
    pub parallel_requests: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutputSettings {
    /// Compress JSON files (default: true)
    #[serde(default = "default_minify_json")]
    pub minify_json: bool,
}

fn default_timeout() -> u64 {
    60
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_parallel_requests() -> usize {
    4
}

fn default_minify_json() -> bool {
    true
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            retry_attempts: default_retry_attempts(),
            parallel_requests: default_parallel_requests(),
        }
    }
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            minify_json: default_minify_json(),
        }
    }
}

impl MetadataGenConfigFile {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| KopiError::InvalidConfig(format!("Failed to read config file: {e}")))?;

        toml::from_str(&content)
            .map_err(|e| KopiError::InvalidConfig(format!("Failed to parse config file: {e}")))
    }

    /// Create a default configuration file
    pub fn default_example() -> Self {
        Self {
            generator: Some(GeneratorSettings {
                distributions: Some(vec![
                    "temurin".to_string(),
                    "corretto".to_string(),
                    "zulu".to_string(),
                    "liberica".to_string(),
                ]),
                platforms: Some(vec![
                    PlatformConfig {
                        os: "linux".to_string(),
                        arch: "x64".to_string(),
                        libc: Some("glibc".to_string()),
                    },
                    PlatformConfig {
                        os: "linux".to_string(),
                        arch: "x64".to_string(),
                        libc: Some("musl".to_string()),
                    },
                    PlatformConfig {
                        os: "linux".to_string(),
                        arch: "aarch64".to_string(),
                        libc: Some("glibc".to_string()),
                    },
                    PlatformConfig {
                        os: "windows".to_string(),
                        arch: "x64".to_string(),
                        libc: None,
                    },
                    PlatformConfig {
                        os: "macos".to_string(),
                        arch: "x64".to_string(),
                        libc: None,
                    },
                    PlatformConfig {
                        os: "macos".to_string(),
                        arch: "aarch64".to_string(),
                        libc: None,
                    },
                ]),
            }),
            api: Some(ApiSettings::default()),
            output: Some(OutputSettings::default()),
        }
    }

    /// Apply configuration file settings to GeneratorConfig
    pub fn apply_to_config(&self, config: &mut GeneratorConfig) -> Result<()> {
        // Apply generator settings
        if let Some(generator) = &self.generator {
            // Apply distributions if not already set by CLI
            if config.distributions.is_none() && generator.distributions.is_some() {
                config.distributions = generator.distributions.clone();
            }

            // Apply platforms if not already set by CLI
            if config.platforms.is_none() && generator.platforms.is_some() {
                let mut platforms = Vec::new();
                for pc in generator.platforms.as_ref().unwrap() {
                    let platform = Platform::from_str(&format!(
                        "{}-{}{}",
                        pc.os,
                        pc.arch,
                        pc.libc
                            .as_ref()
                            .map(|l| format!("-{l}"))
                            .unwrap_or_default()
                    ))?;
                    platforms.push(platform);
                }
                config.platforms = Some(platforms);
            }
        }

        // Apply API settings
        if let Some(api) = &self.api {
            // Only apply parallel_requests if not overridden by CLI
            if config.parallel_requests == 4 {
                // default value
                config.parallel_requests = api.parallel_requests;
            }
        }

        // Apply output settings
        if let Some(output) = &self.output {
            // Only apply minify_json if not explicitly set by --no-minify flag
            // Since we can't distinguish between default true and explicit true,
            // we'll always apply the config value unless --no-minify was used
            config.minify_json = output.minify_json;
        }

        Ok(())
    }
}
