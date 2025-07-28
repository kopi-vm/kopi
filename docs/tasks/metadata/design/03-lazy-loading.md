# Lazy Loading Design

The foojay API doesn't provide `checksum` and `download_url` in the initial package list response. These fields require a separate API call using the package ID. After evaluating multiple approaches, **Option 3 (Optional Fields with Resolver Pattern)** has been adopted.

## Option 1: Lazy Properties Pattern

```rust
#[derive(Debug, Clone)]
pub struct LazyJdkMetadata {
    // Basic fields available immediately
    pub id: String,
    pub distribution: String,
    pub version: Version,
    // ... other fields ...
    
    // Lazy-loaded fields
    lazy_fields: Arc<Mutex<Option<LazyFields>>>,
    source: Arc<dyn MetadataSource>,
}

#[derive(Debug, Clone)]
struct LazyFields {
    pub download_url: String,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
}

impl LazyJdkMetadata {
    pub async fn download_url(&self) -> Result<String> {
        self.ensure_lazy_loaded().await?;
        let fields = self.lazy_fields.lock().unwrap();
        Ok(fields.as_ref().unwrap().download_url.clone())
    }
    
    pub async fn checksum(&self) -> Result<Option<String>> {
        self.ensure_lazy_loaded().await?;
        let fields = self.lazy_fields.lock().unwrap();
        Ok(fields.as_ref().unwrap().checksum.clone())
    }
    
    async fn ensure_lazy_loaded(&self) -> Result<()> {
        let mut fields = self.lazy_fields.lock().unwrap();
        if fields.is_none() {
            drop(fields); // Release lock before async call
            let lazy_data = self.source.fetch_package_details(&self.id).await?;
            let mut fields = self.lazy_fields.lock().unwrap();
            *fields = Some(lazy_data);
        }
        Ok(())
    }
}
```

**Pros:**
- Transparent lazy loading
- Fields loaded on first access
- Can batch load in background

**Cons:**
- Complex implementation
- Requires async everywhere
- Mutex overhead
- Hard to serialize

## Option 2: Two-Phase Metadata Pattern

```rust
/// Basic metadata available from list API
#[derive(Debug, Clone)]
pub struct BasicJdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    // ... other fields except checksum and download_url ...
}

/// Complete metadata with all fields
#[derive(Debug, Clone)]
pub struct CompleteJdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    // ... all fields including checksum and download_url ...
}

#[async_trait]
pub trait MetadataSource {
    /// Get basic metadata (fast, no extra API calls)
    async fn fetch_basic_metadata(&self) -> Result<Vec<BasicJdkMetadata>>;
    
    /// Get complete metadata for specific package (requires extra API call)
    async fn fetch_complete_metadata(&self, id: &str) -> Result<CompleteJdkMetadata>;
    
    /// Batch fetch complete metadata for multiple packages
    async fn fetch_complete_metadata_batch(&self, ids: &[String]) -> Result<Vec<CompleteJdkMetadata>>;
}
```

**Pros:**
- Type safety: Compiler guarantees that `CompleteJdkMetadata` has all fields
- Clear API contract: Different methods for different data
- No nullable fields in `CompleteJdkMetadata`

**Cons:**
- Data duplication: Same fields in two different structs
- Conversion overhead: Need to convert between types
- Cache complexity: Need to cache both types separately

## Option 3: Optional Fields with Resolver Pattern (Adopted ✓)

```rust
#[derive(Debug, Clone)]
pub struct JdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    // Optional fields that might not be loaded
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
    // Track whether optional fields are loaded
    #[serde(skip)]
    pub is_complete: bool,
}

/// Resolver for fetching missing fields
pub struct MetadataResolver {
    source: Arc<dyn MetadataSource>,
}

impl MetadataResolver {
    /// Ensure metadata has all required fields
    pub async fn ensure_complete(&self, metadata: &mut JdkMetadata) -> Result<()> {
        if !metadata.is_complete {
            let details = self.source.fetch_package_details(&metadata.id).await?;
            metadata.download_url = Some(details.download_url);
            metadata.checksum = details.checksum;
            metadata.checksum_type = details.checksum_type;
            metadata.is_complete = true;
        }
        Ok(())
    }
    
    /// Batch resolve multiple metadata entries
    pub async fn ensure_complete_batch(&self, metadata_list: &mut [JdkMetadata]) -> Result<()> {
        let incomplete_ids: Vec<String> = metadata_list
            .iter()
            .filter(|m| !m.is_complete)
            .map(|m| m.id.clone())
            .collect();
            
        if !incomplete_ids.is_empty() {
            let details = self.source.fetch_package_details_batch(&incomplete_ids).await?;
            // Update metadata with fetched details
        }
        Ok(())
    }
}
```

**Pros:**
- Single type: No conversion needed
- Gradual loading: Can load fields as needed
- Cache-friendly: One cache for all states
- Minimal changes to existing code

**Cons:**
- Runtime checks: Must check `is_complete` or `Option` values
- Less type safety: Possible to access `None` values

## Option 4: Callback-Based Loading Pattern

```rust
#[derive(Debug, Clone)]
pub struct JdkMetadata {
    // All fields as before
    pub download_url: String,
    pub checksum: Option<String>,
    // ...
}

pub type MetadataLoader = Arc<dyn Fn(&str) -> BoxFuture<'static, Result<(String, Option<String>)>> + Send + Sync>;

pub struct MetadataWithLoader {
    pub metadata: JdkMetadata,
    pub loader: Option<MetadataLoader>,
}

impl MetadataWithLoader {
    /// Create with placeholder values that will be loaded on demand
    pub fn with_lazy_fields(basic: BasicJdkMetadata, loader: MetadataLoader) -> Self {
        Self {
            metadata: JdkMetadata {
                download_url: String::new(), // Placeholder
                checksum: None,
                // ... copy other fields from basic ...
            },
            loader: Some(loader),
        }
    }
    
    /// Load the lazy fields if needed
    pub async fn ensure_loaded(&mut self) -> Result<()> {
        if self.metadata.download_url.is_empty() && self.loader.is_some() {
            let loader = self.loader.as_ref().unwrap();
            let (url, checksum) = loader(&self.metadata.id).await?;
            self.metadata.download_url = url;
            self.metadata.checksum = checksum;
        }
        Ok(())
    }
}
```

**Pros:**
- Flexible loading strategies
- Can swap loaders at runtime
- Clear separation of concerns

**Cons:**
- Complex type signatures
- Hard to serialize
- Callback management overhead

## Comparison: Option 2 vs Option 3

### Option 2: Two-Phase Metadata Pattern

**Usage:**
```rust
// Phase 1: Get basic metadata
let basic_list: Vec<BasicJdkMetadata> = source.fetch_basic_metadata().await?;

// Phase 2: Convert to complete when needed
let complete: CompleteJdkMetadata = source.fetch_complete_metadata(&basic.id).await?;
```

| Aspect | Option 2 (Two-Phase) | Option 3 (Optional Fields) |
|--------|---------------------|---------------------------|
| Type Safety | ✅ Complete at compile time | ⚠️ Checked at runtime |
| Number of Types | 2 (Basic + Complete) | 1 (JdkMetadata) |
| Memory Usage | Higher (duplication) | Lower (single instance) |
| API Complexity | Higher (multiple methods) | Lower (single method + resolver) |
| Serialization | Need custom handling | Works out of the box |
| Cache Implementation | Complex (two caches) | Simple (one cache) |
| Existing Code Impact | Major refactoring | Minimal changes |

## Source-Specific Considerations

Different metadata sources have different characteristics:

| Source | Data Availability | download_url | checksum |
|--------|------------------|--------------|----------|
| Foojay API | Two-phase | Requires separate API call | Requires separate API call |
| Local Directory | Complete | Available immediately | Available immediately |
| GitHub | Complete | Available immediately | Available immediately |

This difference strongly influences the design choice:

**Option 2 Impact:**
```rust
// Local Directory Source - forced to create artificial separation
impl MetadataSource for LocalDirectorySource {
    fn fetch_basic_metadata(&self) -> Result<Vec<BasicJdkMetadata>> {
        let complete_data = self.load_from_archives()?;
        // Wasteful: Strip out fields we already have
        complete_data.into_iter()
            .map(|full| BasicJdkMetadata {
                id: full.id,
                distribution: full.distribution,
                // ... copy all fields except download_url and checksum
            })
            .collect()
    }
    
    fn fetch_complete_metadata(&self, id: &str) -> Result<CompleteJdkMetadata> {
        // Redundant: Re-read the same data
        let all_data = self.load_from_archives()?;
        all_data.into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| Error::NotFound)
    }
}
```

**Option 3 Impact:**
```rust
// Local Directory Source - natural implementation
impl MetadataSource for LocalDirectorySource {
    fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        let data = self.load_from_archives()?;
        // Simply return complete data with is_complete = true
        data.into_iter()
            .map(|mut m| {
                m.is_complete = true;  // All fields are already populated
                m
            })
            .collect()
    }
}

// Usage is uniform across sources
let metadata = source.fetch_all()?;
// Resolver automatically skips already-complete entries
resolver.ensure_complete_batch(&mut metadata)?;
```

## Recommendation Summary

For Kopi's use case, **Option 3 (Optional Fields with Resolver Pattern)** is strongly recommended because:

1. **Uniform interface**: All sources implement the same simple interface
2. **No artificial separation**: Sources with complete data don't need to pretend they have two phases
3. **Efficient for all sources**: 
   - Foojay: Lazy loads only when needed
   - Local/GitHub: Already complete, no extra work
4. **Single cache strategy**: One metadata format works for all sources
5. **Natural implementation**: Each source works according to its capabilities

The implementation would be:
- **FoojayMetadataSource**: Returns metadata with `is_complete: false`, loads details on demand
- **LocalDirectorySource**: Returns metadata with `is_complete: true`, all fields populated
- **HttpMetadataSource**: Returns metadata with `is_complete: true`, all fields populated
- **MetadataResolver**: Checks `is_complete` flag and only fetches missing data when needed

## Async I/O Consideration

The current design uses async methods in the `MetadataSource` trait. However, this decision should be reconsidered:

### Current State
- The existing codebase is **entirely synchronous**
- Uses `attohttpc` (synchronous HTTP client) for API calls
- No async runtime dependencies (tokio, async-std, etc.)
- CLI tool with sequential operations

### Reasons to Stay Synchronous
1. **Simplicity**: No async runtime complexity
2. **Consistency**: Matches existing codebase
3. **Dependencies**: Avoids adding tokio or similar
4. **CLI nature**: Users expect sequential operations
5. **Minimal benefit**: Most operations are sequential anyway

### Recommendation: Use Synchronous I/O

For Kopi's use case, **synchronous I/O is recommended**:

```rust
/// Trait for metadata sources (synchronous version)
pub trait MetadataSource: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn is_available(&self) -> Result<bool>;
    fn fetch_all(&self) -> Result<Vec<JdkMetadata>>;
    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>>;
    fn fetch_package_details(&self, package_id: &str) -> Result<PackageDetails>;
    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>>;
}
```

This approach:
- Maintains consistency with existing code
- Simplifies implementation and testing
- Avoids unnecessary complexity
- Can be made async later if needed