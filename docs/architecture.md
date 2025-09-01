# Architecture

## Project Structure

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

## Key Files

- `/src/main.rs` - Application entry point with CLI command parsing
- `/src/lib.rs` - Library entry point for shared functionality
- `/src/config.rs` - Configuration management
- `/src/bin/kopi-shim.rs` - Shim binary for transparent JDK switching
- `/docs/adr/` - Architecture Decision Records documenting design choices
- `/docs/reference.md` - User reference manual with command documentation
- Uses `clap` v4.5.40 with derive feature for CLI argument parsing

## Key Architectural Components

- **Command Interface**: Subcommand-based CLI using clap derive API
- **JDK Metadata**: Fetches available JDK versions from foojay.io API
- **Version Management**: Installs and manages multiple JDK versions in `~/.kopi/jdks/<vendor>-<version>/`
- **Shell Integration**: Creates shims in `~/.kopi/shims/` for Java executables
- **Project Configuration**: Reads `.kopi-version` (native format with `@` separator) or `.java-version` (compatibility)
- **Metadata Caching**: Stores JDK metadata in `~/.kopi/cache/metadata.json` with hybrid caching strategy

## Storage Locations

- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/shims/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

## Configuration System

- Global config stored at `~/.kopi/config.toml`
- Loaded automatically by components via `KopiConfig::load()`
- Uses sensible defaults when config file is missing

## Performance Considerations

- **Test execution** is limited to 4 threads by default (configured in `.cargo/config.toml`)
- **Incremental compilation** is enabled for faster rebuilds
- **Build profiles** are optimized:
  - `dev` profile: Dependencies are optimized at level 2
  - `test` profile: Tests run with optimization level 1 and limited debug info
  - `release-fast` profile: Fast release builds without LTO for development