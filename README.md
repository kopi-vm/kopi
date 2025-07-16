# Kopi - JDK Version Manager

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kopi is a fast, modern JDK (Java Development Kit) version manager written in Rust. It allows developers to seamlessly install, manage, and switch between different JDK versions and distributions with minimal overhead.

## Who is this for?

Kopi is designed for:
- **Java developers** who work with multiple projects requiring different JDK versions
- **Teams** needing consistent JDK environments across development machines
- **DevOps engineers** who want to automate JDK version management in CI/CD pipelines
- **Anyone** tired of manually managing `JAVA_HOME` and `PATH` variables

## Key Features

- 🚀 **Fast Performance** - Built with Rust for minimal overhead and instant JDK switching
- 🔄 **Automatic Version Switching** - Detects and switches JDK versions based on project configuration
- 📦 **Multiple Distribution Support** - Install JDKs from various vendors (AdoptOpenJDK, Amazon Corretto, Azul Zulu, Eclipse Temurin, Oracle, and more)
- 🛡️ **Shell Integration** - Transparent version management through shims - no manual PATH configuration needed
- 📌 **Project Pinning** - Lock JDK versions per project using `.kopi-version` or `.java-version` files
- 🌐 **Smart Caching** - Efficient metadata caching with online/offline support
- 🔒 **Security First** - Built-in checksum validation and HTTPS verification for all downloads
- 🎯 **Cross-Platform** - Works on macOS, Linux, and Windows

## How it Works

Kopi integrates with your shell to intercept Java commands and automatically route them to the correct JDK version. It fetches available JDK distributions from [foojay.io](https://foojay.io/), a comprehensive OpenJDK discovery service.

```
┌─────────────┐     ┌──────────────┐     ┌───────────────┐
│  Your Shell │────▶│  Kopi Shims  │────▶│  Active JDK   │
└─────────────┘     └──────────────┘     └───────────────┘
                            │
                            ▼
                    ┌──────────────┐
                    │ .kopi-version│
                    └──────────────┘
```

## Quick Start

### Installation

```bash
# Install kopi (coming soon to package managers)
cargo install kopi

# Initialize shell integration
kopi shell --init >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc
```

### Basic Usage

```bash
# Install a JDK
kopi install 21                    # Latest JDK 21
kopi install temurin@17           # Specific distribution and version
kopi install corretto@11.0.21     # Exact version

# List installed JDKs
kopi list

# Set global default
kopi global 21

# Set project-specific version
cd my-project
kopi local 17                     # Creates .kopi-version file

# Show current JDK
kopi current
```

## Project Configuration

Kopi automatically detects and uses JDK versions from configuration files in your project:

### `.kopi-version` (Recommended)
```
temurin@21
```

### `.java-version` (Compatibility)
```
17.0.9
```

When you `cd` into a project directory, kopi automatically switches to the configured JDK version.

## Advanced Features

### Search Available JDKs
```bash
kopi cache search 21              # Search for JDK 21 variants
kopi cache search --vendor=corretto  # Filter by vendor
```

### Manage Storage
```bash
kopi prune                        # Remove unused JDK versions
kopi doctor                       # Diagnose configuration issues
```

### Offline Support
```bash
kopi cache update                 # Update metadata cache
kopi install 21 --offline        # Install using cached metadata
```

## Architecture

Kopi is designed with performance and reliability in mind:

- **Written in Rust** - Memory safe, fast, and efficient
- **Minimal Dependencies** - Quick installation and low overhead
- **Smart Caching** - Hybrid online/offline metadata management
- **Atomic Operations** - Safe concurrent JDK installations
- **Shell Integration** - Works with bash, zsh, fish, and PowerShell

## Comparison with Similar Tools

| Feature | Kopi | SDKMAN! | jenv | jabba |
|---------|------|---------|------|-------|
| Written in | Rust | Bash | Bash | Go |
| Performance | ⚡ Fast | Moderate | Moderate | Fast |
| Auto-switching | ✅ | ✅ | ❌ | ✅ |
| Multiple vendors | ✅ | ✅ | ❌ | ✅ |
| Offline support | ✅ | ❌ | ✅ | ✅ |
| Windows support | ✅ | ❌ | ❌ | ✅ |
| Shell integration | ✅ | ✅ | ✅ | ✅ |

## Development

### Prerequisites

- Rust toolchain (1.70+)
- sccache (recommended): `cargo install sccache`

### Building from Source

```bash
git clone https://github.com/your-org/kopi.git
cd kopi

# Development build
cargo build

# Run tests
cargo test

# Release build
cargo build --release
```

### Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Workflow

1. Run `cargo fmt` to format code
2. Run `cargo clippy` to check for improvements
3. Run `cargo test` to ensure all tests pass
4. Submit a pull request

## Documentation

- [User Reference](docs/reference.md) - Complete command reference
- [Architecture Decision Records](docs/adr/) - Design decisions and rationale
- [API Documentation](https://docs.rs/kopi) - Rust API documentation

## License

Kopi is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- JDK metadata provided by [foojay.io](https://foojay.io/)
- Inspired by [volta](https://volta.sh/), [nvm](https://github.com/nvm-sh/nvm), and [pyenv](https://github.com/pyenv/pyenv)

---

Built with ❤️ by the Kopi team