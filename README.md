<div align="center">
  <img alt="Kopi Logo" src="https://raw.githubusercontent.com/kopi-vm/kopi/main/docs/images/logo_black.png" width="200" height="200">
</div>

# Kopi - JDK Version Manager

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kopi is a JDK version management tool written in Rust that integrates with your shell to seamlessly switch between different Java Development Kit versions. It fetches JDK metadata from foojay.io and provides a simple, fast interface similar to tools like volta, nvm, and pyenv.

## Who is this for?

Kopi is designed for:
- **Java developers** who work with multiple projects requiring different JDK versions
- **Teams** needing consistent JDK environments across development machines
- **DevOps engineers** who want to automate JDK version management in CI/CD pipelines
- **Anyone** tired of manually managing `JAVA_HOME` and `PATH` variables

## Key Features

- 🚀 **Fast Performance** - Built with Rust for minimal overhead and instant JDK switching
- 🔄 **Automatic Version Switching** - Detects and switches JDK versions based on project configuration
- 📦 **Multiple Distribution Support** - Install JDKs from various vendors (Eclipse Temurin, Amazon Corretto, Azul Zulu, GraalVM, and more)
- 🛡️ **Shell Integration** - Transparent version management through shims - no manual PATH configuration needed
- 📌 **Project Pinning** - Lock JDK versions per project using `.kopi-version` or `.java-version` files
- 🌐 **Smart Caching** - Efficient metadata caching for fast searches and offline support
- 🗑️ **Easy Uninstall** - Remove JDKs with automatic cleanup of metadata and shims
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

# Initial setup - creates directories and installs shims
kopi setup

# Add shims directory to your PATH (in ~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish)
export PATH="$HOME/.kopi/shims:$PATH"
```

### Basic Usage

```bash
# Install a JDK
kopi install 21                    # Latest JDK 21 (Eclipse Temurin by default)
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

# Uninstall a JDK
kopi uninstall temurin@21         # Remove specific version
kopi uninstall corretto --all     # Remove all Corretto versions
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
kopi search 21                    # Search for JDK 21 variants
kopi search corretto              # List all Corretto versions
kopi search 21 --lts-only         # Show only LTS versions
kopi search 21 --detailed         # Show full details
```

### Shell Environment
```bash
# Launch new shell with specific JDK
kopi shell 21                     # Launches new shell with Java 21 active

# Or use the env command for shell evaluation
eval "$(kopi env 21)"             # Bash/Zsh
kopi env 21 | source              # Fish
kopi env 21 | Invoke-Expression   # PowerShell
```

### Cache Management
```bash
kopi cache refresh                # Update metadata from foojay.io
kopi cache info                   # Show cache details
kopi cache clear                  # Remove cached metadata
```

### Shim Management
```bash
kopi shim list                    # List installed shims
kopi shim add native-image        # Add shim for GraalVM tool
kopi shim verify                  # Check shim integrity
```

## Supported Distributions

Kopi supports JDKs from multiple vendors:

- **temurin** - Eclipse Temurin (formerly AdoptOpenJDK) - default
- **corretto** - Amazon Corretto
- **zulu** - Azul Zulu
- **openjdk** - OpenJDK
- **graalvm** - GraalVM
- **dragonwell** - Alibaba Dragonwell
- **sapmachine** - SAP Machine
- **liberica** - BellSoft Liberica
- **mandrel** - Red Hat Mandrel
- **kona** - Tencent Kona
- **semeru** - IBM Semeru
- **trava** - Trava OpenJDK

Run `kopi cache list-distributions` to see all available distributions.

## Configuration

### Global Configuration
Kopi stores global settings in `~/.kopi/config.toml`:

```toml
# Default distribution for installations
default_distribution = "temurin"

# Additional custom distributions
additional_distributions = ["company-jdk"]

[storage]
# Minimum required disk space in MB
min_disk_space_mb = 500
```

### Environment Variables
- `KOPI_HOME` - Override default kopi home directory (default: `~/.kopi`)
- `HTTP_PROXY` / `HTTPS_PROXY` - Proxy configuration for downloads
- `NO_PROXY` - Hosts to bypass proxy

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
git clone https://github.com/kopi-vm/kopi.git
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

When completing any coding task:
1. Run `cargo fmt` to format code
2. Run `cargo clippy` to check for improvements  
3. Run `cargo check` for fast error checking
4. Run `cargo test --lib` to run unit tests
5. Submit a pull request

All commands must pass without errors before considering work complete.

## Documentation

- [User Reference](docs/reference.md) - Complete command reference
- [Architecture Decision Records](docs/adr/) - Design decisions and rationale

## License

Kopi is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- JDK metadata provided by [foojay.io](https://foojay.io/)
- Inspired by [volta](https://volta.sh/), [nvm](https://github.com/nvm-sh/nvm), and [pyenv](https://github.com/pyenv/pyenv)

---

Built with ❤️ by the Kopi team
