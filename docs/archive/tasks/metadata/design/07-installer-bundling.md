# Installer Bundling Strategy

## Overview

Kopi installers bundle recent metadata snapshots to provide immediate offline capability. This ensures users can install and use JDKs even without internet connectivity from the first run.

## Bundling Process

### 1. Metadata Generation

Before each release:

```bash
# Generate fresh metadata from foojay API
kopi-metadata-gen generate --output ./metadata

# Create archive using standard tools
tar czf kopi-metadata-$(date +%Y-%m).tar.gz -C ./metadata .

# Verify the generated metadata
kopi-metadata-gen validate --input ./metadata
```

### 2. Installer Integration

The installer build process:

1. Includes the latest metadata archive in the package
2. Extracts it to `${KOPI_HOME}/bundled-metadata/` during installation
3. Sets appropriate permissions on the extracted files

### 3. Directory Structure

After installation:

```
${KOPI_HOME}/
├── bin/
│   ├── kopi
│   └── kopi-shim
├── bundled-metadata/                    # Extracted during installation
│   ├── index.json
│   └── jdks/
│       ├── temurin-linux-x64-glibc-2024-01.json
│       ├── corretto-linux-x64-glibc-2024-01.json
│       └── ...
├── cache/
│   └── metadata.json                    # Runtime cache
└── config.toml
```

## Update Strategy

### Release Updates

- Each Kopi release includes metadata from the release date
- Metadata typically remains valid for months
- Users get the latest JDK versions available at release time

### Runtime Updates

1. **Normal Operation**: Fetches fresh metadata from HTTP source
2. **Fallback**: Uses bundled metadata if HTTP fails
3. **Cache**: Stores fetched metadata for configured duration

### Version Compatibility

- Bundled metadata format matches the Kopi version
- Older Kopi versions can read newer metadata formats
- Forward compatibility through version field in index.json

## Fallback Behavior

```rust
// Simplified fallback logic
fn get_metadata() -> Result<MetadataCache> {
    // 1. Try HTTP source (default)
    if let Ok(metadata) = http_source.fetch_all() {
        cache.store(metadata);
        return Ok(metadata);
    }

    // 2. Check cache validity
    if let Some(cached) = cache.get_valid() {
        return Ok(cached);
    }

    // 3. Fall back to pre-extracted bundled metadata
    // Already available at ${KOPI_HOME}/bundled-metadata/
    local_source.fetch_all()
}
```

## Benefits

1. **Immediate Availability**: Users can install JDKs right after Kopi installation
2. **Offline Capability**: Works in air-gapped environments
3. **Network Resilience**: Handles temporary connectivity issues
4. **Predictable Behavior**: Known-good metadata always available
5. **Fast First Run**: No initial download required

## Implementation Checklist

- [ ] Modify installer build scripts to include metadata archive
- [ ] Add installer logic to extract bundled metadata to `${KOPI_HOME}/bundled-metadata/`
- [ ] Ensure proper permissions are set on extracted files
- [ ] Set up CI/CD to generate metadata before releases
- [ ] Document bundled metadata location for users
- [ ] Test offline installation scenarios
- [ ] Verify fallback behavior in various network conditions
- [ ] Clean up old bundled metadata on upgrades

## Platform-Specific Considerations

### Windows

- Install to `%LOCALAPPDATA%\kopi\bundled-metadata\`
- Handle path separators correctly

### macOS

- Install to `~/Library/Application Support/kopi/bundled-metadata/`
- Consider code signing requirements

### Linux

- Install to `~/.kopi/bundled-metadata/`
- Respect XDG base directory specification if set

## Archive Naming Convention

```
kopi-metadata-YYYY-MM.tar.gz
```

Where:

- `YYYY`: Four-digit year
- `MM`: Two-digit month

This allows multiple archives in the same directory without conflicts.

## Maintenance

### Regular Updates

- Generate new metadata monthly or before releases
- Monitor foojay API for schema changes
- Test compatibility with older Kopi versions

### Emergency Updates

If critical JDK updates are released:

1. Generate new metadata archive
2. Update HTTP source immediately
3. Consider patch release with updated bundle

## Security Considerations

1. **Integrity**: Verify metadata archives during build
2. **Authenticity**: Consider signing metadata archives
3. **Updates**: Ensure HTTP source uses HTTPS
4. **Validation**: Check metadata format before use
