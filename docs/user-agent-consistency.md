# User-Agent Consistency Implementation

## Overview

Implemented a centralized User-Agent management system to ensure consistent HTTP client identification across all Kopi features.

## Changes Made

### 1. Created `src/user_agent.rs` Module
- Centralized module for all User-Agent string generation
- Uses `env!("CARGO_PKG_VERSION")` to embed version from Cargo.toml at compile time
- Provides specific functions for each feature: `api_client()`, `metadata_client()`, `download_client()`, `doctor_client()`
- Generic `for_feature()` function for future extensibility

### 2. Updated User-Agent Format
All User-Agent strings now follow the consistent format: `kopi/{feature}/{version}`

Examples:
- API Client: `kopi/api/0.1.0`
- Metadata Client: `kopi/metadata/0.1.0`
- Download Client: `kopi/download/0.1.0`
- Doctor Client: `kopi/doctor/0.1.0`

### 3. Updated All HTTP Clients

#### `src/api/client.rs`
- Before: `kopi/{version}`
- After: `kopi/api/{version}`

#### `src/metadata/http.rs`
- Before: `kopi-jdk-manager` (hardcoded)
- After: `kopi/metadata/{version}`

#### `src/download/client.rs`
- Before: `kopi/0.1.0` (hardcoded)
- After: `kopi/download/{version}`

#### `src/doctor/checks/network.rs`
- Before: `kopi-doctor/{version}` (two occurrences)
- After: `kopi/doctor/{version}`

## Benefits

1. **Consistency**: All HTTP clients now use the same format
2. **Maintainability**: Version is automatically updated from Cargo.toml
3. **Traceability**: Easy to identify which Kopi feature is making requests
4. **Future-proof**: Easy to add new features with consistent User-Agent strings

## Testing

All existing tests pass with the new User-Agent implementation:
- Unit tests for the user_agent module
- HTTP metadata tests
- All other existing tests continue to pass