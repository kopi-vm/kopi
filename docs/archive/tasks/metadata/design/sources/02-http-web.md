# HTTP/Web Metadata Source

## Overview

The HTTP source fetches metadata from any static web server. Originally designed for GitHub Pages to avoid API rate limits, this approach works with any web server hosting static JSON files.

## Why Generic HTTP/Web Source?

Since we're just fetching static JSON files via HTTP, this approach is not limited to GitHub Pages:

1. **No vendor lock-in**: Works with any web server
2. **Simple deployment**: Just upload static files
3. **No authentication complexity**: Public HTTP access
4. **No rate limits**: Unlike GitHub API
5. **Flexible hosting options**:
   - GitHub Pages (free, automatic deployment)
   - AWS S3 + CloudFront
   - Cloudflare Pages
   - Netlify
   - Self-hosted Nginx/Apache
   - Any CDN service
   - Corporate internal servers

## Required Structure

The web server must host:

1. An `index.json` that lists available metadata files
2. JSON metadata files in subdirectories

```
metadata/
├── index.json                                    # Lists all metadata files with platform info
├── jdks/
│   ├── temurin-linux-x64-glibc-2024-01.json    # Linux x64 with glibc
│   ├── temurin-linux-x64-musl-2024-01.json     # Linux x64 with musl
│   ├── temurin-linux-aarch64-glibc-2024-01.json # Linux ARM64 with glibc
│   ├── temurin-windows-x64-2024-01.json        # Windows x64
│   ├── temurin-macos-x64-2024-01.json          # macOS Intel
│   ├── temurin-macos-aarch64-2024-01.json      # macOS Apple Silicon
│   ├── corretto-linux-x64-glibc-2024-01.json
│   └── zulu-linux-x64-glibc-2024-01.json
└── (Static web server files)
```

## Index File Format

The index file provides metadata about available files with filtering capabilities:

```json
// index.json
{
  "version": 2,
  "updated": "2024-01-15T10:00:00Z",
  "files": [
    {
      "path": "jdks/temurin-linux-x64-glibc-2024-01.json",
      "distribution": "temurin",
      "architectures": ["x86_64"], // Array to support files with multiple architectures
      "operating_systems": ["linux"],
      "lib_c_types": ["glibc"],
      "size": 45678, // Size of the JSON metadata file in bytes
      "checksum": "sha256:abc123...", // SHA256 of the JSON metadata file content
      "last_modified": "2024-01-15T09:00:00Z"
    },
    {
      "path": "jdks/temurin-windows-x64-2024-01.json",
      "distribution": "temurin",
      "architectures": ["x86_64"],
      "operating_systems": ["windows"],
      "lib_c_types": null, // Windows doesn't use lib_c_type
      "size": 48900,
      "checksum": "sha256:ghi789...",
      "last_modified": "2024-01-15T09:00:00Z"
    }
  ]
}
```

## Implementation

```rust
pub struct HttpMetadataSource {
    base_url: String,
    client: attohttpc::Session,
    cache_dir: Option<PathBuf>,
}

impl HttpMetadataSource {
    pub fn new(base_url: String) -> Self {
        let mut client = attohttpc::Session::new();
        client.header("User-Agent", "kopi-jdk-manager");

        Self {
            base_url,
            client,
            cache_dir: None,
        }
    }

    fn fetch_index(&self) -> Result<IndexFile> {
        let url = format!("{}/index.json", self.base_url);
        let response = self.client.get(&url).send()?;

        if !response.is_success() {
            return Err(KopiError::MetadataFetch(
                format!("Failed to fetch index: {}", response.status())
            ));
        }

        let index: IndexFile = response.json()?;
        Ok(index)
    }

    fn filter_files_for_platform(&self, files: Vec<IndexFileEntry>) -> Vec<IndexFileEntry> {
        let current_arch = crate::platform::get_current_architecture();
        let current_os = crate::platform::get_current_os();
        let current_libc = crate::platform::get_foojay_libc_type();

        files.into_iter()
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
                        if !lib_c_types.contains(&current_libc) {
                            return false;
                        }
                    }
                }

                true
            })
            .collect()
    }

    fn fetch_metadata_file(&self, path: &str) -> Result<Vec<JdkMetadata>> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self.client.get(&url).send()?;

        if !response.is_success() {
            return Err(KopiError::MetadataFetch(
                format!("Failed to fetch metadata file: {}", response.status())
            ));
        }

        let metadata: Vec<JdkMetadata> = response.json()?;
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

        log::info!(
            "Filtered to {} files for current platform (arch: {}, os: {}, libc: {})",
            platform_files.len(),
            crate::platform::get_current_architecture(),
            crate::platform::get_current_os(),
            crate::platform::get_foojay_libc_type()
        );

        // Fetch only metadata files relevant to this platform
        for entry in platform_files {
            match self.fetch_metadata_file(&entry.path) {
                Ok(mut metadata) => {
                    // Mark all as complete since HTTP source provides full metadata
                    for m in &mut metadata {
                        m.is_complete = true;
                    }
                    all_metadata.extend(metadata);
                }
                Err(e) => log::warn!("Failed to fetch {}: {}", entry.path, e),
            }
        }

        Ok(all_metadata)
    }

    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let mut metadata = Vec::new();

        // Fetch index file
        let index = self.fetch_index()?;

        // Filter for platform AND distribution
        let filtered_files: Vec<IndexFileEntry> = self.filter_files_for_platform(index.files)
            .into_iter()
            .filter(|entry| entry.distribution == distribution)
            .collect();

        // Fetch only the specific distribution files
        for entry in filtered_files {
            match self.fetch_metadata_file(&entry.path) {
                Ok(mut pkg_metadata) => {
                    for m in &mut pkg_metadata {
                        m.is_complete = true;
                    }
                    metadata.extend(pkg_metadata);
                }
                Err(e) => log::warn!("Failed to fetch {}: {}", entry.path, e),
            }
        }

        Ok(metadata)
    }

    fn fetch_package_details(&self, _package_id: &str) -> Result<PackageDetails> {
        // HTTP source always returns complete metadata
        Err(KopiError::InvalidOperation(
            "HTTP source provides complete metadata".to_string()
        ))
    }

    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let index = self.fetch_index()?;
        let updated = chrono::DateTime::parse_from_rfc3339(&index.updated)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok();
        Ok(updated)
    }
}
```

## Platform Filtering Behavior

The HTTP source automatically filters metadata based on the current platform:

```rust
// Example: Linux x86_64 with glibc
// Current platform:
// - Architecture: x86_64
// - OS: linux
// - lib_c_type: glibc

// Will download only:
// - temurin-linux-x64-glibc-2024-01.json
// - corretto-linux-x64-glibc-2024-01.json

// Will skip:
// - temurin-windows-x64-2024-01.json (wrong OS)
// - temurin-linux-aarch64-glibc-2024-01.json (wrong architecture)
// - temurin-linux-x64-musl-2024-01.json (wrong lib_c_type)
```

This reduces bandwidth usage by 75-90% compared to downloading all metadata files.

## Benefits of Intelligent Index

1. **Platform Filtering**: Clients only download metadata for their platform
2. **Bandwidth Efficiency**: Reduces unnecessary downloads
3. **Faster Operations**: Less data to process
4. **Checksum Verification**: Ensures data integrity
5. **Size Information**: Clients can estimate download requirements

## Hosting Options

### GitHub Pages

```yaml
# .github/workflows/deploy-metadata.yml
- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./metadata
```

### AWS S3 + CloudFront

```bash
aws s3 sync ./metadata s3://my-bucket/metadata/ --delete
aws cloudfront create-invalidation --distribution-id ABCD1234 --paths "/*"
```

### Cloudflare Pages

```bash
npx wrangler pages publish metadata --project-name=kopi-metadata
```

### Corporate Internal Server

```nginx
server {
    listen 80;
    server_name metadata.internal.company.com;
    root /var/www/metadata;

    location / {
        add_header Access-Control-Allow-Origin *;
        try_files $uri $uri/ =404;
    }
}
```

## Configuration

```toml
# HTTP/Web source is the default primary source
[metadata.sources.http]
enabled = true
# Base URL where metadata is hosted
# Default: Kopi's official metadata repository on GitHub Pages
base_url = "https://kopi-vm.github.io/metadata"
# Cache downloaded metadata locally
cache_locally = true
# No authentication needed for public web servers

# Example: Using S3 bucket
# base_url = "https://my-bucket.s3.amazonaws.com/kopi-metadata"

# Example: Using corporate internal server
# base_url = "https://internal.company.com/jdk-metadata"

# Example: Using Cloudflare Pages
# base_url = "https://metadata.kopi-vm.pages.dev"
```

## Checksum Clarification

There are two different checksums in the system:

1. **Index checksum** (in `index.json`):
   - Checksum of the JSON metadata file itself
   - Used to verify integrity of downloaded metadata files
   - Example: SHA256 of `temurin-linux-x64-glibc-2024-01.json` file content

2. **JDK checksum** (inside metadata files):
   - Checksum of the actual JDK archive (tar.gz/zip)
   - Used to verify integrity of downloaded JDK packages
   - Example: SHA256 of `OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz`
