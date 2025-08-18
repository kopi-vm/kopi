<div align="center">
  <img alt="Kopi Logo" src="https://kopi-vm.github.io/assets/logo_black.png" width="200" height="200">
</div>

# Kopi - JDK Version Manager

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kopi is a JDK version management tool written in Rust that integrates with your shell to seamlessly switch between different Java Development Kit versions. It uses a multi-source metadata system with pre-generated metadata files and the Foojay API for comprehensive JDK availability, providing a simple, fast interface similar to tools like volta, nvm, and pyenv.

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
- 🎯 **Cross-Platform** - Works on macOS (Intel & Apple Silicon), Linux, and Windows

## How it Works

Kopi integrates with your shell to intercept Java commands and automatically route them to the correct JDK version. It uses a multi-source metadata system that fetches JDK information from both pre-generated metadata files hosted at kopi-vm.github.io for optimal performance and the Foojay API for real-time availability of the latest JDK releases.

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

#### Using Homebrew (macOS)

```bash
# Install kopi from the official tap
brew install kopi-vm/tap/kopi
```

#### Using PPA (Ubuntu/Debian)

```bash
# Import GPG key
curl -fsSL https://keyserver.ubuntu.com/pks/lookup?op=get\&search=0xD2AC04A5A34E9BE3A8B32784F507C6D3DB058848 | \
  gpg --dearmor | \
  sudo tee /usr/share/keyrings/kopi-archive-keyring.gpg > /dev/null

# Add repository
echo "deb [arch=amd64,arm64 signed-by=/usr/share/keyrings/kopi-archive-keyring.gpg] \
  https://kopi-vm.github.io/ppa-kopi $(lsb_release -cs) main" | \
  sudo tee /etc/apt/sources.list.d/kopi.list > /dev/null

# Install kopi
sudo apt update && sudo apt install kopi
```

#### Using Windows Package Manager (Windows)

```bash
# Install kopi using winget
winget install kopi
```

#### Using Scoop (Windows)

```bash
# Add the kopi bucket
scoop bucket add kopi https://github.com/kopi-vm/scoop-kopi

# Install kopi
scoop install kopi
```

#### Using Cargo (All platforms)

```bash
# Install kopi
cargo install kopi
```

#### Post-installation Setup

```bash
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
kopi list                         # or: kopi ls

# macOS Note: Kopi automatically handles different JDK directory structures
# (e.g., Temurin's Contents/Home, Liberica's direct layout, Zulu's symlinks)

# Set global default
kopi global 21                    # or: kopi g 21, kopi default 21

# Set project-specific version
cd my-project
kopi local 17                     # or: kopi l 17, kopi pin 17
                                  # Creates .kopi-version file

# Show current JDK
kopi current
kopi current -q                   # Show only version number

# Show installation path
kopi which                        # Path to current java executable
kopi which 21                     # Path for specific version

# Uninstall a JDK
kopi uninstall temurin@21         # or: kopi u temurin@21
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

## Advanced Features

### Search Available JDKs
```bash
kopi search 21                    # Search for JDK 21 variants (alias: kopi s 21)
kopi search corretto              # List all Corretto versions
kopi search 21 --lts-only         # Show only LTS versions
kopi search 21 --detailed         # Show full details
kopi search 21 --json             # Output as JSON

# Note: 'search' is an alias for 'cache search'
```

### Shell Environment
```bash
# Launch new shell with specific JDK
kopi shell 21                     # or: kopi use 21
                                  # Launches new shell with Java 21 active

# Or use the env command for shell evaluation (like direnv)
eval "$(kopi env)"                # Use current project's JDK
eval "$(kopi env 21)"             # Use specific JDK
kopi env | source                 # Fish shell
kopi env | Invoke-Expression      # PowerShell
```

### Cache Management
```bash
kopi cache refresh                # Update metadata from configured sources
kopi cache info                   # Show cache details
kopi cache clear                  # Remove cached metadata
kopi cache search <query>         # Search with various options
kopi cache list-distributions     # List all available distributions

# Shortcuts:
kopi refresh                      # Alias for 'cache refresh' (alias: kopi r)
kopi search <query>               # Alias for 'cache search' (alias: kopi s)
```

### Shim Management
```bash
kopi shim list                    # List installed shims
kopi shim list --available        # Show available tools that could have shims
kopi shim add native-image        # Add shim for specific tool
kopi shim remove jshell           # Remove specific shim
kopi shim verify                  # Check shim integrity
kopi shim verify --fix            # Fix any issues found
```

### Diagnostics
```bash
kopi doctor                       # Run diagnostics on kopi installation
kopi doctor --json                # Output in JSON format
kopi doctor --check network       # Run specific category of checks
kopi -v doctor                    # Show detailed diagnostic information
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

Run `kopi cache list-distributions` to see all available distributions in your cache.

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
- `KOPI_JAVA_VERSION` - Override JDK version for current shell session
- `HTTP_PROXY` / `HTTPS_PROXY` - Proxy configuration for downloads
- `NO_PROXY` - Hosts to bypass proxy
- `RUST_LOG` - Enable debug logging (e.g., `RUST_LOG=debug kopi install 21`)

## Architecture

Kopi is designed with performance and reliability in mind:

- **Written in Rust** - Memory safe, fast, and efficient
- **Minimal Dependencies** - Quick installation and low overhead
- **Smart Caching** - Hybrid online/offline metadata management
- **Atomic Operations** - Safe concurrent JDK installations
- **Shell Integration** - Works with bash, zsh, fish, and PowerShell
- **macOS Support** - Handles diverse JDK directory structures (app bundles, direct layouts)

### macOS JDK Structure Handling

Kopi intelligently handles the various directory structures used by different JDK distributions on macOS:

#### Bundle Structure (Temurin, GraalVM)
```
temurin-21.0.5/
└── Contents/
    └── Home/         # Actual JDK files
        ├── bin/
        ├── lib/
        └── conf/
```

#### Direct Structure (Liberica)
```
liberica-21.0.5/      # JDK files at root
├── bin/
├── lib/
└── conf/
```

#### Hybrid Structure (Azul Zulu)
```
zulu-21.0.5/
├── bin -> zulu-21.jdk/Contents/Home/bin  # Symlinks at root
├── lib -> zulu-21.jdk/Contents/Home/lib
└── zulu-21.jdk/
    └── Contents/Home/                     # Actual files in bundle
```

Kopi automatically detects and handles these structures transparently:
- **Automatic Detection**: Structure is detected during installation
- **Metadata Caching**: Structure information is cached for fast switching
- **Transparent Operation**: Users never need to know about `Contents/Home`
- **Proper JAVA_HOME**: Always sets the correct JAVA_HOME for IDEs and build tools

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
2. Run `cargo clippy --all-targets -- -D warnings` to check for improvements
3. Run `cargo test --lib --quiet` to run unit tests
4. Submit a pull request

All commands must pass without errors before considering work complete.

## Documentation

- [User Reference](docs/reference.md) - Complete command reference with detailed examples
- [Architecture Decision Records](docs/adr/) - Design decisions and rationale

## License

Kopi is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- JDK metadata originally sourced from [foojay.io](https://foojay.io/) and optimized for fast access
- Inspired by [volta](https://volta.sh/), [nvm](https://github.com/nvm-sh/nvm), and [pyenv](https://github.com/pyenv/pyenv)

---

Built with ❤️ by the Kopi team
