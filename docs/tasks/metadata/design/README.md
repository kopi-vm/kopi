# Metadata Abstraction Design

This directory contains the design documentation for abstracting metadata retrieval in Kopi to support multiple sources beyond the current foojay.io API.

## Table of Contents

### Core Design Documents

1. **[Overview](01-overview.md)** - Project overview and current implementation analysis
2. **[Architecture](02-architecture.md)** - Core abstraction design and MetadataProvider architecture
3. **[Lazy Loading Design](03-lazy-loading.md)** - Detailed comparison of lazy loading approaches and adopted solution
4. **[Metadata Generator](04-metadata-generator.md)** - kopi-metadata-gen CLI tool design
5. **[Implementation Plan](05-implementation-plan.md)** - Phased implementation strategy
6. **[Configuration](06-configuration.md)** - Configuration structure for metadata sources
7. **[Installer Bundling](07-installer-bundling.md)** - Strategy for bundling metadata with installers

### Source Implementations

- **[Sources Overview](sources/README.md)** - Overview of all metadata sources
  - **[Foojay Source](sources/01-foojay.md)** - FoojayMetadataSource implementation details
  - **[HTTP/Web Source](sources/02-http-web.md)** - HttpMetadataSource for web-hosted metadata
  - **[Local Directory Source](sources/03-local-directory.md)** - LocalDirectorySource with tar.gz support

## Quick Summary

The metadata abstraction enables Kopi to fetch JDK metadata from multiple sources:

1. **HTTP/Web servers** (default primary: https://kopi-vm.github.io/metadata)
2. **Local directory** (bundled fallback: ${KOPI_HOME}/bundled-metadata)
3. **Foojay.io API** (optional for development/testing)
4. Other future sources

### Key Design Decisions

- **Trait-based abstraction**: `MetadataSource` trait for extensibility
- **Option 3 adopted**: Optional fields with resolver pattern for lazy loading
- **Synchronous I/O**: Matches existing codebase, avoids async complexity
- **Unified format**: Same JSON structure for HTTP and local sources
- **Platform filtering**: Intelligent filtering reduces bandwidth usage

### Implementation Order

1. **FoojayMetadataSource** - Wrap existing ApiClient (verify existing functionality)
2. **kopi-metadata-gen** - Generate metadata files from foojay API
3. **HttpMetadataSource** - Web-hosted metadata support (default primary)
4. **LocalDirectorySource** - Bundled metadata for offline fallback
5. **Full Integration** - Fallback logic and installer bundling