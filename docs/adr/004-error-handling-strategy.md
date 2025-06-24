# 004: Error Handling Strategy

## Status
Proposed

## Context
Kopi needs a robust error handling strategy that provides:
- Clear error messages for users
- Proper error propagation between modules
- Graceful handling of common CLI scenarios (broken pipes, missing permissions)
- Debugging information when needed
- Type safety and maintainability

## Decision
We will adopt a hybrid error handling approach using:
1. **thiserror** for defining structured error types in modules
2. **color-eyre** for user-friendly error reporting in the main binary
3. **anyhow** for error context in application logic where specific error types aren't needed

### Error Architecture

#### Core Error Type
```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KopiError {
    // Version-related errors
    #[error("JDK version '{0}' is not available")]
    VersionNotAvailable(String),
    
    #[error("Invalid version format: {0}")]
    InvalidVersionFormat(String),
    
    #[error("JDK '{0}' is not installed")]
    JdkNotInstalled(String),
    
    // Download and installation errors
    #[error("Failed to download JDK: {0}")]
    Download(String),
    
    #[error("Failed to extract archive: {0}")]
    Extract(String),
    
    #[error("Checksum verification failed")]
    ChecksumMismatch,
    
    // Configuration errors
    #[error("No JDK configured for current project")]
    NoLocalVersion,
    
    #[error("Configuration file error: {0}")]
    ConfigFile(#[source] std::io::Error),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    // Shell integration errors
    #[error("Shell '{0}' is not supported")]
    UnsupportedShell(String),
    
    #[error("Failed to update PATH: {0}")]
    PathUpdate(String),
    
    #[error("Failed to create shim: {0}")]
    ShimCreation(String),
    
    // Metadata errors
    #[error("Failed to fetch metadata: {0}")]
    MetadataFetch(String),
    
    #[error("Invalid metadata format")]
    InvalidMetadata,
    
    // System errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
    
    // Wrapped standard errors
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error(transparent)]
    Http(#[from] attohttpc::Error),
    
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, KopiError>;
```

#### Module-Specific Errors
Each module can define its own error types that convert to `KopiError`:

```rust
// src/jdk/download.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Network timeout while downloading {url}")]
    Timeout { url: String },
    
    #[error("Server returned {status} for {url}")]
    BadStatus { status: u16, url: String },
    
    #[error("Download interrupted")]
    Interrupted,
}

impl From<DownloadError> for KopiError {
    fn from(err: DownloadError) -> Self {
        KopiError::Download(err.to_string())
    }
}
```

### Error Reporting

#### Main Application
```rust
// src/main.rs
use color_eyre::eyre::Result;
use std::process;

fn main() -> Result<()> {
    // Install color-eyre for better error reports
    color_eyre::install()?;
    
    // Run the application
    if let Err(e) = run() {
        // Handle broken pipe gracefully
        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
            if io_err.kind() == std::io::ErrorKind::BrokenPipe {
                process::exit(0);
            }
        }
        
        // Map errors to appropriate exit codes
        let exit_code = match e.downcast_ref::<KopiError>() {
            Some(KopiError::JdkNotInstalled(_)) => 127,  // Command not found
            Some(KopiError::PermissionDenied(_)) => 126,  // Permission denied
            Some(KopiError::InvalidVersionFormat(_)) => 2, // Invalid argument
            _ => 1,  // General error
        };
        
        process::exit(exit_code);
    }
    
    Ok(())
}
```

#### Context Addition
```rust
use anyhow::{Context, Result};

fn install_jdk(version: &str) -> Result<()> {
    let metadata = fetch_metadata(version)
        .with_context(|| format!("Failed to fetch metadata for JDK {}", version))?;
    
    let download_path = download_jdk(&metadata)
        .context("Failed to download JDK archive")?;
    
    extract_archive(&download_path)
        .with_context(|| format!("Failed to extract JDK archive at {:?}", download_path))?;
    
    Ok(())
}
```

### Exit Codes
Following standard Unix conventions:
- 0: Success
- 1: General error
- 2: Invalid argument or configuration
- 126: Permission denied
- 127: Command/version not found

## Consequences

### Positive
- Clear, user-friendly error messages with color-eyre
- Type-safe error propagation with thiserror
- Easy to add context to errors with anyhow
- Proper exit codes for shell scripting
- Graceful handling of common CLI scenarios

### Negative
- Three different error handling crates (complexity)
- Need to maintain error type conversions
- Slightly larger binary size due to color-eyre

### Neutral
- Developers need to understand when to use each approach
- Error types need to be kept in sync with actual failure modes

## Implementation Plan

1. Add dependencies to `Cargo.toml`:
   ```toml
   [dependencies]
   thiserror = "1.0"
   anyhow = "1.0"
   color-eyre = "0.6"
   ```

2. Create `src/error.rs` with core error types

3. Update `src/main.rs` to use color-eyre

4. Implement module-specific error types as needed

5. Add error context throughout the codebase

6. Write tests for error scenarios

## References
- [The Rust Programming Language - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Error Handling in Rust - A Deep Dive](https://nick.groenen.me/posts/rust-error-handling/)
- [CLI Guidelines - Errors](https://clig.dev/#errors)