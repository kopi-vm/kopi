# Configuration

## Metadata Source Configuration

The metadata abstraction supports multiple sources configured via `config.toml`:

```toml
[metadata]
# Primary source for metadata
primary_source = "http"
# Fallback source when primary is unavailable
fallback_source = "local"

# Cache configuration
[metadata.cache]
max_age_hours = 720
auto_refresh = true

# HTTP/Web source (default)
[metadata.sources.http]
enabled = true
# Base URL where metadata is hosted
# Can be any static web server: GitHub Pages, S3, CDN, etc.
base_url = "https://kopi-vm.github.io/metadata"
# Cache downloaded metadata locally
cache_locally = true
# No authentication needed for public web servers

# Local directory source (fallback)
[metadata.sources.local]
enabled = true
# Default: Look for bundled metadata in installation directory
directory = "${KOPI_HOME}/bundled-metadata"
# Pattern for matching tar.gz files containing metadata
archive_pattern = "*.tar.gz"
# Whether to cache extracted metadata
cache_extracted = true

# Archives must contain:
# - index.json (same format as HTTP source)
# - Subdirectories with metadata JSON files
# Bundled with installer at: ${KOPI_HOME}/bundled-metadata/kopi-metadata-YYYY-MM.tar.gz

# Foojay API source (optional - for development/testing)
[metadata.sources.foojay]
enabled = false
base_url = "https://api.foojay.io/disco"
timeout_secs = 30
# Not used as fallback - only enable for development/testing

# Example: Using S3 bucket
# base_url = "https://my-bucket.s3.amazonaws.com/kopi-metadata"

# Example: Using corporate internal server
# base_url = "https://internal.company.com/jdk-metadata"

# Example: Using Cloudflare Pages
# base_url = "https://metadata.kopi-vm.pages.dev"
```

## Configuration Priority

1. **Primary Source**: HTTP/Web source (always tried first)
2. **Fallback Source**: Local directory with bundled metadata
3. **Cache**: Used to avoid repeated downloads
4. **Foojay**: Only used when explicitly enabled for development/testing

## Source-Specific Settings

### Foojay Source
- `base_url`: API endpoint
- `timeout_secs`: Request timeout
- `retry_attempts`: Number of retries on failure

### Local Directory Source
- `directory`: Path to directory containing tar.gz archives
- `archive_pattern`: Glob pattern for archive files
- `cache_extracted`: Whether to cache extracted metadata

### HTTP/Web Source
- `base_url`: Base URL of the web server
- `cache_locally`: Whether to cache downloaded metadata
- `timeout_secs`: Request timeout

## Environment Variables

Configuration can be overridden with environment variables using the `KOPI_` prefix and double underscore (`__`) for nested fields:

```bash
# Override primary source
export KOPI_METADATA__PRIMARY_SOURCE=local

# Override fallback source  
export KOPI_METADATA__FALLBACK_SOURCE=foojay

# Override specific source settings
export KOPI_METADATA__SOURCES__FOOJAY__ENABLED=true
export KOPI_METADATA__SOURCES__FOOJAY__BASE_URL=https://api.foojay.io/disco
export KOPI_METADATA__SOURCES__LOCAL__DIRECTORY=/custom/path
export KOPI_METADATA__SOURCES__HTTP__BASE_URL=https://custom.example.com/metadata

# Override cache settings
export KOPI_METADATA__CACHE__MAX_AGE_HOURS=168
export KOPI_METADATA__CACHE__AUTO_REFRESH=false
```

The pattern follows: `KOPI_<SECTION>__<SUBSECTION>__<FIELD>`
- Single underscore (`_`) after KOPI prefix
- Double underscore (`__`) for nested field separation
- All uppercase letters
- Boolean values: `true` or `false`
- Numeric values: parsed automatically

## Default Configuration

If no configuration is provided, the following defaults are used:

```rust
impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            primary_source: "http".to_string(),
            fallback_source: "local".to_string(),
            cache: CacheConfig {
                max_age_hours: 720,
                auto_refresh: true,
            },
            sources: HashMap::from([
                ("http".to_string(), SourceConfig::Http(HttpConfig {
                    enabled: true,
                    base_url: "https://kopi-vm.github.io/metadata".to_string(),
                    cache_locally: true,
                    timeout_secs: 30,
                })),
                ("local".to_string(), SourceConfig::Local(LocalConfig {
                    enabled: true,
                    directory: "${KOPI_HOME}/bundled-metadata".to_string(),
                    archive_pattern: "*.tar.gz".to_string(),
                    cache_extracted: true,
                })),
                ("foojay".to_string(), SourceConfig::Foojay(FoojayConfig {
                    enabled: false,
                    base_url: "https://api.foojay.io/disco".to_string(),
                    timeout_secs: 30,
                })),
            ]),
        }
    }
}
```

## Configuration Validation

The configuration is validated at startup:

1. **Source Availability**: At least one source must be enabled
2. **Primary Source**: Must be one of the enabled sources
3. **Path Validation**: Local directory paths must be valid
4. **URL Validation**: HTTP URLs must be well-formed

## Examples

### Offline Environment
```toml
[metadata]
primary_source = "local"

[metadata.sources.local]
enabled = true
directory = "/opt/kopi-metadata"

[metadata.sources.foojay]
enabled = false
```

### Corporate Network with Fallback
```toml
[metadata]
primary_source = "http"

[metadata.sources.http]
enabled = true
base_url = "https://internal.company.com/jdk-metadata"

[metadata.sources.foojay]
enabled = true  # Fallback to public API if internal server is unavailable
```

### Default Configuration with Bundled Fallback
```toml
[metadata]
primary_source = "http"
fallback_source = "local"

[metadata.sources.http]
enabled = true
base_url = "https://kopi-vm.github.io/metadata"

[metadata.sources.local]
enabled = true
directory = "${KOPI_HOME}/bundled-metadata"
```

### Development/Testing
```toml
[metadata]
primary_source = "local"

[metadata.sources.local]
enabled = true
directory = "./test-metadata"
cache_extracted = false  # Always re-read for testing

[metadata.cache]
max_age_hours = 0  # Disable caching
```

### Environment Variable Examples

```bash
# Use local metadata for testing
export KOPI_METADATA__PRIMARY_SOURCE=local
export KOPI_METADATA__SOURCES__LOCAL__DIRECTORY=/tmp/test-metadata

# Enable foojay for development
export KOPI_METADATA__SOURCES__FOOJAY__ENABLED=true
export KOPI_METADATA__PRIMARY_SOURCE=foojay

# Use custom HTTP source
export KOPI_METADATA__SOURCES__HTTP__BASE_URL=https://internal.company.com/jdk-metadata

# Disable auto-refresh for debugging
export KOPI_METADATA__CACHE__AUTO_REFRESH=false
export KOPI_METADATA__CACHE__MAX_AGE_HOURS=0
```