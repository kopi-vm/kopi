<div align="center">
  <img alt="Kopi Logo" src="https://kopi-vm.github.io/assets/logo_black.png" width="200" height="200">
</div>

# Kopi - JDK Version Manager

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Kopi is a JDK version management tool written in Rust that integrates with your shell to seamlessly switch between different Java Development Kit versions.

**Full documentation at [https://kopi-vm.github.io/](https://kopi-vm.github.io/)**

## Installation

### Homebrew (macOS)
```bash
brew install kopi-vm/tap/kopi
```

### PPA (Ubuntu/Debian)
```bash
# Import GPG key
curl -fsSL https://keyserver.ubuntu.com/pks/lookup?op=get\&search=0xD2AC04A5A34E9BE3A8B32784F507C6D3DB058848 | \
  gpg --dearmor | \
  sudo tee /usr/share/keyrings/kopi-archive-keyring.gpg > /dev/null

# Add repository
echo "deb [arch=amd64,arm64 signed-by=/usr/share/keyrings/kopi-archive-keyring.gpg] \
  https://kopi-vm.github.io/ppa-kopi $(lsb_release -cs) main" | \
  sudo tee /etc/apt/sources.list.d/kopi.list > /dev/null

# Install
sudo apt update && sudo apt install kopi
```

### Windows Package Manager
```bash
winget install kopi
```

### Scoop (Windows)
```bash
scoop bucket add kopi https://github.com/kopi-vm/scoop-kopi
scoop install kopi
```

### Cargo (All platforms)
```bash
cargo install kopi
```

### Post-installation Setup
```bash
# Initial setup
kopi setup

# Add to PATH (in ~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish)
export PATH="$HOME/.kopi/shims:$PATH"
```

## Quick Start

### Essential Commands

```bash
# Install JDKs
kopi install 21                    # Latest JDK 21 (Eclipse Temurin by default)
kopi install temurin@17           # Specific distribution and version
kopi install corretto@11.0.21     # Exact version

# List installed JDKs
kopi list

# Set versions
kopi global 21                    # Set global default
kopi local 17                     # Set project version (creates .kopi-version)
kopi shell 11                     # Temporary shell override

# Show current JDK
kopi current
kopi which java                   # Path to java executable

# Search available JDKs
kopi search 21                    # Search for JDK 21 variants
kopi search corretto              # List all Corretto versions

# Uninstall JDKs
kopi uninstall temurin@21
```

### Working with Projects

```bash
# Create version file for project
cd my-project
kopi local 17                     # Creates .kopi-version

# Kopi automatically switches when entering directories with version files
cd my-java-21-project            # Automatically switches to Java 21
cd my-java-17-project            # Automatically switches to Java 17
```

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

### Development Workflow

When completing any coding task:
1. Run `cargo fmt` to format code
2. Run `cargo clippy --all-targets -- -D warnings` to check for improvements
3. Run `cargo test --lib --quiet` to run unit tests
4. Submit a pull request

All commands must pass without errors before considering work complete.

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## Documentation

- **User Documentation**: [https://kopi-vm.github.io/](https://kopi-vm.github.io/)
- **Architecture Decision Records**: [docs/adr/](docs/adr/)
- **Developer Reference**: [CLAUDE.md](CLAUDE.md)

## License

Kopi is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- JDK metadata originally sourced from [foojay.io](https://foojay.io/) and optimized for fast access
- Inspired by [volta](https://volta.sh/), [nvm](https://github.com/nvm-sh/nvm), and [pyenv](https://github.com/pyenv/pyenv)

---

Built with ❤️ by the Kopi team
