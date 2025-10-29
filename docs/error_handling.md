# Error Handling Guidelines

## Error Types

### 1. User Errors

Invalid input, missing arguments, or incorrect usage

- Return clear, actionable error messages
- Include examples of correct usage
- Exit codes: 2 (invalid format/config), 3 (no local version), 4 (JDK not installed)

### 2. Network Errors

Failed API calls or downloads

- Implement retry logic with exponential backoff
- Provide offline fallback when possible (cached metadata)
- Show progress indicators for long operations
- Covers `KopiError::NetworkError`, HTTP transport errors, and metadata fetch failures (`KopiError::Http`, `KopiError::MetadataFetch`)
- Exit code: 20

### 3. System Errors

Permission issues, disk space, missing dependencies

- Check permissions before operations
- Validate available disk space before downloads
- Provide platform-specific guidance
- Exit codes: 13 (permission denied), 28 (disk space), 127 (command not found)

### 4. Locking Errors

Failure to coordinate cross-process access (advisory or fallback locking)

- Use dedicated `KopiError::LockingAcquire`, `KopiError::LockingTimeout`, and `KopiError::LockingRelease` variants
- Include scope labels (`installation temurin-21`, `cache writer`, etc.) and original IO details
- Let the controller downgrade to fallback automatically; surface INFO logs for downgrade decisions
- Hygiene failures should log WARN but not abort the CLI; acquisition failures bubble up to commands with actionable text
- Distinguish user cancellations with `KopiError::LockingCancelled` so scripts can differentiate manual interrupts from timeouts
- Timeout errors include the resolved timeout value and its provenance (CLI flag, environment variable, configuration file, or built-in default) so users can see which override to adjust
- Encourage users to tune lock behaviour via `--lock-timeout`, `KOPI_LOCK_TIMEOUT`, or `locking.timeout` in the config file

## Error Message Format

```rust
// Use thiserror for strongly-typed error handling
use thiserror::Error;

// Define specific error types with clear messages
#[derive(Error, Debug)]
pub enum KopiError {
    #[error("Failed to download JDK: {0}")]
    Download(String),

    #[error("JDK version '{0}' is not available")]
    VersionNotAvailable(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error(transparent)]
    Http(#[from] attohttpc::Error),
}

// Return specific error types
operation()
    .map_err(|e| KopiError::Download(e.to_string()))?;
```

## Error Context System

The codebase includes an `ErrorContext` system that provides helpful suggestions and details based on error types:

```rust
use crate::error::{ErrorContext, format_error_with_color};

// Errors are automatically enriched with context when displayed
match result {
    Err(e) => {
        let context = ErrorContext::new(&e);
        eprintln!("{}", format_error_with_color(&e, std::io::stderr().is_terminal()));
        std::process::exit(get_exit_code(&e));
    }
    Ok(_) => {}
}
```

The `ErrorContext` system automatically provides:

- User-friendly suggestions for common errors (e.g., "Run 'kopi cache search' to see available versions")
- Platform-specific guidance (e.g., different commands for Windows vs Unix)
- Detailed error information when available
- Proper exit codes based on error type (see `get_exit_code`)

**Note**: Most error handling is done automatically by the framework. When creating new errors, simply use the appropriate `KopiError` variant and the context system will handle the rest.

## Exit Codes Summary

| Code | Meaning               | Context                                                          |
| ---- | --------------------- | ---------------------------------------------------------------- |
| 1    | General error         | Default exit code for unspecified errors                         |
| 2    | Invalid format/config | User error - malformed input, configuration, or validation error |
| 3    | No local version      | No `.kopi-version` or `.java-version` file found                 |
| 4    | JDK not installed     | Requested JDK version is not installed                           |
| 5    | Tool not found        | Required tool (e.g., java, javac) not found in JDK               |
| 6    | Shell detection error | Failed to detect the current shell                               |
| 7    | Unsupported shell     | Shell is not supported by Kopi                                   |
| 13   | Permission denied     | System error - insufficient permissions                          |
| 17   | Already exists        | Resource already exists (e.g., JDK already installed)            |
| 20   | Network error         | Failed API calls, downloads, or metadata fetching                |
| 28   | Disk space            | Insufficient disk space for operation                            |
| 75   | Lock wait cancelled   | User interrupted lock acquisition (e.g., Ctrl-C)                 |
| 127  | Command not found     | Kopi command not found or shell not found                        |

Lock acquisition timeouts (`KopiError::LockingTimeout`) currently map to exit code `1` because the operation exhausted the configured deadline. Recommend documenting the elapsed wait time and pointing users to the timeout overrides when raising this error.

## Best Practices

1. **Always provide actionable suggestions** - Tell users how to fix the problem
2. **Use appropriate exit codes** - Enables proper scripting and automation
3. **Include context** - Show what was attempted and why it failed
4. **Be platform-aware** - Provide platform-specific guidance when relevant
5. **Fail fast** - Check preconditions early to avoid partial operations
6. **Log appropriately** - Use debug logging for diagnostic information
7. **Record scope metadata** - Include lock scope, backend (advisory/fallback), and lease identifiers in logs for contention analysis
