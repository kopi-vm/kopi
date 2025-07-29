use crate::error::Result;
use crate::metadata::provider::MetadataProvider;
use crate::models::metadata::JdkMetadata;
use std::sync::Arc;

/// Handles lazy loading of metadata fields
pub struct MetadataResolver {
    provider: Arc<MetadataProvider>,
}

impl MetadataResolver {
    /// Create a new resolver with the given provider
    pub fn new(provider: Arc<MetadataProvider>) -> Self {
        Self { provider }
    }

    /// Resolve a single metadata entry, ensuring all fields are loaded
    pub fn resolve(&self, metadata: &mut JdkMetadata) -> Result<()> {
        self.provider.ensure_complete(metadata)
    }

    /// Resolve multiple metadata entries in batch
    pub fn resolve_batch(&self, metadata_list: &mut [JdkMetadata]) -> Result<()> {
        self.provider.ensure_complete_batch(metadata_list)
    }

    /// Get a reference to the underlying provider
    pub fn provider(&self) -> &MetadataProvider {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::foojay::FoojayMetadataSource;
    use crate::models::metadata::JdkMetadata;
    use crate::models::package::{ArchiveType, PackageType};
    use crate::models::platform::{Architecture, OperatingSystem};
    use crate::version::Version;

    fn create_test_metadata(complete: bool) -> JdkMetadata {
        JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::new(21, 0, 1),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: if complete {
                Some("https://example.com/download".to_string())
            } else {
                None
            },
            checksum: if complete {
                Some("abc123".to_string())
            } else {
                None
            },
            checksum_type: if complete {
                Some(crate::models::package::ChecksumType::Sha256)
            } else {
                None
            },
            size: 100_000_000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
            is_complete: complete,
        }
    }

    #[test]
    fn test_resolver_with_complete_metadata() {
        let foojay_source = Box::new(FoojayMetadataSource::new());
        let provider = Arc::new(MetadataProvider::new_with_source(foojay_source));
        let resolver = MetadataResolver::new(provider);

        let mut metadata = create_test_metadata(true);
        let result = resolver.resolve(&mut metadata);
        assert!(result.is_ok());
        assert!(metadata.is_complete);
    }

    #[test]
    fn test_resolver_batch() {
        let foojay_source = Box::new(FoojayMetadataSource::new());
        let provider = Arc::new(MetadataProvider::new_with_source(foojay_source));
        let resolver = MetadataResolver::new(provider);

        let mut metadata_list = vec![
            create_test_metadata(true),
            create_test_metadata(true),
            create_test_metadata(true),
        ];

        let result = resolver.resolve_batch(&mut metadata_list);
        assert!(result.is_ok());
        assert!(metadata_list.iter().all(|m| m.is_complete));
    }
}
