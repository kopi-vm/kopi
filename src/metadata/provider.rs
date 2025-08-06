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

use crate::config::{KopiConfig, MetadataConfig, SourceConfig};
use crate::error::{KopiError, Result};
use crate::metadata::source::MetadataSource;
use crate::metadata::{FoojayMetadataSource, HttpMetadataSource, LocalDirectorySource};
use crate::models::metadata::JdkMetadata;
use log::{debug, warn};
use std::collections::HashMap;

/// Manages multiple metadata sources with sequential fallback support
pub struct MetadataProvider {
    /// Ordered list of source names and their implementations
    sources: Vec<(String, Box<dyn MetadataSource>)>,
}

impl MetadataProvider {
    /// Create a new provider with a single source
    pub fn new_with_source(source: Box<dyn MetadataSource>) -> Self {
        let source_id = source.id().to_string();
        Self {
            sources: vec![(source_id, source)],
        }
    }

    /// Create a provider from configuration
    pub fn from_config(config: &KopiConfig) -> Result<Self> {
        Self::from_metadata_config(&config.metadata, config.kopi_home())
    }

    /// Create a provider from metadata configuration
    pub fn from_metadata_config(
        metadata_config: &MetadataConfig,
        kopi_home: &std::path::Path,
    ) -> Result<Self> {
        let mut sources: Vec<(String, Box<dyn MetadataSource>)> = Vec::new();

        // Initialize sources based on configuration
        for source_config in &metadata_config.sources {
            match source_config {
                SourceConfig::Http {
                    name,
                    enabled,
                    base_url,
                    ..
                } if *enabled => {
                    debug!("Initializing HTTP metadata source '{name}' at {base_url}");
                    let source = HttpMetadataSource::new(base_url.clone());
                    sources.push((name.clone(), Box::new(source)));
                }
                SourceConfig::Local {
                    name,
                    enabled,
                    directory,
                    ..
                } if *enabled => {
                    debug!("Initializing local metadata source '{name}' at {directory}");
                    // Expand ${KOPI_HOME} in directory path
                    let expanded_directory = if directory.contains("${KOPI_HOME}") {
                        directory.replace("${KOPI_HOME}", &kopi_home.to_string_lossy())
                    } else {
                        directory.clone()
                    };
                    let source =
                        LocalDirectorySource::new(std::path::PathBuf::from(&expanded_directory));
                    sources.push((name.clone(), Box::new(source)));
                }
                SourceConfig::Foojay {
                    name,
                    enabled,
                    base_url,
                    ..
                } if *enabled => {
                    debug!("Initializing Foojay metadata source '{name}' at {base_url}");
                    let source = FoojayMetadataSource::new();
                    sources.push((name.clone(), Box::new(source)));
                }
                _ => {
                    // Source is disabled
                }
            }
        }

        // Validate configuration
        if sources.is_empty() {
            return Err(KopiError::InvalidConfig(
                "No metadata sources are enabled".to_string(),
            ));
        }

        debug!(
            "Initialized {} metadata sources: {:?}",
            sources.len(),
            sources.iter().map(|(name, _)| name).collect::<Vec<_>>()
        );

        Ok(Self { sources })
    }

    /// Get metadata from sources, trying each in order until one succeeds
    pub fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        let mut errors: Vec<(String, String)> = Vec::new();

        for (source_name, source) in &self.sources {
            debug!("Attempting to fetch metadata from source: {source_name}");

            // Check if source is available
            match source.is_available() {
                Ok(true) => {
                    // Source is available, try to fetch
                    match source.fetch_all() {
                        Ok(metadata) => {
                            if errors.is_empty() {
                                debug!("Successfully fetched metadata from source: {source_name}");
                            } else {
                                warn!(
                                    "Successfully fetched metadata from source '{}' after {} failed attempts",
                                    source_name,
                                    errors.len()
                                );
                            }
                            return Ok(metadata);
                        }
                        Err(e) => {
                            warn!("Failed to fetch from source '{source_name}': {e}");
                            errors.push((source_name.clone(), e.to_string()));
                        }
                    }
                }
                Ok(false) => {
                    warn!("Source '{source_name}' is not available");
                    errors.push((source_name.clone(), "Source not available".to_string()));
                }
                Err(e) => {
                    warn!("Error checking availability of source '{source_name}': {e}");
                    errors.push((
                        source_name.clone(),
                        format!("Availability check failed: {e}"),
                    ));
                }
            }
        }

        // All sources failed
        let error_summary = errors
            .iter()
            .map(|(name, err)| format!("{name}: {err}"))
            .collect::<Vec<_>>()
            .join(", ");

        Err(KopiError::MetadataFetch(format!(
            "All {} sources failed: {}",
            errors.len(),
            error_summary
        )))
    }

    /// Fetch metadata for a specific distribution, trying each source in order
    pub fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let mut errors: Vec<(String, String)> = Vec::new();

        for (source_name, source) in &self.sources {
            debug!("Attempting to fetch distribution '{distribution}' from source: {source_name}");

            // Check if source is available
            match source.is_available() {
                Ok(true) => {
                    // Source is available, try to fetch
                    match source.fetch_distribution(distribution) {
                        Ok(metadata) => {
                            if errors.is_empty() {
                                debug!(
                                    "Successfully fetched distribution '{distribution}' from source: {source_name}"
                                );
                            } else {
                                warn!(
                                    "Successfully fetched distribution '{}' from source '{}' after {} failed attempts",
                                    distribution,
                                    source_name,
                                    errors.len()
                                );
                            }
                            return Ok(metadata);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to fetch distribution '{distribution}' from source '{source_name}': {e}"
                            );
                            errors.push((source_name.clone(), e.to_string()));
                        }
                    }
                }
                Ok(false) => {
                    warn!("Source '{source_name}' is not available");
                    errors.push((source_name.clone(), "Source not available".to_string()));
                }
                Err(e) => {
                    warn!("Error checking availability of source '{source_name}': {e}");
                    errors.push((
                        source_name.clone(),
                        format!("Availability check failed: {e}"),
                    ));
                }
            }
        }

        // All sources failed
        let error_summary = errors
            .iter()
            .map(|(name, err)| format!("{name}: {err}"))
            .collect::<Vec<_>>()
            .join(", ");

        Err(KopiError::MetadataFetch(format!(
            "Failed to fetch distribution '{}' from all {} sources: {}",
            distribution,
            errors.len(),
            error_summary
        )))
    }

    /// Ensure metadata has all required fields (lazy loading)
    pub fn ensure_complete(&self, metadata: &mut JdkMetadata) -> Result<()> {
        if !metadata.is_complete() {
            let details = self.fetch_package_details(&metadata.id)?;
            metadata.download_url = Some(details.download_url);
            metadata.checksum = details.checksum;
            metadata.checksum_type = details.checksum_type;
        }
        Ok(())
    }

    /// Fetch package details, trying each source in order
    fn fetch_package_details(
        &self,
        package_id: &str,
    ) -> Result<crate::metadata::source::PackageDetails> {
        let mut errors: Vec<(String, String)> = Vec::new();

        for (source_name, source) in &self.sources {
            debug!(
                "Attempting to fetch package details for '{package_id}' from source: {source_name}"
            );

            // Check if source is available
            match source.is_available() {
                Ok(true) => {
                    // Source is available, try to fetch
                    match source.fetch_package_details(package_id) {
                        Ok(details) => {
                            if errors.is_empty() {
                                debug!(
                                    "Successfully fetched package details for '{package_id}' from source: {source_name}"
                                );
                            } else {
                                warn!(
                                    "Successfully fetched package details for '{}' from source '{}' after {} failed attempts",
                                    package_id,
                                    source_name,
                                    errors.len()
                                );
                            }
                            return Ok(details);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to fetch package details for '{package_id}' from source '{source_name}': {e}"
                            );
                            errors.push((source_name.clone(), e.to_string()));
                        }
                    }
                }
                Ok(false) => {
                    warn!("Source '{source_name}' is not available");
                    errors.push((source_name.clone(), "Source not available".to_string()));
                }
                Err(e) => {
                    warn!("Error checking availability of source '{source_name}': {e}");
                    errors.push((
                        source_name.clone(),
                        format!("Availability check failed: {e}"),
                    ));
                }
            }
        }

        // All sources failed
        let error_summary = errors
            .iter()
            .map(|(name, err)| format!("{name}: {err}"))
            .collect::<Vec<_>>()
            .join(", ");

        Err(KopiError::MetadataFetch(format!(
            "Failed to fetch package details for '{}' from all {} sources: {}",
            package_id,
            errors.len(),
            error_summary
        )))
    }

    /// Batch resolve multiple metadata entries
    pub fn ensure_complete_batch(&self, metadata_list: &mut [JdkMetadata]) -> Result<()> {
        // For now, process each item individually
        // Future optimization: group by source and batch load
        for metadata in metadata_list.iter_mut() {
            self.ensure_complete(metadata)?;
        }
        Ok(())
    }

    /// Check health of all configured sources
    pub fn check_sources_health(&self) -> HashMap<String, SourceHealth> {
        let mut health_status = HashMap::new();

        for (name, source) in &self.sources {
            let health = match source.is_available() {
                Ok(true) => SourceHealth::Available,
                Ok(false) => SourceHealth::Unavailable("Source reports unavailable".to_string()),
                Err(e) => SourceHealth::Unavailable(e.to_string()),
            };

            health_status.insert(name.clone(), health);
        }

        health_status
    }

    /// Get the first available source name
    pub fn get_first_available_source(&self) -> Option<&str> {
        for (name, source) in &self.sources {
            if source.is_available().unwrap_or(false) {
                return Some(name);
            }
        }
        None
    }

    /// List all configured sources in order
    pub fn list_sources(&self) -> Vec<&str> {
        self.sources.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Get the number of configured sources
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

/// Health status of a metadata source
#[derive(Debug, Clone)]
pub enum SourceHealth {
    Available,
    Unavailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::foojay::FoojayMetadataSource;

    #[test]
    fn test_provider_with_single_source() {
        let foojay_source = Box::new(FoojayMetadataSource::new());
        let provider = MetadataProvider::new_with_source(foojay_source);

        assert_eq!(provider.sources.len(), 1);
        assert_eq!(provider.sources[0].0, "foojay");
    }

    #[test]
    fn test_ensure_complete_with_complete_metadata() {
        use crate::models::metadata::JdkMetadata;
        use crate::models::package::{ArchiveType, PackageType};
        use crate::models::platform::{Architecture, OperatingSystem};
        use crate::version::Version;

        let foojay_source = Box::new(FoojayMetadataSource::new());
        let provider = MetadataProvider::new_with_source(foojay_source);

        // Create a metadata that is already complete
        let mut metadata = JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::new(21, 0, 1),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: Some("https://example.com/download".to_string()),
            checksum: Some("abc123".to_string()),
            checksum_type: Some(crate::models::package::ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        // ensure_complete should not make any changes
        let result = provider.ensure_complete(&mut metadata);
        assert!(result.is_ok());
        assert!(metadata.is_complete());
        assert_eq!(
            metadata.download_url,
            Some("https://example.com/download".to_string())
        );
    }
}

// Include comprehensive tests
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
