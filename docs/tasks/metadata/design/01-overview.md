# Metadata Abstraction Overview

## Implementation Status

The metadata abstraction has been **partially implemented**. The core abstractions (`MetadataSource` trait, `MetadataProvider`, `FoojayMetadataSource`) and the `kopi-metadata-gen` tool are complete, but integration with the existing cache module and additional sources (HTTP/Web, Local Directory) are not yet implemented.

## Project Goal

This project aims to abstract metadata retrieval in Kopi to support multiple sources beyond the current foojay.io API. The abstraction will enable fetching metadata from:

1. HTTP/Web servers (default: https://kopi-vm.github.io/metadata)
2. Foojay.io API (fallback source)
3. Local directory containing tar.gz archives
4. Other future sources

## Current Implementation Analysis

### Data Flow
1. `ApiClient` fetches metadata from foojay.io API
2. API responses are converted to `JdkMetadata` structs
3. Metadata is stored in `MetadataCache` and persisted as JSON
4. Commands access metadata through the cache module
5. Cache implements TTL and refresh mechanisms

### Key Components
- `JdkMetadata`: Core metadata structure for JDK packages
- `MetadataCache`: Stores all metadata with distribution information
- `ApiClient`: Handles HTTP requests to foojay.io
- Cache storage: JSON serialization to disk

### Current Limitations
- Tightly coupled to foojay.io API
- No support for offline metadata sources
- Limited flexibility for custom JDK distributions
- All metadata operations require internet connectivity

## Design Goals

1. **Clean Architecture**: Refactor existing code to use the new abstraction layer
2. **Extensibility**: Easy to add new metadata sources
3. **Configuration**: Sources can be configured via `config.toml`
4. **Consistency**: All sources provide the same `JdkMetadata` format
5. **Performance**: Efficient caching and retrieval mechanisms
6. **Backward Compatibility**: Existing functionality must continue to work
7. **Offline Support**: Enable metadata access without internet connection
8. **Platform Optimization**: Download only platform-relevant metadata

## Expected Benefits

- **Flexibility**: Support for corporate environments with restricted internet
- **Reliability**: Automatic fallback to bundled metadata when HTTP source fails
- **Offline Support**: Works immediately after installation without internet
- **Performance**: Local sources eliminate network latency
- **Customization**: Support for private JDK distributions
- **Testing**: Easier to test with mock metadata sources

## Fallback Strategy

1. **Primary**: HTTP/Web source (https://kopi-vm.github.io/metadata)
2. **Fallback**: Local directory with bundled metadata from installer
3. **Development**: Foojay API for generating fresh metadata

This ensures Kopi always has access to JDK metadata, even in offline environments.