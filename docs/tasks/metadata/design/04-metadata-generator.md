# Metadata Generator CLI Tool

## Overview

A CLI tool to generate metadata files from the foojay API for use with LocalDirectorySource and HTTP/Web Source.

### Why This Tool Is Needed

The foojay API requires two API calls to get complete JDK metadata:
1. **List packages**: Returns basic metadata but missing `download_url` and `checksum`
2. **Get package by ID**: Returns complete details including download URL and checksum

This tool handles the complexity of:
- Making multiple API calls per JDK package
- Managing rate limits and API quotas
- Organizing metadata into efficient file structures
- Creating platform-specific metadata files
- Generating the index.json for intelligent filtering

## Tool Name: `kopi-metadata-gen`

## Command Structure

```bash
# Generate metadata for all distributions and platforms
kopi-metadata-gen generate --output ./metadata

# Generate for specific distributions
kopi-metadata-gen generate --output ./metadata --distributions temurin,corretto

# Generate for specific platforms only
kopi-metadata-gen generate --output ./metadata --platforms linux-x64-glibc,macos-aarch64

# Generate metadata files only
kopi-metadata-gen generate --output ./metadata

# Create archive using standard tools
tar czf metadata-2024-01.tar.gz -C ./metadata .

# Update existing metadata (incremental)
kopi-metadata-gen update --input ./metadata --output ./metadata-updated

# Validate metadata structure
kopi-metadata-gen validate --input ./metadata
```

## Design

```rust
use clap::{Parser, Subcommand};
use crate::api::ApiClient;
use crate::models::metadata::JdkMetadata;

#[derive(Parser)]
#[command(name = "kopi-metadata-gen")]
#[command(about = "Generate metadata files from foojay API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate metadata from foojay API
    Generate {
        /// Output directory for metadata files
        #[arg(short, long)]
        output: PathBuf,
        
        /// Specific distributions to include (comma-separated)
        #[arg(long)]
        distributions: Option<String>,
        
        /// Specific platforms to include (format: os-arch-libc)
        #[arg(long)]
        platforms: Option<String>,
        
        
        /// Include JavaFX bundled versions
        #[arg(long)]
        javafx: bool,
        
        /// Number of parallel API requests
        #[arg(long, default_value = "4")]
        parallel: usize,
    },
    
    /// Update existing metadata
    Update {
        /// Input directory with existing metadata
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for updated metadata
        #[arg(short, long)]
        output: PathBuf,
    },
    
    /// Validate metadata structure
    Validate {
        /// Directory to validate
        #[arg(short, long)]
        input: PathBuf,
    },
}

struct MetadataGenerator {
    api_client: ApiClient,
    config: GeneratorConfig,
}

struct GeneratorConfig {
    distributions: Option<Vec<String>>,
    platforms: Option<Vec<Platform>>,
    javafx_bundled: bool,
    parallel_requests: usize,
}

#[derive(Clone)]
struct Platform {
    os: String,
    arch: String,
    libc: Option<String>,
}

impl MetadataGenerator {
    fn new(config: GeneratorConfig) -> Self {
        Self {
            api_client: ApiClient::new(),
            config,
        }
    }
    
    /// Generate metadata files
    fn generate(&self, output_dir: &Path) -> Result<()> {
        // Step 1: Fetch all distributions
        let distributions = self.fetch_distributions()?;
        
        // Step 2: Collect all platform combinations
        let platforms = self.collect_platforms(&distributions)?;
        
        // Step 3: Fetch metadata for each platform
        let metadata_by_platform = self.fetch_metadata_parallel(&platforms)?;
        
        // Step 4: Organize metadata by distribution and platform
        let organized_files = self.organize_metadata(metadata_by_platform)?;
        
        // Step 5: Create index.json
        let index = self.create_index(&organized_files)?;
        
        // Step 6: Write files
        self.write_output(output_dir, &index, &organized_files)?;
        
        Ok(())
    }
    
    /// Fetch metadata with lazy loading handling
    fn fetch_metadata_for_platform(&self, platform: &Platform) -> Result<Vec<JdkMetadata>> {
        let mut metadata = Vec::new();
        
        // Fetch basic metadata
        let packages = self.api_client.get_packages(Some(PackageQuery {
            architecture: Some(platform.arch.clone()),
            operating_system: Some(platform.os.clone()),
            lib_c_type: platform.libc.clone(),
            javafx_bundled: Some(self.config.javafx_bundled),
            ..Default::default()
        }))?;
        
        // Convert and fetch missing fields
        for package in packages {
            let mut jdk_metadata = convert_package_to_jdk_metadata(package)?;
            
            // Fetch missing fields (download_url, checksum)
            if jdk_metadata.download_url.is_none() {
                let details = self.api_client.get_package_by_id(&jdk_metadata.id)?;
                jdk_metadata.download_url = Some(details.direct_download_uri);
                jdk_metadata.checksum = Some(details.checksum);
                jdk_metadata.checksum_type = Some(parse_checksum_type(&details.checksum_type));
            }
            
            jdk_metadata.is_complete = true;
            metadata.push(jdk_metadata);
        }
        
        Ok(metadata)
    }
    
    /// Batch fetch with rate limit handling
    fn fetch_metadata_parallel(&self, platforms: &[Platform]) -> Result<HashMap<Platform, Vec<JdkMetadata>>> {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let results = Arc::new(Mutex::new(HashMap::new()));
        let semaphore = Arc::new(Semaphore::new(self.config.parallel_requests));
        
        let handles: Vec<_> = platforms.iter()
            .map(|platform| {
                let platform = platform.clone();
                let results = Arc::clone(&results);
                let semaphore = Arc::clone(&semaphore);
                let generator = self.clone();
                
                thread::spawn(move || {
                    let _permit = semaphore.acquire();
                    match generator.fetch_metadata_for_platform(&platform) {
                        Ok(metadata) => {
                            results.lock().unwrap().insert(platform, metadata);
                        }
                        Err(e) => {
                            eprintln!("Error fetching metadata for {:?}: {}", platform, e);
                        }
                    }
                })
            })
            .collect();
        
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
        
        Arc::try_unwrap(results)
            .unwrap()
            .into_inner()
            .unwrap()
    }
    
    /// Create index.json with platform filtering metadata
    fn create_index(&self, files: &HashMap<String, FileMetadata>) -> Result<IndexFile> {
        let mut entries = Vec::new();
        
        for (path, metadata) in files {
            entries.push(IndexFileEntry {
                path: path.clone(),
                distribution: metadata.distribution.clone(),
                architectures: Some(vec![metadata.architecture.clone()]),
                operating_systems: Some(vec![metadata.os.clone()]),
                lib_c_types: metadata.libc.as_ref().map(|l| vec![l.clone()]),
                size: metadata.content.len() as u64,
                checksum: Some(calculate_sha256(&metadata.content)),
                last_modified: Some(Utc::now().to_rfc3339()),
            });
        }
        
        Ok(IndexFile {
            version: 2,
            updated: Utc::now().to_rfc3339(),
            files: entries,
        })
    }
    
    /// Write output files
    fn write_output(&self, output_dir: &Path, index: &IndexFile, files: &HashMap<String, FileMetadata>) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;
        
        // Write index.json
        let index_path = output_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(index)?;
        fs::write(&index_path, &index_json)?;
        
        // Write metadata files
        for (path, metadata) in files {
            let file_path = output_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, &metadata.content)?;
        }
        
        Ok(())
    }
}

/// Progress reporting
impl MetadataGenerator {
    fn report_progress(&self, message: &str) {
        println!("📦 {}", message);
    }
    
    fn create_progress_bar(&self, total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    }
}
```

## Configuration File Support

```toml
# metadata-gen.toml
[generator]
# Distributions to include (leave empty for all)
distributions = ["temurin", "corretto", "zulu", "liberica"]

# Platforms to generate (leave empty for all)
platforms = [
    { os = "linux", arch = "x86_64", libc = "glibc" },
    { os = "linux", arch = "x86_64", libc = "musl" },
    { os = "linux", arch = "aarch64", libc = "glibc" },
    { os = "windows", arch = "x86_64" },
    { os = "macos", arch = "x86_64" },
    { os = "macos", arch = "aarch64" },
]

# API settings
[api]
timeout_secs = 60
retry_attempts = 3
parallel_requests = 4

# Output settings
[output]
# Compress JSON files (default: true)
minify_json = true
```

## Features

1. **Parallel Fetching**: Fetch metadata for multiple platforms concurrently
2. **Incremental Updates**: Only fetch new/changed metadata
3. **Platform Filtering**: Generate only for specific platforms
4. **Progress Reporting**: Show progress with visual indicators
5. **Validation**: Ensure generated files are valid
6. **Rate Limit Handling**: Respect foojay API rate limits
7. **Resume Support**: Continue from interruption
8. **Dry Run Mode**: Preview what would be generated
9. **Diff Report**: Show changes between versions

## Update Command Details

The `update` command provides incremental updates to existing metadata, optimizing API calls for periodic synchronization.

### How It Works

The Foojay API requires two types of API calls:
1. **List API** (`/packages`): Returns basic metadata (without `download_url` and `checksum`)
2. **Detail API** (`/packages/{id}`): Returns complete package information

The update process:
```rust
// 1. Load existing metadata from input directory
let existing_metadata = load_existing_metadata(input_dir)?;

// 2. Fetch current list from API (unavoidable)
let current_list = api_client.get_packages()?;  // e.g., 1000 JDKs

// 3. Compare and detect changes using basic metadata
let updates_needed = detect_changes(&existing_metadata, &current_list);

// 4. Fetch details only for changed items
for jdk in updates_needed {  // e.g., only 10 JDKs
    let details = api_client.get_package_by_id(&jdk.id)?;
    // ... update metadata
}
```

### API Call Optimization

| Command | List API Calls | Detail API Calls | Total |
|---------|----------------|------------------|-------|
| Generate | 1 | All JDKs (e.g., 1000) | 1001 |
| Update | 1 | Only changed (e.g., 10) | 11 |

**Note**: The list API call cannot be avoided as we need to check all available JDKs for changes.

### Change Detection Criteria

Changes are detected using fields available in the list API response:
- **New JDK**: ID not present in existing metadata
- **Updated JDK**: Changes in:
  - `distribution_version` (patch releases)
  - `size` (file updates)
  - `latest_build_available` flag
  - `release_status` (e.g., EA → GA)

### Generate vs Update

| Aspect | Generate | Update |
|--------|----------|---------|
| Purpose | Full metadata creation | Incremental synchronization |
| Existing data | Not required | Required |
| API efficiency | Fetches all details | Fetches only changes |
| Use case | Initial setup, full refresh | Periodic updates, CI/CD |
| Output | Complete metadata set | Updated metadata set |

### Benefits

- **Reduced API calls**: Typically 90%+ reduction in detail API calls
- **Faster execution**: Only process changed JDKs
- **Lower bandwidth**: Minimal data transfer
- **CI/CD friendly**: Efficient for automated weekly/daily updates

## Output Structure

```
metadata/
├── index.json
├── linux-x64-glibc/
│   ├── temurin.json
│   ├── corretto.json
│   ├── zulu.json
│   └── ...
├── linux-aarch64-glibc/
│   ├── temurin.json
│   ├── corretto.json
│   └── ...
├── windows-x64/
│   ├── temurin.json
│   ├── corretto.json
│   └── ...
├── macos-x64/
│   ├── temurin.json
│   └── ...
└── macos-aarch64/
    ├── temurin.json
    └── ...
```

## Metadata Grouping Strategy

The generator organizes metadata files to optimize for:
1. **Platform-specific directories**: Each platform has its own directory
2. **Distribution files**: Each distribution gets a separate JSON file within the platform directory
3. **Efficient loading**: Applications can load only the distributions they need for a specific platform

```rust
fn organize_metadata(&self, metadata: Vec<JdkMetadata>) -> HashMap<String, Vec<JdkMetadata>> {
    let mut grouped = HashMap::new();
    
    for jdk in metadata {
        // Create platform directory name
        let platform_dir = if let Some(libc) = &jdk.lib_c_type {
            format!("{}-{}-{}", jdk.operating_system, jdk.architecture, libc)
        } else {
            format!("{}-{}", jdk.operating_system, jdk.architecture)
        };
        
        // Group by platform/distribution
        let key = format!("{}/{}.json", platform_dir, jdk.distribution);
        
        grouped.entry(key).or_insert_with(Vec::new).push(jdk);
    }
    
    grouped
}
```

## Resume Support Implementation

The Resume Support feature allows the generator to continue from interruptions without re-fetching already completed metadata. This is crucial for handling network failures, API rate limits, or manual interruptions.

### State File Design

Instead of a single global state file, the system creates individual `.state` files for each JSON file being generated. This design avoids lock contention in multi-threaded execution.

**State File Locations:**
- `metadata/index.json.state` - Tracks index.json generation
- `metadata/linux-x64-glibc/temurin.json.state` - Tracks individual metadata file generation
- Each JSON file has a corresponding `.state` file in the same directory

### State File Structure

```rust
#[derive(Serialize, Deserialize)]
struct FileState {
    status: FileStatus,
    started_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    attempts: u32,
    error: Option<String>,
    // Checksum of completed file for validation
    checksum: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum FileStatus {
    InProgress,
    Completed,
    Failed,
}
```

### Generation Workflow with State Management

```rust
impl MetadataGenerator {
    fn process_metadata_file(&self, path: &Path, metadata: &[JdkMetadata]) -> Result<()> {
        let state_path = PathBuf::from(format!("{}.state", path.display()));
        
        // 1. Create .state file at work start
        let state = FileState {
            status: FileStatus::InProgress,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            attempts: 1,
            error: None,
            checksum: None,
        };
        fs::write(&state_path, serde_json::to_string(&state)?)?;
        
        // 2. Perform actual processing
        match self.write_metadata_json(path, metadata) {
            Ok(_) => {
                // 3. Update .state on success
                let mut state = state;
                state.status = FileStatus::Completed;
                state.updated_at = Utc::now();
                state.checksum = Some(calculate_file_checksum(path)?);
                fs::write(&state_path, serde_json::to_string(&state)?)?;
                Ok(())
            }
            Err(e) => {
                // Update .state on failure
                let mut state = state;
                state.status = FileStatus::Failed;
                state.error = Some(e.to_string());
                state.updated_at = Utc::now();
                fs::write(&state_path, serde_json::to_string(&state)?)?;
                Err(e)
            }
        }
    }
}
```

### Resume Logic

When `--resume` flag is provided:

```rust
fn should_skip_file(&self, json_path: &Path) -> bool {
    let state_path = PathBuf::from(format!("{}.state", json_path.display()));
    
    if let Ok(content) = fs::read_to_string(&state_path) {
        if let Ok(state) = serde_json::from_str::<FileState>(&content) {
            match state.status {
                FileStatus::Completed => {
                    // Validate the file still exists and matches checksum
                    if let Some(checksum) = state.checksum {
                        if json_path.exists() {
                            if let Ok(current_checksum) = calculate_file_checksum(json_path) {
                                return current_checksum == checksum;
                            }
                        }
                    }
                    false
                }
                FileStatus::InProgress => {
                    // Check if the process is stale (e.g., > 1 hour old)
                    let age = Utc::now() - state.updated_at;
                    age.num_hours() > 1
                }
                FileStatus::Failed => false,
            }
        } else {
            false
        }
    } else {
        false
    }
}
```

### Cleanup Process

After all metadata files are successfully generated:

```rust
fn cleanup_state_files(&self, output_dir: &Path) -> Result<()> {
    // 1. Remove all .state files in subdirectories
    for entry in walkdir::WalkDir::new(output_dir) {
        let entry = entry?;
        let path = entry.path();
        
        // Skip index.json.state for now
        if path.extension() == Some(OsStr::new("state")) 
            && path != output_dir.join("index.json.state") {
            fs::remove_file(path)?;
        }
    }
    
    // 2. Finally remove index.json.state
    let index_state = output_dir.join("index.json.state");
    if index_state.exists() {
        fs::remove_file(index_state)?;
    }
    
    Ok(())
}
```

### Benefits of This Design

1. **Lock-free concurrency**: Each thread works with independent state files
2. **Fine-grained recovery**: Failed files can be regenerated individually
3. **Progress visibility**: File system shows real-time progress
4. **Atomic operations**: Each file generation is an independent atomic operation
5. **Validation support**: Checksums ensure file integrity on resume

### Multi-threaded Execution

The state file design enables safe parallel execution:

```rust
fn generate_parallel(&self, tasks: Vec<GenerationTask>) -> Result<()> {
    let results: Vec<_> = tasks
        .into_par_iter()
        .map(|task| {
            // Each thread checks/creates its own state file
            if self.should_skip_file(&task.output_path) {
                return Ok(());
            }
            self.process_metadata_file(&task.output_path, &task.metadata)
        })
        .collect();
    
    // Check all results
    for result in results {
        result?;
    }
    
    Ok(())
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum MetadataGenError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Invalid platform specification: {0}")]
    InvalidPlatform(String),
    
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimitExceeded(u64),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## Usage Examples

```bash
# Generate for all distributions and platforms
kopi-metadata-gen generate --output ./metadata

# Generate only for Linux platforms
kopi-metadata-gen generate --output ./metadata --platforms linux-x64-glibc,linux-aarch64-glibc

# Generate metadata then create archive for offline use
kopi-metadata-gen generate --output ./metadata
tar czf metadata-$(date +%Y-%m).tar.gz -C ./metadata .

# Dry run to see what would be generated
kopi-metadata-gen generate --output ./metadata --dry-run

# Generate with pretty-printed JSON (default is minified)
kopi-metadata-gen generate --output ./metadata --no-minify

# Update existing metadata (efficient for periodic updates)
kopi-metadata-gen update --input ./metadata --output ./metadata-updated

# Update with diff report (not implemented yet)
kopi-metadata-gen update --input ./metadata --output ./metadata-new --show-diff

# Validate existing metadata
kopi-metadata-gen validate --input ./metadata

# Generate with custom config
kopi-metadata-gen generate --output ./metadata --config metadata-gen.toml

# Resume interrupted generation
kopi-metadata-gen generate --output ./metadata --resume

# Use with GitHub Actions for automated updates
- name: Generate Metadata
  run: |
    kopi-metadata-gen generate --output ./docs
    git add docs/
    git commit -m "Update metadata $(date +%Y-%m-%d)"
    git push
```

## Integration with CI/CD

```yaml
# .github/workflows/update-metadata.yml
name: Update Metadata
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday
  workflow_dispatch:

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Build metadata generator
        run: cargo build --release --bin kopi-metadata-gen
      - name: Update metadata
        run: |
          # First time: use generate
          if [ ! -d ./metadata ]; then
            ./target/release/kopi-metadata-gen generate --output ./metadata
          else
            # Subsequent runs: use update for efficiency
            ./target/release/kopi-metadata-gen update --input ./metadata --output ./metadata-new
            rm -rf ./metadata
            mv ./metadata-new ./metadata
          fi
      - name: Create PR
        uses: peter-evans/create-pull-request@v5
        with:
          title: Update JDK metadata
          commit-message: Update JDK metadata from foojay API
          branch: update-metadata
```