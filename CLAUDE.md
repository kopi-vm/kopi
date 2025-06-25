# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
│   ├── main.rs          # Application entry point with CLI command parsing
│   ├── lib.rs           # Library root (if needed for integration tests)
│   ├── cli.rs           # CLI argument structures and parsing
│   ├── commands.rs      # Command implementations (or commands/ module)
│   ├── config.rs        # Configuration management
│   ├── jdk.rs           # JDK installation and metadata handling
│   ├── shell.rs         # Shell integration and shim generation
│   └── error.rs         # Error types and handling
├── tests/               # Integration tests
│   └── integration.rs   # End-to-end command tests
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
- **Version Management**: Installs and manages multiple JDK versions in `~/.kopi/jdks/<vendor>-<version>-<arch>/`
- **Shell Integration**: Creates shims in `~/.kopi/bin/` for Java executables
- **Project Configuration**: Reads `.kopi-version` or `.java-version` files
- **Metadata Caching**: Stores JDK metadata in `~/.kopi/cache/metadata.json` with hybrid caching strategy

Storage locations:
- JDKs: `~/.kopi/jdks/<vendor>-<version>-<arch>/`
- Shims: `~/.kopi/bin/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

## Key Dependencies

Core functionality:
- `clap`: CLI argument parsing with derive API
- `attohttpc`: HTTP client for foojay.io API calls
- `serde`/`serde_json`: JSON parsing for API responses and metadata

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