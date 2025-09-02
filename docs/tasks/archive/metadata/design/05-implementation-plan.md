# Implementation Plan

## Implementation Summary

The metadata abstraction has been partially implemented:

- **MetadataSource trait**: ✅ Fully defined and implemented
- **FoojayMetadataSource**: ✅ Implemented with lazy loading support
- **MetadataProvider**: ✅ Basic implementation with single-source support
- **kopi-metadata-gen tool**: ✅ Fully functional with all major features
- **IndexFile structures**: ✅ Implemented for metadata file organization
- **HTTP/Web source**: ❌ Not yet implemented
- **Local directory source**: ❌ Not yet implemented
- **Integration with cache**: ❌ MetadataProvider not yet used by cache module
- **Fallback logic**: ❌ Not yet implemented

## Current Status

### ✅ Phase 1: Foojay Source Implementation (COMPLETED)
1. ✅ Define `MetadataSource` trait and core abstractions
2. ✅ Refactor `ApiClient` into `FoojayMetadataSource` implementing the trait
3. ✅ Create `MetadataProvider` struct with single source support
4. ⚠️  Update existing code to use `MetadataProvider` instead of direct `ApiClient` (Partially done - provider exists but not integrated with cache)
5. ✅ Verify all existing functionality works correctly

### ✅ Phase 2: Metadata Generator Tool (COMPLETED)
1. ✅ Create `kopi-metadata-gen` as a separate binary
2. ✅ Use FoojayMetadataSource to fetch complete metadata
3. ✅ Add platform filtering and organization logic
4. ✅ Generate index.json and metadata files
5. ✅ Create standard directory structure for easy archiving

### Phase 3: HTTP/Web Source (Default Primary Source)
1. Implement `HttpMetadataSource`
2. Fetch index.json and metadata files via HTTP
3. Parse JSON metadata files
4. Implement local caching
5. Update MetadataProvider to support multiple sources
6. Set as default primary source with https://kopi-vm.github.io/metadata

### Phase 4: Local Directory Source (Bundled Fallback)
1. Implement `LocalDirectorySource`
2. Extract index.json from tar.gz archives
3. Apply platform filtering (same as HTTP source)
4. Support bundled metadata in ${KOPI_HOME}/bundled-metadata
5. Configure as automatic fallback when HTTP source fails

### Phase 5: Full Integration
1. Implement fallback logic in MetadataProvider
2. Add configuration for source priority
3. Update installer to bundle metadata archives
4. Add source management commands
5. Comprehensive testing of fallback scenarios

### Phase 6: Integration
1. Update cache module to use `MetadataProvider`
2. Modify commands to work with the abstraction
3. Add source management commands
4. Update documentation

## Expected Code Changes

### Current Code
```rust
// In cache/mod.rs
let api_client = ApiClient::new();
let metadata = api_client.fetch_all_metadata_with_options(javafx_bundled)?;
```

### After Refactoring
```rust
// In cache/mod.rs
let provider = MetadataProvider::from_config(config)?;
let metadata = provider.get_metadata()?;
```

### Affected Components
1. **cache/mod.rs**: `fetch_and_cache_metadata()` and `fetch_and_cache_distribution()`
2. **commands/install.rs**: Direct API calls for package info
3. **commands/cache.rs**: Cache refresh functionality
4. **api/client.rs**: Will be refactored into `FoojayMetadataSource`
5. **Configuration**: New metadata source configuration section

## Migration Path

1. Implement the abstraction layer (`MetadataSource` trait and `MetadataProvider`)
2. Refactor `ApiClient` to implement `MetadataSource` trait
3. Update all code that directly uses `ApiClient` to use `MetadataProvider` instead
4. Update configuration structure to support multiple metadata sources
5. Implement additional sources (local directory, HTTP/Web)
6. Remove direct dependencies on foojay-specific code from commands

## Testing Strategy

1. Unit tests for each metadata source
2. Integration tests with mock sources
3. Performance tests for caching and retrieval
4. Fallback behavior tests
5. Configuration validation tests

## Security Considerations

1. Validate checksums when available
2. Use HTTPS for remote sources
3. Sanitize filenames from local sources
4. Implement rate limiting for API sources
5. Validate metadata format and content

## Implementation Phases

From ADR-001, implement features in this order:

1. **Phase 1**: Core commands (install, list, use, current)
2. **Phase 2**: Project support (local, pin, config files) and shell command
3. **Phase 3**: Advanced features (default, doctor, prune)
4. **Phase 4**: Shell completions and enhanced integration

## Command Structure

Primary commands to implement:
- `kopi install <version>` or `kopi install <distribution>@<version>`
- `kopi use <version>` - Temporary version switching
- `kopi global <version>` - Set global default
- `kopi local <version>` - Set project-specific version
- `kopi list` - List installed JDKs
- `kopi current` - Show active JDK
- `kopi which` - Show JDK installation path

## Future Extensions

1. Metadata source priorities and fallback chains
2. Parallel fetching from multiple sources
3. Metadata transformation pipelines
4. Custom metadata source plugins
5. Metadata validation and verification
6. Source health monitoring and metrics
7. Metadata diff tool for comparing versions
8. Automated testing of generated metadata