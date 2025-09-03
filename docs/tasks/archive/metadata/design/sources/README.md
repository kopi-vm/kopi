# Metadata Sources

This directory contains implementation details for each metadata source type.

## Available Sources

### 1. [Foojay Source](01-foojay.md) (Development/Testing)

- Wraps existing `ApiClient` functionality
- Used for generating metadata files
- Two-phase loading (list API + details API)
- Returns metadata with `is_complete: false`
- Implements lazy loading via `fetch_package_details`
- Not used in production fallback chain

### 2. [HTTP/Web Source](02-http-web.md) (Default Primary Source)

- Fetches from any static web server
- Default URL: https://kopi-vm.github.io/metadata
- Supports GitHub Pages, S3, CDN, etc.
- Platform-specific filtering via index.json
- Returns metadata with `is_complete: true`
- Caching for performance

### 3. [Local Directory Source](03-local-directory.md) (Bundled Fallback)

- Reads from tar.gz archives bundled with installer
- Default location: ${KOPI_HOME}/bundled-metadata
- Automatic fallback when HTTP source fails
- Platform-specific filtering at extraction time
- Returns metadata with `is_complete: true`
- No lazy loading needed

## Source Characteristics Comparison

| Source          | Internet Required | Lazy Loading | Platform Filtering | Use Case                        |
| --------------- | ----------------- | ------------ | ------------------ | ------------------------------- |
| HTTP/Web        | Yes               | No           | Client-side        | Default primary source          |
| Local Directory | No                | No           | Client-side        | Bundled fallback                |
| Foojay          | Yes               | Yes          | Server-side        | Development/metadata generation |

## Common Features

All sources must:

1. Implement the `MetadataSource` trait
2. Return metadata in `JdkMetadata` format
3. Support platform filtering (reduce unnecessary data)
4. Handle errors gracefully
5. Provide source availability checking

## Source Selection

The `MetadataProvider` selects sources based on:

1. Configuration priority
2. Source availability
3. Fallback strategy
4. Performance characteristics
