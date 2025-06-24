# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kopi is a JDK version management tool written in Rust that integrates with your shell to seamlessly switch between different Java Development Kit versions. It fetches JDK metadata from foojay.io and provides a simple, fast interface similar to tools like volta, nvm, and pyenv.

Key features:
- Automatic JDK version switching based on project configuration
- Multiple JDK vendor support (AdoptOpenJDK, Amazon Corretto, Azul Zulu, etc.)
- Shell integration via shims for transparent version management
- Project-specific JDK pinning
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

Address any errors from each command before proceeding to the next. All three must pass successfully before considering the work complete.

## Architecture

The project structure:
- `/src/main.rs` - Application entry point with CLI command parsing
- `/docs/adr/` - Architecture Decision Records documenting design choices
- Uses `clap` v4.5.40 with derive feature for CLI argument parsing

Key architectural components:
- **Command Interface**: Subcommand-based CLI using clap derive API
- **JDK Metadata**: Fetches available JDK versions from foojay.io API
- **Version Management**: Installs and manages multiple JDK versions in `~/.kopi/`
- **Shell Integration**: Creates shims in `~/.kopi/bin/` for Java executables
- **Project Configuration**: Reads `.kopi-version` or `.java-version` files

Dependencies:
- `attohttpc`: HTTP client for foojay.io API calls
- `serde`/`serde_json`: JSON parsing for API responses
- `tar`/`zip`: Archive extraction for JDK downloads
- `tempfile`: Safe temporary file handling during downloads
- Platform-specific: `winreg` (Windows), `junction` (Windows symlinks)
