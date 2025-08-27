# Metadata Fetch Progress Indicator Design

**Status**: ✅ Implemented (Phases 1-11 completed)

## Overview
Add progress indicator support to the `fetch_and_cache_metadata` function and related metadata fetching operations to provide visual feedback during metadata retrieval from various sources.

## Goals
- Provide real-time feedback during metadata fetching operations
- Support multiple metadata sources with progress reporting
- Maintain consistent progress indicator usage across the codebase
- Update all affected components without maintaining backward compatibility
- **Calculate total steps in advance for deterministic progress bars**

## Progress Step Calculation

### Step-based Progress Strategy

For `fetch_and_cache_metadata`, calculate total steps based on:

#### Base Steps (Always Present)
1. **Initialization** (1 step)
2. **Fetching metadata** (variable based on sources)
3. **Processing metadata** (1 step)
4. **Grouping by distribution** (1 step) 
5. **Saving to cache** (1 step)
6. **Finalization** (1 step)

#### Source-based Steps
- **Single source**: 1 step for fetch attempt
- **Multiple sources with fallback**: 1 step per source attempt (max)

#### Total Steps Formula
```
total_steps = 5 (base) + number_of_sources
```

Example calculations:
- 1 source configured: 6 steps total
- 3 sources configured: 8 steps total

### Progress Updates During Execution

```rust
// Start with calculated total
let total_steps = 5 + sources.len() as u64;
let mut current_step = 0;

// Step 1: Initialization
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Initializing metadata fetch...");

// Step 2-N: Try each source
for (index, source) in sources.iter().enumerate() {
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message(format!("Fetching from source {}/{}: {}", 
                                  index + 1, sources.len(), source.name()));
    
    // Attempt fetch...
    if success { break; }
}

// Step N+1: Processing metadata
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Processing metadata...");

// Step N+2: Grouping
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Organizing by distribution...");

// Step N+3: Saving
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Saving to cache...");

// Step N+4: Complete
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.complete(Some("Metadata cache updated successfully"));
```

## Implementation Plan

### 1. MetadataSource Trait Changes

**File**: `src/metadata/source.rs`

Update the `MetadataSource` trait methods to accept a progress indicator:

```rust
pub trait MetadataSource: Send + Sync {
    fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>;
    fn fetch_distribution(&self, distribution: &str, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>;
    fn ensure_complete(&self, metadata: &mut JdkMetadata, progress: &mut dyn ProgressIndicator) -> Result<()>;
}
```

### 2. Source Implementations

#### 2.1 FoojayMetadataSource
**File**: `src/metadata/foojay.rs`

Progress reporting points:
- Before API call: `progress.set_message("Connecting to Foojay API...")`
- After receiving packages: `progress.set_message(format!("Retrieved {} packages from Foojay", packages.len()))`
- During conversion: `progress.set_message("Processing Foojay metadata...")`
- On completion: Report total packages converted

#### 2.2 HttpMetadataSource  
**File**: `src/metadata/http.rs`

Progress reporting points:
- Start fetch: `progress.set_message("Fetching metadata from HTTP source...")`
- During download: Update with bytes downloaded if available
- Processing: `progress.set_message("Processing HTTP metadata...")`
- Completion: Report number of packages loaded

#### 2.3 LocalDirectorySource
**File**: `src/metadata/local.rs`

Progress reporting points:
- Start: `progress.set_message("Reading local metadata directory...")`
- Per file: `progress.set_message(format!("Reading {}", filename))`
- Completion: `progress.set_message(format!("Loaded {} local packages", count))`

### 3. MetadataProvider Updates

**File**: `src/metadata/provider.rs`

#### 3.1 Method Signature Updates
```rust
pub fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>
pub fn fetch_distribution(&self, distribution: &str, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>

// New method to get source count for progress calculation
pub fn source_count(&self) -> usize {
    self.sources.len()
}
```

#### 3.2 Progress Reporting Strategy
Progress updates should be step-based, not message-only:
```rust
// Assuming progress is already started with total steps
let mut current_step = 1; // Start after initialization

for (index, (source_name, source)) in self.sources.iter().enumerate() {
    // Update step for this source attempt
    current_step += 1;
    progress.update(current_step, None); // Total already set
    progress.set_message(format!("Trying source {}/{}: {}", 
                                  index + 1, self.sources.len(), source_name));
    
    // Try to fetch from source
    match source.fetch_all(progress) {
        Ok(metadata) => {
            progress.set_message(format!("Successfully fetched from '{}'", source_name));
            return Ok(metadata);
        }
        Err(e) => {
            if index < self.sources.len() - 1 {
                progress.set_message(format!("Source '{}' failed, trying next...", source_name));
            }
        }
    }
}
```

### 4. Cache Module Updates

**File**: `src/cache/mod.rs`

#### 4.1 Function Signature Updates
```rust
pub fn fetch_and_cache_metadata(
    config: &KopiConfig,
    progress: &mut dyn ProgressIndicator
) -> Result<MetadataCache>

pub fn fetch_and_cache_distribution(
    distribution_name: &str,
    config: &KopiConfig,
    progress: &mut dyn ProgressIndicator
) -> Result<MetadataCache>
```

#### 4.2 Progress Reporting Flow

The function assumes progress is already started with the correct total by the caller:

```rust
pub fn fetch_and_cache_metadata(
    config: &KopiConfig,
    progress: &mut dyn ProgressIndicator,
    current_step: &mut u64,  // Track current step across the operation
) -> Result<MetadataCache> {
    // Step 1: Initialization (already done by caller)
    
    // Step 2-N: Fetch from sources (delegated to MetadataProvider)
    let provider = MetadataProvider::from_config(config)?;
    let metadata = provider.fetch_all(progress, current_step)?;
    
    // Step N+1: Processing metadata
    *current_step += 1;
    progress.update(*current_step, None);
    progress.set_message("Processing metadata...");
    
    // Convert to cache format
    let mut new_cache = MetadataCache::new();
    
    // Step N+2: Grouping by distribution
    *current_step += 1;
    progress.update(*current_step, None);
    progress.set_message("Organizing metadata by distribution...");
    
    // Group metadata by distribution
    let mut distributions: HashMap<String, Vec<JdkMetadata>> = HashMap::new();
    for jdk in metadata {
        distributions.entry(jdk.distribution.clone())
            .or_default()
            .push(jdk);
    }
    
    // Create distribution caches
    for (dist_name, packages) in distributions {
        // ... existing logic
    }
    
    // Step N+3: Saving to cache
    *current_step += 1;
    progress.update(*current_step, None);
    progress.set_message("Saving metadata to cache...");
    
    let cache_path = config.metadata_cache_path()?;
    new_cache.save(&cache_path)?;
    
    // Step N+4: Complete
    *current_step += 1;
    progress.update(*current_step, None);
    let message = format!("✓ Cached {} distributions with {} packages",
                          new_cache.distributions.len(),
                          new_cache.total_packages());
    progress.complete(Some(message));
    
    Ok(new_cache)
}
```

Note: Consider whether to pass `current_step` as a parameter or manage it internally.

#### 4.3 get_metadata Function
For the fallback case when cache is not found:
```rust
// Create a silent progress indicator for fallback
let mut silent_progress = SilentProgress;
fetch_and_cache_metadata(config, &mut silent_progress)
```

### 5. Commands Update

**File**: `src/commands/cache.rs`

Update the `refresh_cache` function to initialize progress with calculated total steps:
```rust
fn refresh_cache(config: &KopiConfig, no_progress: bool) -> Result<()> {
    let mut progress = ProgressFactory::create(no_progress);
    
    // Calculate total steps based on configured sources
    // We need to peek at the provider to get source count
    let provider = MetadataProvider::from_config(config)?;
    let total_steps = 5 + provider.source_count() as u64;
    
    // Start progress with total steps
    let progress_config = ProgressConfig::new(
        "Refreshing",
        "metadata cache",
        ProgressStyle::Count,  // Use Count style for step-based progress
    ).with_total(total_steps);
    
    progress.start(progress_config);
    
    // Initialize step counter
    let mut current_step = 0;
    
    // Step 1: Initialization
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Initializing metadata fetch...");
    
    // Steps 2-N+4: Handled inside fetch_and_cache_metadata
    let cache = cache::fetch_and_cache_metadata(config, &mut *progress, &mut current_step)?;
    
    // Progress completion is handled inside fetch_and_cache_metadata
    
    // Print summary
    println!("Successfully refreshed metadata cache:");
    println!("  - {} distributions", cache.distributions.len());
    println!("  - {} total packages", cache.total_packages());
    
    Ok(())
}
```

Note: This requires:
- Adding a `source_count()` method to `MetadataProvider`
- Adding a `total_packages()` method to `MetadataCache` (if not already present)

### 6. Test Updates

#### 6.1 Integration Tests

**File**: `tests/cache_integration.rs`

Update all integration tests to provide progress parameter:

```rust
use kopi::indicator::SilentProgress;

#[test]
fn test_fetch_and_cache_metadata() {
    let config = create_test_config();
    let mut progress = SilentProgress;
    let mut current_step = 0;
    
    // Updated function call with progress parameter
    let result = fetch_and_cache_metadata(&config, &mut progress, &mut current_step);
    
    assert!(result.is_ok());
    let cache = result.unwrap();
    assert!(!cache.distributions.is_empty());
}

#[test]
fn test_get_metadata_with_fallback() {
    // Test that get_metadata uses SilentProgress internally
    let config = create_test_config();
    let result = get_metadata(Some("21"), &config);
    assert!(result.is_ok());
}
```

#### 6.2 Unit Tests for MetadataSource Implementations

**File**: `src/metadata/foojay_tests.rs` (if exists) or create new test module

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::SilentProgress;
    
    #[test]
    fn test_foojay_fetch_all_with_progress() {
        let source = FoojayMetadataSource::new();
        let mut progress = SilentProgress;
        
        // Mock or skip actual API call in unit test
        if cfg!(feature = "integration_tests") {
            let result = source.fetch_all(&mut progress);
            assert!(result.is_ok());
        }
    }
}
```

**File**: `src/metadata/http_tests.rs`

```rust
#[test]
fn test_http_source_with_progress() {
    let mock_server = MockServer::start();
    let source = HttpMetadataSource::new(mock_server.url());
    let mut progress = SilentProgress;
    
    mock_server.mock(|when, then| {
        when.method(GET).path("/metadata.json");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"packages": [...]}"#);
    });
    
    let result = source.fetch_all(&mut progress);
    assert!(result.is_ok());
}
```

**File**: `src/metadata/local_tests.rs`

```rust
#[test]
fn test_local_source_with_progress() {
    let temp_dir = TempDir::new().unwrap();
    create_test_metadata_file(&temp_dir);
    
    let source = LocalDirectorySource::new(temp_dir.path().to_path_buf());
    let mut progress = SilentProgress;
    
    let result = source.fetch_all(&mut progress);
    assert!(result.is_ok());
}
```

#### 6.3 Mock MetadataSource Updates

**File**: `src/metadata/provider_tests.rs`

Update the mock implementation to accept progress parameter:

```rust
struct MockMetadataSource {
    id: String,
    name: String,
    packages: Vec<JdkMetadata>,
    available: bool,
}

impl MetadataSource for Arc<MockMetadataSource> {
    fn fetch_all(&self, _progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>> {
        // Note: progress parameter is ignored in mock
        if self.available {
            Ok(self.packages.clone())
        } else {
            Err(KopiError::MetadataFetch("Mock error".to_string()))
        }
    }
    
    fn fetch_distribution(
        &self, 
        distribution: &str,
        _progress: &mut dyn ProgressIndicator
    ) -> Result<Vec<JdkMetadata>> {
        // Filter packages by distribution
        let filtered: Vec<_> = self.packages.iter()
            .filter(|p| p.distribution == distribution)
            .cloned()
            .collect();
        Ok(filtered)
    }
    
    fn ensure_complete(
        &self, 
        metadata: &mut JdkMetadata,
        _progress: &mut dyn ProgressIndicator
    ) -> Result<()> {
        // Mock implementation doesn't need to do anything
        Ok(())
    }
}
```

#### 6.4 Cache Module Tests

**File**: `src/cache/tests.rs`

```rust
#[test]
fn test_cache_operations_with_progress() {
    let config = create_test_config();
    let mut progress = SilentProgress;
    let mut current_step = 0;
    
    // Test fetch_and_cache_metadata
    let cache = fetch_and_cache_metadata(&config, &mut progress, &mut current_step).unwrap();
    assert!(!cache.distributions.is_empty());
    
    // Test fetch_and_cache_distribution
    let mut progress2 = SilentProgress;
    let mut step2 = 0;
    let cache2 = fetch_and_cache_distribution(
        "temurin", 
        &config, 
        &mut progress2,
        &mut step2
    ).unwrap();
    assert!(cache2.distributions.contains_key("temurin"));
}
```

#### 6.5 Install Command Tests

**File**: `tests/install_integration.rs` (new or existing)

```rust
#[test]
fn test_install_with_progress() {
    let config = create_test_config();
    let command = InstallCommand::new(&config, false).unwrap();
    
    // Test that install command properly initializes progress
    // This would be a mock/dry-run test
    let result = command.execute(
        "temurin@21",
        false,  // force
        true,   // dry_run
        false,  // no_progress - testing WITH progress
        None    // timeout
    );
    
    assert!(result.is_ok());
}

#[test]
fn test_ensure_fresh_cache_with_progress() {
    let config = create_test_config();
    let command = InstallCommand::new(&config, false).unwrap();
    let mut progress = SilentProgress;
    
    let cache = command.ensure_fresh_cache(&mut progress);
    assert!(cache.is_ok());
}
```

#### 6.6 Test Helper Functions

Create common test utilities:

```rust
// src/test_helpers.rs or tests/common/mod.rs

use crate::indicator::{ProgressIndicator, ProgressConfig};

/// A test progress indicator that captures all operations for verification
pub struct TestProgressCapture {
    pub messages: Vec<String>,
    pub updates: Vec<(u64, Option<u64>)>,
    pub completed: bool,
    pub error_message: Option<String>,
}

impl TestProgressCapture {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            updates: Vec::new(),
            completed: false,
            error_message: None,
        }
    }
}

impl ProgressIndicator for TestProgressCapture {
    fn start(&mut self, _config: ProgressConfig) {
        self.messages.push("Started".to_string());
    }
    
    fn update(&mut self, current: u64, total: Option<u64>) {
        self.updates.push((current, total));
    }
    
    fn set_message(&mut self, message: String) {
        self.messages.push(message);
    }
    
    fn complete(&mut self, message: Option<String>) {
        self.completed = true;
        if let Some(msg) = message {
            self.messages.push(msg);
        }
    }
    
    fn error(&mut self, message: String) {
        self.error_message = Some(message);
    }
}

// Use in tests to verify progress behavior
#[test]
fn test_progress_tracking() {
    let mut progress = TestProgressCapture::new();
    let mut current_step = 0;
    
    let result = fetch_and_cache_metadata(&config, &mut progress, &mut current_step);
    
    // Verify progress was properly tracked
    assert!(progress.completed);
    assert!(progress.messages.contains(&"Fetching metadata from configured sources...".to_string()));
    assert_eq!(progress.updates.len(), 5); // Should have 5 steps
}
```

#### 6.7 Testing Strategy Summary

1. **Use SilentProgress** for all existing tests to maintain test performance
2. **Add TestProgressCapture** for tests that specifically verify progress behavior
3. **Update all mock implementations** to accept progress parameter
4. **Create integration tests** that verify end-to-end progress flow
5. **Test error scenarios** to ensure progress is properly terminated on failure
6. **Test different environments** (CI, terminal, no-progress flag) where applicable

#### 6.8 Backward Compatibility Tests

Since we're not maintaining backward compatibility, ensure all tests are updated:

```bash
# Run this command to find all test files that need updating
grep -r "fetch_and_cache_metadata\|fetch_distribution\|MetadataSource" --include="*test*.rs" .

# Files that typically need updates:
- tests/cache_integration.rs
- src/cache/tests.rs
- src/metadata/provider_tests.rs
- src/metadata/http_tests.rs
- src/metadata/foojay_tests.rs (if exists)
- src/metadata/local_tests.rs (if exists)
- src/commands/install_tests.rs (if exists)
```

### 7. Error Handling

Ensure progress indicators properly handle errors:
- On error: Call `progress.error(message)` before returning
- Clean up progress state appropriately
- Don't leave progress bars hanging on failure

## Implementation Strategy

### Bottom-Up Implementation Approach

To maintain a compilable codebase throughout the implementation, use a **bottom-up approach** with temporary `SilentProgress` instances:

#### Phase 1: Update Lower-Level Components
1. **Update MetadataSource trait** - Add progress parameter to all methods
2. **Update source implementations** one by one:
   - When updating a source (e.g., FoojayMetadataSource), temporarily use `SilentProgress` in its tests
   - Each source can be implemented and tested independently

#### Phase 2: Update Mid-Level Components with Temporary Fix
3. **Update MetadataProvider**:
   ```rust
   // Temporarily use SilentProgress when calling updated sources
   pub fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
       let mut silent_progress = SilentProgress;
       // Call source with progress parameter
       source.fetch_all(&mut silent_progress)
   }
   ```
   
4. **Update cache module functions**:
   ```rust
   // Initially, keep old signature for external callers
   pub fn fetch_and_cache_metadata(config: &KopiConfig) -> Result<MetadataCache> {
       let mut silent_progress = SilentProgress;
       let mut current_step = 0;
       fetch_and_cache_metadata_with_progress(config, &mut silent_progress, &mut current_step)
   }
   
   // New internal function with progress support
   fn fetch_and_cache_metadata_with_progress(
       config: &KopiConfig,
       progress: &mut dyn ProgressIndicator,
       current_step: &mut u64
   ) -> Result<MetadataCache> {
       // Actual implementation
   }
   ```

#### Phase 3: Update Top-Level Components
5. **Update commands/cache.rs**:
   - Remove temporary wrapper functions
   - Use actual ProgressIndicator instances
   - Wire up proper progress flow

6. **Update commands/install.rs**:
   - Update ensure_fresh_cache to accept progress
   - Add overall progress tracking

#### Phase 4: Cleanup
7. **Remove temporary fixes**:
   - Remove wrapper functions that used SilentProgress
   - Update all callers to pass progress explicitly
   - Ensure no SilentProgress instances remain except in tests

### Key Implementation Rules

1. **Always Keep Code Compilable**: After each change, the code must compile
2. **Use SilentProgress Temporarily**: When updating a component's signature, use `SilentProgress` in callers until they're updated
3. **Test Each Phase**: Each component should be testable after its update
4. **Document Temporary Code**: Mark temporary SilentProgress usage with `// TODO: Replace with actual progress`

### Example Implementation Sequence

```rust
// Step 1: Update MetadataSource trait
pub trait MetadataSource {
    fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>;
}

// Step 2: Update FoojayMetadataSource
impl MetadataSource for FoojayMetadataSource {
    fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>> {
        progress.set_message("Connecting to Foojay API...");
        // Implementation
    }
}

// Step 3: Temporarily fix MetadataProvider to keep it compilable
impl MetadataProvider {
    pub fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        // TODO: Replace with actual progress parameter
        let mut silent_progress = SilentProgress;
        self.sources[0].1.fetch_all(&mut silent_progress)
    }
}

// Step 4: Later, update MetadataProvider properly
impl MetadataProvider {
    pub fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>> {
        // Now properly propagate progress
        self.sources[0].1.fetch_all(progress)
    }
}
```

This approach ensures:
- **No massive breaking changes**: Each component is updated incrementally
- **Continuous testing**: Can run tests after each component update
- **Clear migration path**: Easy to track what's been updated and what remains
- **Reduced merge conflicts**: Smaller, focused changes

## Implementation Order

1. Update MetadataSource trait
2. Update all source implementations (foojay, http, local) with SilentProgress in their callers
3. Update MetadataProvider to accept progress, use SilentProgress in its callers
4. Update cache module functions with progress support, use SilentProgress in callers
5. Update commands/cache.rs to use real progress indicators
6. Update commands/install.rs to use progress throughout
7. Remove all temporary SilentProgress instances (except in tests)
8. Run full test suite to verify functionality

## Testing Strategy

1. **Unit Tests**: Use SilentProgress for all tests
2. **Integration Tests**: Test with SilentProgress to verify flow
3. **Manual Testing**: 
   - Test `kopi cache refresh` with terminal output
   - Test with `--no-progress` flag
   - Test in CI environment (should use SimpleProgress)

## Progress Messages Reference

### Successful Flow
1. "Refreshing metadata cache" (start)
2. "Fetching metadata from configured sources..."
3. "Trying source 1/2: foojay"
4. "Connecting to Foojay API..."
5. "Retrieved 150 packages from Foojay"
6. "Processing Foojay metadata..."
7. "Organizing metadata by distribution..."
8. "Saving metadata to cache..."
9. "✓ Cached 25 distributions with 150 packages"

### Error Recovery Flow
1. "Trying source 1/3: foojay"
2. "Connecting to Foojay API..."
3. "Source 'foojay' failed, trying next..."
4. "Trying source 2/3: local"
5. "Reading local metadata directory..."
6. "Loaded 50 local packages"
7. "✓ Cached 10 distributions with 50 packages"

## Install Command Progress Support

### Overview
The `install` command also needs progress bar support for metadata fetching operations when refreshing the cache during package installation.

### Install Command Steps

The install command has the following major steps:

#### Core Installation Steps
1. **Parse version** (1 step)
2. **Check/refresh metadata cache** (variable, delegated to `fetch_and_cache_metadata`)
3. **Search for matching package** (1 step)
4. **Check if already installed** (1 step)
5. **Fetch checksum** (1 step, optional)
6. **Download JDK** (has own progress)
7. **Verify checksum** (1 step, optional)
8. **Extract archive** (1 step)
9. **Detect JDK structure** (1 step)
10. **Move to final location** (1 step)
11. **Create shims** (1 step, optional)
12. **Finalization** (1 step)

#### Total Steps Calculation
```
base_steps = 8 (always present)
optional_steps = up to 3 (checksum fetch, verify, shims)
cache_refresh_steps = 5 + sources_count (if cache refresh needed)

total = base_steps + optional_steps + cache_refresh_steps (if needed)
```

### Implementation Changes for Install

**File**: `src/commands/install.rs`

#### 1. Update `ensure_fresh_cache` Method

```rust
fn ensure_fresh_cache(&self, progress: &mut dyn ProgressIndicator) -> Result<MetadataCache> {
    let cache_path = self.config.metadata_cache_path()?;
    let max_age = Duration::from_secs(self.config.metadata.cache.max_age_hours * 3600);

    // Check if cache needs refresh
    let should_refresh = /* existing logic */;

    if should_refresh && self.config.metadata.cache.auto_refresh {
        progress.set_message("Refreshing metadata cache...");
        
        // Create sub-progress for cache refresh
        let mut current_step = 0;
        match cache::fetch_and_cache_metadata(self.config, progress, &mut current_step) {
            Ok(cache) => Ok(cache),
            Err(e) => {
                // Fallback to existing cache if available
                /* existing fallback logic */
            }
        }
    } else {
        cache::load_cache(&cache_path)
    }
}
```

#### 2. Update `execute` Method

```rust
pub fn execute(
    &self,
    version_spec: &str,
    force: bool,
    dry_run: bool,
    no_progress: bool,
    timeout_secs: Option<u64>,
) -> Result<()> {
    let mut progress = ProgressFactory::create(no_progress);
    
    // Calculate total steps (approximate, will be refined after cache check)
    let mut total_steps = 8; // Base steps
    if self.config.shims.auto_create_shims {
        total_steps += 1; // Shim creation
    }
    
    // Start progress
    let progress_config = ProgressConfig::new(
        "Installing",
        &format!("{}", version_spec),
        ProgressStyle::Count,
    ).with_total(total_steps);
    progress.start(progress_config);
    
    let mut current_step = 0;
    
    // Step 1: Parse version
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Parsing version specification...");
    let parser = VersionParser::new(self.config);
    let version_request = parser.parse(version_spec)?;
    
    // Step 2: Check/refresh cache (may add steps)
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Checking metadata cache...");
    
    // If cache needs refresh, update total steps
    let cache_needs_refresh = /* check logic */;
    if cache_needs_refresh {
        let provider = MetadataProvider::from_config(self.config)?;
        let cache_steps = 5 + provider.source_count() as u64;
        total_steps += cache_steps;
        progress.update(current_step, Some(total_steps)); // Update with new total
    }
    
    let cache = self.ensure_fresh_cache(&mut *progress)?;
    
    // Step 3: Find matching package
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Searching for matching package...");
    let package = self.find_matching_package(&distribution, version, &version_request)?;
    
    // Step 4: Check if already installed
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Checking installation status...");
    /* existing check logic */
    
    // Step 5: Fetch checksum (optional)
    if jdk_metadata.checksum.is_none() {
        current_step += 1;
        progress.update(current_step, Some(total_steps));
        progress.set_message("Fetching package checksum...");
        /* existing checksum fetch logic */
    }
    
    // Step 6: Download (has its own progress, so just update message)
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Downloading JDK package...");
    let download_result = download_jdk(&jdk_metadata_with_checksum, no_progress, timeout_secs)?;
    
    // Step 7: Verify checksum (optional)
    if jdk_metadata_with_checksum.checksum.is_some() {
        current_step += 1;
        progress.update(current_step, Some(total_steps));
        progress.set_message("Verifying checksum...");
        /* existing verify logic */
    }
    
    // Step 8: Extract archive
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Extracting archive...");
    extract_archive(download_path, &context.temp_path)?;
    
    // Step 9: Detect JDK structure
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Detecting JDK structure...");
    let structure_info = detect_jdk_root(&context.temp_path)?;
    
    // Step 10: Move to final location
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.set_message("Installing JDK to final location...");
    let final_path = self.finalize_with_structure(/* params */)?;
    
    // Step 11: Create shims (optional)
    if self.config.shims.auto_create_shims {
        current_step += 1;
        progress.update(current_step, Some(total_steps));
        progress.set_message("Creating shims...");
        /* existing shim creation logic */
    }
    
    // Step 12: Complete
    current_step += 1;
    progress.update(current_step, Some(total_steps));
    progress.complete(Some(format!(
        "✓ Successfully installed {} {}",
        distribution.name(),
        jdk_metadata.distribution_version
    )));
    
    Ok(())
}
```

### Progress Flow Example

#### With Cache Refresh
```
Installing temurin@21 [1/17] Parsing version specification...
Installing temurin@21 [2/17] Checking metadata cache...
Installing temurin@21 [3/17] Initializing metadata fetch...
Installing temurin@21 [4/17] Trying source 1/2: foojay
Installing temurin@21 [5/17] Trying source 2/2: local
Installing temurin@21 [6/17] Processing metadata...
Installing temurin@21 [7/17] Organizing by distribution...
Installing temurin@21 [8/17] Saving metadata to cache...
Installing temurin@21 [9/17] Searching for matching package...
Installing temurin@21 [10/17] Checking installation status...
Installing temurin@21 [11/17] Fetching package checksum...
Installing temurin@21 [12/17] Downloading JDK package...
Installing temurin@21 [13/17] Verifying checksum...
Installing temurin@21 [14/17] Extracting archive...
Installing temurin@21 [15/17] Detecting JDK structure...
Installing temurin@21 [16/17] Installing JDK to final location...
Installing temurin@21 [17/17] Creating shims...
✓ Successfully installed Temurin 21.0.5+11
```

#### Without Cache Refresh
```
Installing temurin@21 [1/11] Parsing version specification...
Installing temurin@21 [2/11] Checking metadata cache...
Installing temurin@21 [3/11] Searching for matching package...
Installing temurin@21 [4/11] Checking installation status...
Installing temurin@21 [5/11] Fetching package checksum...
Installing temurin@21 [6/11] Downloading JDK package...
Installing temurin@21 [7/11] Verifying checksum...
Installing temurin@21 [8/11] Extracting archive...
Installing temurin@21 [9/11] Detecting JDK structure...
Installing temurin@21 [10/11] Installing JDK to final location...
Installing temurin@21 [11/11] Creating shims...
✓ Successfully installed Temurin 21.0.5+11
```

### Progress Bar Integration Strategy

#### Download Progress Bar Handling

The download operation should maintain its **independent progress bar** separate from the overall installation progress. This provides the best user experience:

**Rationale for Independent Progress Bars:**
1. **Different Granularity**: Overall progress is step-based (e.g., 6/11), while download is byte-based (e.g., 123.4MB/256.8MB)
2. **Better User Feedback**: Users can see both macro-level progress and micro-level download details
3. **Reusability**: Existing `DownloadProgressAdapter` implementation can be reused without modification
4. **Industry Standard**: Similar to npm, cargo, apt, and other package managers

**Display Examples:**

*Terminal with animation support (indicatif):*
```
Installing temurin@21 [6/11] ████████░░░░░░░░░░░░ 55% Downloading JDK package...
  ↳ Downloading temurin@21 ███████████░░░░░░░░ 65% 123.4 MB / 189.5 MB (2.3 MB/s)
```

*CI environment or non-terminal (simple progress):*
```
Installing temurin@21 [6/11] Downloading JDK package...
Downloading temurin@21: 123.4 MB / 189.5 MB (65%)
```

*With --no-progress flag (silent):*
```
Installing temurin@21...
  Downloading temurin-21.0.5 (id: abc123)
```

**Implementation Approach:**
```rust
// Update overall step progress
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Downloading JDK package...");

// Download uses its own progress bar internally
// DownloadProgressAdapter handles its own display independently
let download_result = download_jdk(&jdk_metadata_with_checksum, no_progress, timeout_secs)?;

// After download completes, continue with next step
current_step += 1;
progress.update(current_step, Some(total_steps));
progress.set_message("Verifying checksum...");
```

This approach ensures:
- The overall progress bar shows which step we're on (Downloading is step 6 of 11)
- The download progress bar shows detailed byte-level progress
- Both progress bars can coexist without interfering with each other
- If download is interrupted and resumed, only the download progress resets, not the overall progress

### Notes for Install Command

1. **Dynamic Step Count**: The total step count changes based on whether cache refresh is needed and what optional operations are performed
2. **Independent Download Progress**: The download operation maintains its own byte-based progress bar while the overall progress shows step-based progress
3. **Error Recovery**: Progress should be properly terminated with error state on failures
4. **Dry Run**: In dry-run mode, skip actual operations but still show progress simulation
5. **Progress Bar Hierarchy**: Overall progress is the parent, download progress is a child indicator

## Alternative Design Considerations

### Option A: Indeterminate Progress (Current Approach)
- **Pros**: Simple implementation, no need to calculate steps
- **Cons**: Users don't know how much work remains
- **Use case**: When total work is truly unknown

### Option B: Step-based Progress (Recommended)
- **Pros**: Users see clear progress, better UX
- **Cons**: Need to calculate steps in advance
- **Use case**: When operations have clear phases

### Option C: Hybrid Approach
- **Pros**: Best of both worlds
- **Cons**: More complex implementation
- **Implementation**: Use step-based for main phases, indeterminate for sub-operations

**Decision**: Use **Option B (Step-based Progress)** for better user experience.

## Notes

- No backward compatibility maintained - all call sites will be updated
- Progress indicator is required (not optional) for all metadata operations
- Use SilentProgress for scenarios where no UI feedback is needed
- Consistent message format across all sources for better UX
- Step-based progress provides deterministic feedback to users
- Consider adding sub-progress for large operations (e.g., processing 1000+ packages)

## Implementation Summary

### Completed Phases (1-11)

The metadata progress indicator implementation has been successfully completed through 11 phases:

1. **Phase 1**: Updated MetadataSource trait and all implementations with progress parameter signatures
2. **Phase 2**: Implemented progress reporting in FoojayMetadataSource
3. **Phase 3**: Implemented progress reporting in HttpMetadataSource
4. **Phase 4**: Implemented progress reporting in LocalDirectorySource
5. **Phase 5**: Updated MetadataProvider to propagate progress indicators
6. **Phase 6**: Updated cache module functions with step-based progress tracking
7. **Phase 7**: Integrated progress indicators into cache command
8. **Phase 8**: Added cache refresh progress support to install command
9. **Phase 9**: Implemented full step-based progress for install command
10. **Phase 10**: Updated all integration tests with progress support
11. **Phase 11**: Cleaned up temporary code and optimized performance

### Key Achievements

- **Step-based Progress**: Implemented deterministic progress with calculated total steps
- **Multi-source Support**: All metadata sources (Foojay, HTTP, Local) now report progress
- **Command Integration**: Both `cache refresh` and `install` commands show detailed progress
- **Test Coverage**: Comprehensive test suite with TestProgressCapture helper
- **Performance**: No measurable performance regression from progress updates
- **Clean Architecture**: Bottom-up implementation maintained compilability throughout

### Lessons Learned

1. **Bottom-up Approach Worked Well**: Starting with trait definitions and working up through the stack prevented breaking changes
2. **Step Calculation is Worth It**: Pre-calculating total steps provides much better UX than indeterminate progress
3. **Progress Propagation Pattern**: Passing progress through multiple layers requires careful parameter management
4. **Independent Download Progress**: Maintaining separate progress bars for downloads vs overall installation provides best UX
5. **Test Infrastructure Important**: TestProgressCapture helper enabled comprehensive progress behavior testing

### Deviations from Original Design

- **No ensure_complete method**: The MetadataSource trait didn't have an `ensure_complete` method; instead it has `fetch_package_details`
- **Wrapper Functions**: Used temporary wrapper functions during migration to maintain backward compatibility
- **Step Management**: Passed `current_step` as mutable reference rather than managing internally in some cases
- **Error Handling**: Added `progress.error()` calls on failures to properly terminate progress indicators

### Future Considerations

- Sub-progress indicators for operations processing many items (1000+ packages)
- Progress persistence for resumable operations
- Network retry progress indication
- Parallel source fetching with aggregated progress