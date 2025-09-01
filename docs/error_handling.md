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
- Exit code: 20

### 3. System Errors
Permission issues, disk space, missing dependencies
- Check permissions before operations
- Validate available disk space before downloads
- Provide platform-specific guidance
- Exit codes: 13 (permission denied), 28 (disk space), 127 (command not found)

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

| Code | Meaning | Context |
|------|---------|---------|
| 2 | Invalid format/config | User error - malformed input or configuration |
| 3 | No local version | No `.kopi-version` or `.java-version` file found |
| 4 | JDK not installed | Requested JDK version is not installed |
| 13 | Permission denied | System error - insufficient permissions |
| 20 | Network error | Failed API calls or downloads |
| 28 | Disk space | Insufficient disk space for operation |
| 127 | Command not found | System error - missing dependencies |

## Best Practices

1. **Always provide actionable suggestions** - Tell users how to fix the problem
2. **Use appropriate exit codes** - Enables proper scripting and automation
3. **Include context** - Show what was attempted and why it failed
4. **Be platform-aware** - Provide platform-specific guidance when relevant
5. **Fail fast** - Check preconditions early to avoid partial operations
6. **Log appropriately** - Use debug logging for diagnostic information