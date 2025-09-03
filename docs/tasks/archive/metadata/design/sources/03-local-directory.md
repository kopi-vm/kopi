# Local Directory Metadata Source

## Overview

The LocalDirectorySource reads metadata from a directory structure that was extracted from the bundled archive during installation. This provides automatic offline fallback when the primary HTTP source is unavailable.

## Directory Structure

After installation, the bundled metadata is extracted to:

```
${KOPI_HOME}/bundled-metadata/
├── index.json                    # Lists all metadata files with version info
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

## Implementation

```rust
pub struct LocalDirectorySource {
    directory: PathBuf,  // ${KOPI_HOME}/bundled-metadata
}

impl LocalDirectorySource {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// Read metadata from extracted directory structure
    fn read_metadata(&self) -> Result<Vec<JdkMetadata>> {
        // Read index.json
        let index_path = self.directory.join("index.json");
        let index_file = File::open(&index_path)
            .map_err(|e| KopiError::MetadataNotFound(format!(
                "Bundled metadata not found at {}: {}",
                index_path.display(), e
            )))?;

        let index: IndexFile = serde_json::from_reader(index_file)?;

        // Get current platform info
        let current_arch = crate::platform::get_current_architecture();
        let current_os = crate::platform::get_current_os();
        let current_libc = crate::platform::get_foojay_libc_type();

        // Build platform directory name
        let platform_dir = if current_os == "linux" {
            format!("{}-{}-{}", current_os, current_arch, current_libc)
        } else {
            format!("{}-{}", current_os, current_arch)
        };

        // Filter files for current platform
        let platform_files = self.filter_files_for_platform(
            index.files,
            &platform_dir
        );

        // Read metadata files from the platform directory
        let mut all_metadata = Vec::new();
        for file_info in platform_files {
            let file_path = self.directory.join(&file_info.path);

            if let Ok(file) = File::open(&file_path) {
                let metadata: Vec<JdkMetadata> = serde_json::from_reader(file)
                    .map_err(|e| KopiError::ParseError(format!(
                        "Failed to parse {}: {}", file_path.display(), e
                    )))?;

                // Mark all as complete since local files have full metadata
                for mut m in metadata {
                    m.is_complete = true;
                    all_metadata.push(m);
                }
            } else {
                log::warn!("Metadata file not found: {}", file_path.display());
            }
        }

        Ok(all_metadata)
    }

    fn filter_files_for_platform(
        &self,
        files: Vec<IndexFileEntry>,
        platform_dir: &str
    ) -> Vec<IndexFileEntry> {
        files.into_iter()
            .filter(|entry| {
                // Check if the file path starts with our platform directory
                entry.path.starts_with(&format!("{}/", platform_dir))
            })
            .collect()
    }
}

impl MetadataSource for LocalDirectorySource {
    fn id(&self) -> &str {
        "local"
    }

    fn name(&self) -> &str {
        "Local Directory"
    }

    fn is_available(&self) -> Result<bool> {
        // Check if the bundled metadata directory exists and has index.json
        let index_path = self.directory.join("index.json");
        Ok(index_path.exists())
    }

    fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        self.read_metadata()
    }

    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let all_metadata = self.read_metadata()?;
        Ok(all_metadata
            .into_iter()
            .filter(|m| m.distribution == distribution)
            .collect())
    }

    fn fetch_package_details(&self, _package_id: &str) -> Result<PackageDetails> {
        // Local directory source always returns complete metadata
        // This method should never be called, but implement for completeness
        Err(KopiError::InvalidOperation(
            "Local directory source provides complete metadata".to_string()
        ))
    }

    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        // Try to get the bundle generation time from index.json
        let index_path = self.directory.join("index.json");
        if let Ok(file) = File::open(&index_path) {
            if let Ok(index) = serde_json::from_reader::<_, serde_json::Value>(file) {
                if let Some(updated) = index.get("updated").and_then(|v| v.as_str()) {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                        return Ok(Some(dt.with_timezone(&chrono::Utc)));
                    }
                }
            }
        }

        // Fallback to index.json modification time
        if let Ok(metadata) = std::fs::metadata(&index_path) {
            if let Ok(modified) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                return Ok(Some(datetime));
            }
        }

        Ok(None)
    }
}
```

## Key Features

### Platform Filtering

- Reads only metadata files relevant to the current platform
- Reduces memory usage and processing time
- Uses the same filtering logic as HTTP source

### Fast Access

- Direct file system access without decompression overhead
- OS file system cache improves repeated access
- No need to parse tar archives

### Simple Structure

- Standard directory layout matches HTTP source
- Easy to inspect and debug
- No complex archive handling

## Installation Process

The installer extracts the bundled metadata archive:

```bash
# During installation
tar xzf kopi-metadata-YYYY-MM.tar.gz -C "${KOPI_HOME}/bundled-metadata/"

# Result: Ready-to-use directory structure
${KOPI_HOME}/bundled-metadata/
├── index.json
├── linux-x64-glibc/
│   ├── temurin.json
│   └── ...
├── windows-x64/
│   ├── temurin.json
│   └── ...
└── ...
```

## Configuration

```toml
[metadata.sources.local]
enabled = true
# Default: Bundled metadata in installation directory
directory = "${KOPI_HOME}/bundled-metadata"
```

## Bundled Metadata

During installation, Kopi extracts a recent snapshot of metadata:

- Location: `${KOPI_HOME}/bundled-metadata/`
- Contains pre-extracted JSON files from release time
- Updated with each Kopi release
- Provides offline capability from first install
- Automatically used when HTTP source fails

## Use Cases

1. **Automatic Fallback**: When HTTP source is unavailable
2. **Offline Environments**: No internet access required
3. **First Install**: Immediate availability without downloads
4. **Network Issues**: Temporary internet connectivity problems
5. **Corporate Networks**: Behind restrictive firewalls
6. **Air-gapped Systems**: Complete offline operation
