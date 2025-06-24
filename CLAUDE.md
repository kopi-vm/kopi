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

## Development Workflow

### Completing Work
When finishing any coding task, always run the following commands in order and fix any issues:

1. `cargo fmt` - Auto-format code
2. `cargo test` - Run all unit tests
3. `cargo clippy` - Check for type and linting errors
4. `cargo check` - Fast error checking without building

Address any errors from each command before proceeding to the next. All four must pass successfully before considering the work complete.

## Architecture

The project structure:
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
