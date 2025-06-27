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

## Development Commands

### Build and Run
- `cargo build` - Build the project in debug mode
- `cargo run` - Build and run the application
- `cargo build --release` - Build optimized release version
- `cargo run --release` - Run the release build

### Code Quality
- `cargo fmt` - Format code using rustfmt
- `cargo clippy` - Run linter for code improvements
- `cargo check` - Fast error checking without building

### Testing
- `cargo test` - Run all tests
- `cargo test -- --nocapture` - Run tests with stdout/stderr output
- `cargo test [test_name]` - Run specific test

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

1. `cargo fmt` - Auto-format code
2. `cargo clippy` - Check for type and linting errors
3. `cargo check` - Fast error checking without building
4. `cargo test` - Run all unit tests

Address any errors from each command before proceeding to the next. All four must pass successfully before considering the work complete.

## Architecture

### Project Structure

```
kopi/
├── src/
│   ├── api/             # API integration with foojay.io
│   ├── archive/         # Archive extraction functionality (TAR/ZIP)
│   ├── commands/        # Command implementations
│   ├── download/        # Download management and progress reporting
│   ├── models/          # Data models and structures
│   ├── security/        # Security validation and HTTPS verification
│   ├── storage/         # Storage and disk space management
│   └── version/         # Version parsing and handling
├── tests/               # Integration tests
├── docs/
│   ├── adr/             # Architecture Decision Records
│   └── reference.md     # User reference manual
└── Cargo.toml           # Project dependencies and metadata
```

Key files:
- `/src/main.rs` - Application entry point with CLI command parsing
- `/docs/adr/` - Architecture Decision Records documenting design choices
- `/docs/reference.md` - User reference manual with command documentation
- Uses `clap` v4.5.40 with derive feature for CLI argument parsing

Key architectural components:
- **Command Interface**: Subcommand-based CLI using clap derive API
- **JDK Metadata**: Fetches available JDK versions from foojay.io API
- **Version Management**: Installs and manages multiple JDK versions in `~/.kopi/jdks/<vendor>-<version>/`
- **Shell Integration**: Creates shims in `~/.kopi/bin/` for Java executables
- **Project Configuration**: Reads `.kopi-version` or `.java-version` files
- **Metadata Caching**: Stores JDK metadata in `~/.kopi/cache/metadata.json` with hybrid caching strategy

Storage locations:
- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/bin/`
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
   - Use exit code 1

2. **Network Errors**: Failed API calls or downloads
   - Implement retry logic with exponential backoff
   - Provide offline fallback when possible (cached metadata)
   - Show progress indicators for long operations

3. **System Errors**: Permission issues, disk space, missing dependencies
   - Check permissions before operations
   - Validate available disk space before downloads
   - Provide platform-specific guidance

### Error Message Format
```rust
// Use anyhow for error handling with context
use anyhow::{Context, Result};

// Add context to errors
operation()
    .context("Failed to download JDK")?;

// User-friendly error messages
bail!("JDK version '{}' not found. Run 'kopi list-remote' to see available versions.", version);
```

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
