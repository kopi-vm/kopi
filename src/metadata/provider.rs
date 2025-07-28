use crate::error::Result;
use crate::metadata::source::MetadataSource;
use crate::models::metadata::JdkMetadata;
use std::collections::HashMap;

/// Manages multiple metadata sources
pub struct MetadataProvider {
    sources: HashMap<String, Box<dyn MetadataSource>>,
    primary_source: String,
}

impl MetadataProvider {
    /// Create a new provider with a single source
    pub fn new_with_source(source: Box<dyn MetadataSource>) -> Self {
        let source_id = source.id().to_string();
        let mut sources = HashMap::new();
        sources.insert(source_id.clone(), source);

        Self {
            sources,
            primary_source: source_id,
        }
    }

    /// Get metadata from the primary source
    pub fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        let source = self.sources.get(&self.primary_source).ok_or_else(|| {
            crate::error::KopiError::InvalidConfig(format!(
                "Primary source '{}' not found",
                self.primary_source
            ))
        })?;

        source.fetch_all()
    }

    /// Fetch metadata for a specific distribution
    pub fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let source = self.sources.get(&self.primary_source).ok_or_else(|| {
            crate::error::KopiError::InvalidConfig(format!(
                "Primary source '{}' not found",
                self.primary_source
            ))
        })?;

        source.fetch_distribution(distribution)
    }

    /// Ensure metadata has all required fields (lazy loading)
    pub fn ensure_complete(&self, metadata: &mut JdkMetadata) -> Result<()> {
        if !metadata.is_complete {
            // For now, we assume all metadata comes from the primary source
            // In the future, we'll track which source provided each metadata
            let source = self.sources.get(&self.primary_source).ok_or_else(|| {
                crate::error::KopiError::InvalidConfig(format!(
                    "Primary source '{}' not found",
                    self.primary_source
                ))
            })?;

            let details = source.fetch_package_details(&metadata.id)?;
            metadata.download_url = Some(details.download_url);
            metadata.checksum = details.checksum;
            metadata.checksum_type = details.checksum_type;
            metadata.is_complete = true;
        }
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::foojay::FoojayMetadataSource;

    #[test]
    fn test_provider_with_single_source() {
        let foojay_source = Box::new(FoojayMetadataSource::new());
        let provider = MetadataProvider::new_with_source(foojay_source);

        assert_eq!(provider.primary_source, "foojay");
        assert!(provider.sources.contains_key("foojay"));
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
            is_complete: true,
        };

        // ensure_complete should not make any changes
        let result = provider.ensure_complete(&mut metadata);
        assert!(result.is_ok());
        assert!(metadata.is_complete);
        assert_eq!(
            metadata.download_url,
            Some("https://example.com/download".to_string())
        );
    }
}
