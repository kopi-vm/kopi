# Core Architecture

## Current Implementation Structure

The metadata module is organized as follows:

```
src/metadata/
├── mod.rs              # Module exports
├── source.rs           # MetadataSource trait definition
├── provider.rs         # MetadataProvider implementation
├── foojay.rs          # FoojayMetadataSource implementation
├── generator.rs       # MetadataGenerator for kopi-metadata-gen
└── index.rs           # IndexFile structures for metadata files
```

## Core Abstraction

Based on the adopted Option 3 and synchronous I/O decision, here are the core components:

```rust
use crate::models::metadata::JdkMetadata;
use crate::error::Result;

/// Trait for metadata sources (synchronous)
pub trait MetadataSource: Send + Sync {
    /// Get a unique identifier for this source
    fn id(&self) -> &str;

    /// Get a human-readable name for this source
    fn name(&self) -> &str;

    /// Check if the source is available and can be accessed
    fn is_available(&self) -> Result<bool>;

    /// Fetch all available metadata from this source
    /// For foojay: returns metadata with is_complete=false
    /// For local/GitHub: returns metadata with is_complete=true
    fn fetch_all(&self) -> Result<Vec<JdkMetadata>>;

    /// Fetch metadata for a specific distribution
    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>>;

    /// Fetch complete details for a specific package (used by MetadataResolver)
    /// Only needed for sources that return incomplete metadata
    fn fetch_package_details(&self, package_id: &str) -> Result<PackageDetails>;

    /// Get the last update time of the source (if applicable)
    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>>;
}

/// Details fetched for lazy-loaded fields
#[derive(Debug, Clone)]
pub struct PackageDetails {
    pub download_url: String,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
}
```

## Metadata Provider (Current Implementation)

The current implementation is simpler than the full design, supporting only a single source at a time:

```rust
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
            KopiError::InvalidConfig(format!(
                "Primary source '{}' not found",
                self.primary_source
            ))
        })?;

        source.fetch_all()
    }

    /// Fetch metadata for a specific distribution
    pub fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let source = self.sources.get(&self.primary_source).ok_or_else(|| {
            KopiError::InvalidConfig(format!(
                "Primary source '{}' not found",
                self.primary_source
            ))
        })?;

        source.fetch_distribution(distribution)
    }

    /// Ensure metadata has all required fields (lazy loading)
    pub fn ensure_complete(&self, metadata: &mut JdkMetadata) -> Result<()> {
        if !metadata.is_complete {
            let source = self.sources.get(&self.primary_source).ok_or_else(|| {
                KopiError::InvalidConfig(format!(
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
```

## Metadata Format

All sources must provide metadata in the standard `JdkMetadata` format (based on adopted Option 3):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    pub distribution_version: Version,
    pub architecture: Architecture,
    pub operating_system: OperatingSystem,
    pub package_type: PackageType,
    pub archive_type: ArchiveType,

    // Lazy-loaded fields (may be None if not yet loaded from foojay)
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,

    pub size: u64,
    pub lib_c_type: Option<String>,
    pub javafx_bundled: bool,
    pub term_of_support: Option<String>,
    pub release_status: Option<String>,
    pub latest_build_available: Option<bool>,

    // Tracks whether lazy fields have been loaded
    #[serde(skip)]
    pub is_complete: bool,
}
```

## Error Handling

```rust
#[derive(Error, Debug)]
pub enum MetadataSourceError {
    #[error("Source '{0}' is not available")]
    SourceUnavailable(String),

    #[error("Failed to parse metadata: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

## Usage Example

```rust
// In commands or other components
pub fn get_available_jdks(config: &KopiConfig) -> Result<Vec<JdkMetadata>> {
    let provider = MetadataProvider::from_config(config)?;
    let cache = provider.get_metadata()?;

    // Use metadata as before
    let results = cache.search(&search_query)?;
    Ok(results)
}

// Loading lazy fields when needed
pub fn download_jdk(config: &KopiConfig, package_id: &str) -> Result<()> {
    let provider = MetadataProvider::from_config(config)?;
    let mut metadata = provider.find_package(package_id)?;

    // Ensure download_url is loaded
    provider.ensure_complete(&mut metadata)?;

    let download_url = metadata.download_url
        .ok_or_else(|| KopiError::MissingField("download_url"))?;

    // Proceed with download...
}
```

## Key Design Decisions

1. **Synchronous I/O**: Matches existing codebase, avoids async complexity
2. **Trait-based Design**: Clean abstraction for different source types
3. **Lazy Loading**: Optional fields with resolver pattern (Option 3)
4. **Source Management**: MetadataProvider handles multiple sources with fallback
5. **Error Propagation**: Consistent error handling across all sources
