use attohttpc::Session;
use chrono::{DateTime, Utc};
use log::{info, warn};

use crate::error::{KopiError, Result};
use crate::metadata::index::{IndexFile, IndexFileEntry};
use crate::metadata::source::{MetadataSource, PackageDetails};
use crate::models::metadata::JdkMetadata;
use crate::platform::{get_current_architecture, get_current_os, get_foojay_libc_type};
use crate::user_agent;

/// HTTP/Web metadata source that fetches from static web servers
pub struct HttpMetadataSource {
    base_url: String,
    client: Session,
}

impl HttpMetadataSource {
    /// Create a new HTTP metadata source
    pub fn new(base_url: String) -> Self {
        let mut client = Session::new();
        client.header("User-Agent", user_agent::metadata_client());

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Fetch the index file
    pub(crate) fn fetch_index(&self) -> Result<IndexFile> {
        let url = format!("{}/index.json", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch index: {e}")))?;

        if !response.is_success() {
            return Err(KopiError::MetadataFetch(format!(
                "Failed to fetch index: HTTP {}",
                response.status()
            )));
        }

        let index: IndexFile = response
            .json()
            .map_err(|e| KopiError::MetadataFetch(format!("Failed to parse index: {e}")))?;

        Ok(index)
    }

    /// Filter files for the current platform
    fn filter_files_for_platform(&self, files: Vec<IndexFileEntry>) -> Vec<IndexFileEntry> {
        let current_arch = get_current_architecture();
        let current_os = get_current_os();
        let current_libc = get_foojay_libc_type();

        files
            .into_iter()
            .filter(|entry| {
                // Check architecture
                if let Some(ref archs) = entry.architectures {
                    if !archs.contains(&current_arch) {
                        return false;
                    }
                }

                // Check operating system
                if let Some(ref oses) = entry.operating_systems {
                    if !oses.contains(&current_os) {
                        return false;
                    }
                }

                // Check lib_c_type (only for Linux)
                if current_os == "linux" {
                    if let Some(ref lib_c_types) = entry.lib_c_types {
                        if !lib_c_types.contains(&current_libc.to_string()) {
                            return false;
                        }
                    }
                }

                true
            })
            .collect()
    }

    /// Fetch a metadata file from the server
    fn fetch_metadata_file(&self, path: &str) -> Result<Vec<JdkMetadata>> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| KopiError::MetadataFetch(format!("Failed to fetch {path}: {e}")))?;

        if !response.is_success() {
            return Err(KopiError::MetadataFetch(format!(
                "Failed to fetch {}: HTTP {}",
                path,
                response.status()
            )));
        }

        let metadata: Vec<JdkMetadata> = response
            .json()
            .map_err(|e| KopiError::MetadataFetch(format!("Failed to parse {path}: {e}")))?;

        Ok(metadata)
    }
}

impl MetadataSource for HttpMetadataSource {
    fn id(&self) -> &str {
        "http"
    }

    fn name(&self) -> &str {
        "HTTP/Web"
    }

    fn is_available(&self) -> Result<bool> {
        // Try to fetch index to check availability
        match self.fetch_index() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        let mut all_metadata = Vec::new();

        // Fetch index file
        let index = self.fetch_index()?;

        // Filter files for current platform
        let platform_files = self.filter_files_for_platform(index.files);

        info!(
            "Filtered to {} files for current platform (arch: {}, os: {}, libc: {})",
            platform_files.len(),
            get_current_architecture(),
            get_current_os(),
            get_foojay_libc_type()
        );

        // Fetch only metadata files relevant to this platform
        for entry in platform_files {
            match self.fetch_metadata_file(&entry.path) {
                Ok(metadata) => {
                    // HTTP source provides full metadata with download_url and checksums
                    all_metadata.extend(metadata);
                }
                Err(e) => warn!("Failed to fetch {}: {}", entry.path, e),
            }
        }

        Ok(all_metadata)
    }

    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let mut metadata = Vec::new();

        // Fetch index file
        let index = self.fetch_index()?;

        // Filter for platform AND distribution
        let filtered_files: Vec<IndexFileEntry> = self
            .filter_files_for_platform(index.files)
            .into_iter()
            .filter(|entry| entry.distribution == distribution)
            .collect();

        // Fetch only the specific distribution files
        for entry in filtered_files {
            match self.fetch_metadata_file(&entry.path) {
                Ok(pkg_metadata) => {
                    // HTTP source provides full metadata with download_url and checksums
                    metadata.extend(pkg_metadata);
                }
                Err(e) => warn!("Failed to fetch {}: {}", entry.path, e),
            }
        }

        Ok(metadata)
    }

    fn fetch_package_details(&self, _package_id: &str) -> Result<PackageDetails> {
        // HTTP source always returns complete metadata
        Err(KopiError::MetadataFetch(
            "HTTP source provides complete metadata".to_string(),
        ))
    }

    fn last_updated(&self) -> Result<Option<DateTime<Utc>>> {
        let index = self.fetch_index()?;
        let updated = DateTime::parse_from_rfc3339(&index.updated)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();
        Ok(updated)
    }
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
