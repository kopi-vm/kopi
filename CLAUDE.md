# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Documentation Language Policy

All documentation output in this project must be written in English, including:
- Code comments
- Commit messages
- Architecture Decision Records (ADRs)
- README files
- API documentation
- Error messages
- User-facing documentation
- Test descriptions
- TODO comments
- Any other written documentation

## Project Overview

Kopi is a JDK version management tool written in Rust that integrates with your shell to seamlessly switch between different Java Development Kit versions. It fetches JDK metadata from foojay.io and provides a simple, fast interface similar to tools like volta, nvm, and pyenv.

Key features:
- Automatic JDK version switching based on project configuration
- Multiple JDK vendor support (AdoptOpenJDK, Amazon Corretto, Azul Zulu, etc.)
- Shell integration via shims for transparent version management
- Project-specific JDK pinning via `.kopi-version` or `.java-version` files
- Fast performance using Rust

## Development Setup

### Prerequisites
Before starting development, ensure you have the following installed:
- Rust toolchain (via rustup)
- sccache for faster compilation: `cargo install sccache`

### sccache Configuration
This project uses sccache to cache compilation artifacts and speed up build times. The configuration is already set in `.cargo/config.toml`.

To verify sccache is working:
```bash
# Check if sccache is installed
which sccache

# View sccache statistics
sccache --show-stats
```

For advanced sccache configuration (e.g., using cloud storage backends):
```bash
# Example: Configure S3 backend for CI/CD
export SCCACHE_BUCKET=my-sccache-bucket
export SCCACHE_REGION=us-east-1

# Example: Set cache size limit (default: 10GB)
export SCCACHE_CACHE_SIZE="20G"
```

## Development Commands

### Build and Run
- `cargo build` - Build the project in debug mode (fastest compilation)
- `cargo run` - Build and run the application
- `cargo build --profile release-fast` - Fast release build for development
- `cargo build --release` - Build optimized release version for production
- `cargo run --release` - Run the release build

### Code Quality
- `cargo fmt` - Format code using rustfmt
- `cargo clippy` - Run linter for code improvements
- `cargo check` - Fast error checking without building

### Testing
- `cargo test` - Run all tests with optimized test profile
- `cargo test --lib` - Run only unit tests (fastest)
- `cargo test -- --nocapture` - Run tests with stdout/stderr output
- `cargo test [test_name]` - Run specific test
- `cargo test --features perf-tests` - Run performance tests (usually ignored)

**Test Organization**:
- Unit tests should be placed in the same file as the code being tested using `#[cfg(test)]`
- Integration tests go in the `tests/` directory
- Example:
```rust
// src/jdk.rs
pub fn parse_version(version: &str) -> Result<Version> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("11.0.2"), Ok(Version::new(11, 0, 2)));
    }
}
```

## Development Workflow

### Completing Work
When finishing any coding task, always run the following commands in order and fix any issues:

1. Verify sccache is working: `sccache --show-stats` (should show cache hits after first build)
2. `cargo fmt` - Auto-format code
3. `cargo clippy` - Check for type and linting errors
4. `cargo check` - Fast error checking without building
5. `cargo test --lib` - Run unit tests (faster than full test suite)

Address any errors from each command before proceeding to the next. All must pass successfully before considering the work complete.

If sccache is not installed, you'll see an error when running cargo commands. Install it with:
```bash
cargo install sccache
```

### Performance Considerations
- **Test execution** is limited to 4 threads by default (configured in `.cargo/config.toml`)
- **Incremental compilation** is enabled for faster rebuilds
- **Build profiles** are optimized:
  - `dev` profile: Dependencies are optimized at level 2
  - `test` profile: Tests run with optimization level 1 and limited debug info
  - `release-fast` profile: Fast release builds without LTO for development

## Architecture

### Project Structure

```
kopi/
├── src/
│   ├── api/             # API integration with foojay.io
│   ├── archive/         # Archive extraction functionality (TAR/ZIP)
│   ├── bin/             # Binary executables (kopi-shim)
│   ├── cache/           # Metadata caching functionality
│   ├── commands/        # Command implementations
│   ├── download/        # Download management and progress reporting
│   ├── error/           # Error handling and formatting
│   ├── models/          # Data models and structures
│   ├── platform/        # Platform-specific functionality
│   ├── search/          # JDK search functionality
│   ├── security/        # Security validation and HTTPS verification
│   ├── shim/            # Shim management
│   ├── storage/         # Storage and disk space management
│   └── version/         # Version parsing and handling
├── tests/               # Integration tests
│   └── common/          # Common test utilities
├── benches/             # Performance benchmarks
├── benchmarks/          # Benchmark results and history
├── docs/
│   ├── adr/             # Architecture Decision Records
│   ├── reviews/         # Code and design reviews
│   └── tasks/           # Task planning documents
├── scripts/             # Development and CI scripts
└── Cargo.toml           # Project dependencies and metadata
```

Key files:
- `/src/main.rs` - Application entry point with CLI command parsing
- `/src/lib.rs` - Library entry point for shared functionality
- `/src/config.rs` - Configuration management
- `/src/bin/kopi-shim.rs` - Shim binary for transparent JDK switching
- `/docs/adr/` - Architecture Decision Records documenting design choices
- `/docs/reference.md` - User reference manual with command documentation
- Uses `clap` v4.5.40 with derive feature for CLI argument parsing

Key architectural components:
- **Command Interface**: Subcommand-based CLI using clap derive API
- **JDK Metadata**: Fetches available JDK versions from foojay.io API
- **Version Management**: Installs and manages multiple JDK versions in `~/.kopi/jdks/<vendor>-<version>/`
- **Shell Integration**: Creates shims in `~/.kopi/shims/` for Java executables
- **Project Configuration**: Reads `.kopi-version` (native format with `@` separator) or `.java-version` (compatibility)
- **Metadata Caching**: Stores JDK metadata in `~/.kopi/cache/metadata.json` with hybrid caching strategy

Storage locations:
- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/shims/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

Configuration System:
- Global config stored at `~/.kopi/config.toml`
- Loaded automatically by components via `KopiConfig::load()`
- Uses sensible defaults when config file is missing

## Key Dependencies

Core functionality:
- `clap`: CLI argument parsing with derive API
- `attohttpc`: HTTP client for foojay.io API calls
- `serde`/`serde_json`: JSON parsing for API responses and metadata
- `indicatif`: Progress bars and spinners for download feedback

Archive handling:
- `tar`: Extract JDK tar archives
- `zip`: Extract JDK zip archives
- `tempfile`: Safe temporary file handling during downloads

Platform integration:
- `dirs`: Platform-specific directory paths
- `which`: Find executables in PATH
- `walkdir`: Recursive directory traversal
- Platform-specific: `winreg` (Windows registry), `junction` (Windows symlinks)

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

## Error Handling Guidelines

### Error Types
1. **User Errors**: Invalid input, missing arguments, or incorrect usage
   - Return clear, actionable error messages
   - Include examples of correct usage
   - Exit codes: 2 (invalid format/config), 3 (no local version), 4 (JDK not installed)

2. **Network Errors**: Failed API calls or downloads
   - Implement retry logic with exponential backoff
   - Provide offline fallback when possible (cached metadata)
   - Show progress indicators for long operations
   - Exit code: 20

3. **System Errors**: Permission issues, disk space, missing dependencies
   - Check permissions before operations
   - Validate available disk space before downloads
   - Provide platform-specific guidance
   - Exit codes: 13 (permission denied), 28 (disk space), 127 (command not found)

### Error Message Format
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

### Error Context System
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

Note: Most error handling is done automatically by the framework. When creating new errors, simply use the appropriate `KopiError` variant and the context system will handle the rest.

## Developer Principles

### Memory Safety Over Micro-optimization
- Prioritize memory safety and correctness over micro-optimizations
- Accept reasonable overhead (e.g., cloning small strings) to avoid memory leaks
- Follow Rust's ownership model strictly - avoid `unsafe` code and memory leaks from techniques like `Box::leak()`
- When faced with lifetime complexity, prefer simpler solutions that may use slightly more memory but are correct
- Example: Clone strings for HTTP headers instead of using `Box::leak()` to create static references

### Code Clarity
- Write clear, readable code that is easy to understand and maintain
- Use descriptive variable and function names
- Add comments for complex logic, but prefer self-documenting code
- Structure code to minimize cognitive load for future developers

### Clean Code Maintenance
- Remove unused variables, parameters, and struct members promptly
- When refactoring, trace through all callers to eliminate unnecessary parameters
- Keep structs lean by removing fields that are no longer used
- Use `cargo clippy` to identify unused code elements
- Example: If a function parameter like `arch` is no longer used in the implementation, remove it from the function signature and update all callers

### Prefer Functions Over Structs Without State
- When there's no state to manage, prefer implementing functionality as standalone functions rather than defining structs
- Only create structs when you need to maintain state, implement traits, or group related data
- This keeps the code simpler and more straightforward
- Example: For utility operations like file validation or string parsing, use functions directly instead of creating a struct with methods

### External API Testing
- When writing code that calls external Web APIs, implement at least one unit test that includes the actual JSON response obtained from calling the API with curl
- Store the JSON response as a string within the test code
- This ensures that the parsing logic is tested against real API responses
- Example:
```rust
#[test]
fn test_parse_foojay_api_response() {
    // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/packages?version=21
    let json_response = r#"{
        "result": [
            {
                "id": "abcd1234",
                "distribution": "temurin",
                "major_version": 21,
                ...
            }
        ]
    }"#;
    
    let packages: Vec<Package> = serde_json::from_str(json_response).unwrap();
    assert_eq!(packages[0].distribution, "temurin");
}
```

### Avoid Generic "Manager" Naming
- When the name "manager" appears in file names, structs, traits, or similar constructs, consider more specific and descriptive alternatives
- "Manager" is often too abstract and doesn't clearly communicate the responsibility
- Choose names that describe what the component actually does
- Examples of better alternatives:
  - `FileManager` → `FileSystem`, `FileStore`, `FileRepository`
  - `ConnectionManager` → `ConnectionPool`, `ConnectionFactory`
  - `TaskManager` → `TaskScheduler`, `TaskExecutor`, `TaskQueue`
  - `ShimManager` → `ShimInstaller`, `ShimRegistry`, `ShimProvisioner`
- This principle helps maintain code clarity and makes the codebase more intuitive

### Avoid Vague "Util" or "Utils" Naming
- Never use "util" or "utils" in directory names, file names, class names, or variable names
- These terms are too generic and don't clearly convey the purpose or responsibility
- Always choose specific names that describe the actual functionality
- Examples of better alternatives:
  - `utils/strings.rs` → `string_operations.rs`, `text_processing.rs`, `string_formatter.rs`
  - `FileUtils` → `FileOperations`, `FileSystem`, `PathValidator`
  - `DateUtil` → `DateFormatter`, `DateParser`, `TimeCalculator`
  - `CommonUtils` → Split into specific modules based on functionality
  - `util_function()` → Name based on what it does: `validate_input()`, `format_output()`
- This principle ensures code is self-documenting and responsibilities are clear
