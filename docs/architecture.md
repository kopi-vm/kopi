# Architecture

## Project Structure

```text
kopi/
├── src/
│   ├── api/             # API integration with foojay.io
│   ├── archive/         # Archive extraction functionality (TAR/ZIP)
│   ├── bin/             # Binary executables (kopi-shim)
│   ├── cache/           # Metadata caching functionality
│   ├── commands/        # Command implementations
│   ├── doctor/          # Doctor command for system diagnostics
│   │   └── checks/      # Individual diagnostic checks
│   ├── download/        # Download management and progress reporting
│   ├── error/           # Error handling and formatting
│   ├── indicator/       # Progress indicator and user feedback
│   ├── locking/         # Lock controller, fallback strategy, hygiene runner
│   ├── installation/    # JDK installation management
│   ├── metadata/        # Metadata management and parsing
│   │   └── generator/   # Metadata generation utilities
│   ├── models/          # Data models and structures
│   ├── platform/        # Platform-specific functionality
│   ├── paths/           # Canonical Kopi home path registry and helpers
│   ├── security/        # Security validation and HTTPS verification
│   ├── shim/            # Shim management
│   ├── storage/         # Storage and disk space management
│   ├── test/            # Test utilities and helpers
│   ├── uninstall/       # JDK uninstallation functionality
│   └── version/         # Version parsing and handling
├── tests/               # Integration tests
│   └── common/          # Common test utilities
├── benches/             # Performance benchmarks
├── benchmarks/          # Benchmark results and history
│   └── baselines/       # Baseline benchmark data
├── docs/
│   ├── analysis/        # Problem exploration and requirement discovery (TDL)
│   │   └── archive/     # Archived analysis documents
│   ├── requirements/    # Formal requirements (FR-####, NFR-####)
│   ├── adr/             # Architecture Decision Records
│   ├── tasks/           # Task-specific design and planning documents
│   │   └── <task>/      # Per-task directory containing:
│   │       ├── design.md # Technical design document
│   │       └── plan.md   # Implementation plan
│   ├── templates/       # TDL document templates
│   │   └── examples/    # Template usage examples
│   ├── images/          # Documentation images and diagrams
│   ├── reviews/         # Code and design reviews
│   └── traceability.md  # Central requirements-to-tasks mapping
├── pkg/                 # Packaging configurations
│   ├── nfpm/            # nfpm package manager configs
│   └── wix/             # Windows installer configs
├── scripts/             # Development and CI scripts
├── .cargo/              # Cargo configuration
├── .github/             # GitHub Actions workflows
│   └── workflows/       # CI/CD pipeline definitions
├── AGENTS.md            # AI agent instructions and workflow
├── CLAUDE.md            # Claude Code guidance and conventions
├── Cargo.toml           # Project dependencies and metadata
├── Cargo.lock           # Dependency lock file
├── README.md            # Project documentation
├── LICENSE              # Project license
└── rust-toolchain.toml  # Rust toolchain specification
```

## Key Files

### Core Entry Points

- `src/main.rs` - Main application entry point with CLI command parsing (uses `clap` v4.5.40 with derive feature)
- `src/lib.rs` - Library entry point exposing shared functionality
- `src/bin/kopi-shim.rs` - Shim binary for transparent JDK version switching
- `src/bin/kopi-metadata-gen.rs` - Metadata generation utility

### Configuration & Models

- `src/config.rs` - Global configuration management and loading
- `src/models/` - Core data models:
  - `api.rs` - API response structures
  - `distribution.rs` - JDK distribution definitions
  - `metadata.rs` - Metadata structures
  - `package.rs` - Package information
  - `platform.rs` - Platform-specific models

### Filesystem Paths

- `src/paths/mod.rs` - Module root re-exporting Kopi home helpers
- `src/paths/home.rs` - Base directory constants and ensure helpers (`jdks`, `cache`, `shims`, `bin`, `locks`)
- `src/paths/install.rs` - Installation directory utilities and metadata file helpers
- `src/paths/cache.rs` - Cache directory helpers including metadata cache paths
- `src/paths/shims.rs` - Shim directory helpers and executable resolution
- `src/paths/locking.rs` - Lock directory helpers aligned with ADR-8mnaz
- `src/paths/shared.rs` - Shared sanitisation and directory creation utilities

### Command Implementations

- `src/commands/mod.rs` - Command registry and dispatch
- `src/commands/install.rs` - JDK installation logic
- `src/commands/cache.rs` - Cache management commands
- `src/commands/current.rs` - Display current JDK version
- `src/commands/env.rs` - Shell environment setup

### Documentation & Process

- `CLAUDE.md` - Repository conventions and AI assistant guidance
- `AGENTS.md` - AI agent workflow instructions
- `docs/tdl.md` - Traceable Development Lifecycle (TDL) documentation
- `docs/templates/README.md` - TDL template usage instructions
- `docs/traceability.md` - Requirements-to-implementation mapping
- `docs/adr/` - Architecture Decision Records directory

### Development & Build

- `Cargo.toml` - Project dependencies and metadata
- `rust-toolchain.toml` - Rust version specification
- `.cargo/config.toml` - Cargo build configuration
- `.github/workflows/` - CI/CD pipeline definitions

## Key Architectural Components

### Command System

- **CLI Interface**: Subcommand-based architecture using `clap` derive API
- **Command Registry**: Centralized command dispatch in `src/commands/mod.rs`
- **Subcommands**: Install, uninstall, cache, current, env, doctor, shell, local, global, etc.
- **Exit Codes**: Standardized error codes for different failure scenarios (see `src/error/exit_codes.rs`)

### Metadata Management

- **Data Sources**: Primary source from foojay.io API, with local index fallback
- **Caching Strategy**: Hybrid approach with `~/.kopi/cache/metadata.json`
  - Network-first with fallback to cache
  - Automatic refresh on cache commands
  - TTL-based invalidation
- **Metadata Generation**: Dedicated tool (`kopi-metadata-gen`) for offline metadata creation
- **Provider Abstraction**: Flexible provider system supporting HTTP and local sources

### JDK Installation & Storage

- **Installation Path**: `~/.kopi/jdks/<vendor>-<version>/`
- **Path Registry**: The `src/paths/` helpers (e.g., `paths::install`, `paths::cache`, `paths::locking`) derive every Kopi home subdirectory to guarantee consistent layout across commands (FR-hq1ns).
- **Archive Support**: TAR and ZIP extraction with platform-specific handling
- **Download Management**: Progress reporting with resumable downloads
- **Storage Repository**: Centralized JDK management with disk space validation
- **Uninstallation**: Safe batch uninstall with dependency checking

### Filesystem Path Registry

- **Module Layout**: `src/paths/` exposes domain modules (`home`, `install`, `cache`, `shims`, `locking`, `shared`) that are the sole source of Kopi home path construction.
- **Responsibilities**: Helpers encapsulate directory naming, atomic directory creation, and sanitisation shared between installation, cache, shim, and locking flows.
- **Integration Points**: Commands, storage subsystems, and doctor checks import the helpers rather than joining strings, preventing drift in directory layout and satisfying FR-hq1ns/NFR-4sxdr.

### Version Resolution

- **Version Files**: `.kopi-version` (native with `@` separator) and `.java-version` (compatibility)
- **Resolution Order**:
  1. Environment variable (`KOPI_JAVA_VERSION`)
  2. Local project file (`.kopi-version` or `.java-version`)
  3. Global default version
- **Version Parser**: Flexible parsing supporting `vendor@version` format

### Shell Integration

- **Shim System**: Transparent executable proxies in `~/.kopi/shims/`
- **Shim Binary**: Lightweight Rust binary (`kopi-shim`) for JDK switching
- **Tool Discovery**: Automatic detection of Java tools (java, javac, jar, etc.)
- **Shell Support**: Bash, Zsh, Fish with environment setup commands

### User Feedback & Progress

- **Progress Indicators**: Configurable indicators (simple, fancy, silent)
- **Status Reporting**: Real-time feedback during long operations
- **Diagnostic Tool**: `doctor` command for system health checks
- **Error Formatting**: Context-aware error messages with actionable suggestions

### Platform Abstraction

- **OS Detection**: Runtime platform detection for OS-specific behavior
- **Path Handling**: Cross-platform path manipulation
- **Symlink Management**: Platform-specific symlink creation and validation
- **Process Execution**: Abstracted process spawning for cross-platform support

### Security & Validation

- **HTTPS Enforcement**: All downloads use verified HTTPS connections
- **Archive Validation**: Security checks before extraction
- **Permission Checks**: File system permission validation
- **Shim Security**: Protection against path traversal and injection

### Error Handling

- **Error Context**: Rich error context with cause chains
- **Recovery Suggestions**: Actionable error messages with fix hints
- **Graceful Degradation**: Fallback strategies for network failures
- **Structured Exit Codes**: Consistent exit codes for scripting

### Locking Subsystem

- **LockController**: Central API that coordinates advisory locks, filesystem detection, and fallback selection per \[`src/locking/controller.rs`]
- **Advisory Backend**: Uses `std::fs::File` locks for supported filesystems with RAII release semantics (`src/locking/handle.rs`)
- **Atomic Fallback**: `create_new`-based locking for network filesystems with JSON metadata and marker files (`src/locking/fallback.rs`)
- **Lock Hygiene Runner**: Startup sweep that removes stale fallback artifacts and staging files (`src/locking/hygiene.rs`, invoked from `src/main.rs`)
- **Configuration**: `locking.mode` (`auto`, `advisory`, `fallback`) and `locking.timeout` control acquisition strategy and hygiene thresholds (`src/config.rs`)
- **Wait Instrumentation**: `LockFeedbackBridge` charts wait lifecycle events through shared progress indicators (TTY, non-TTY, or silent), while `StatusReporterObserver` falls back to textual messaging when no indicator is available; timeout errors continue to record both the resolved value and its provenance (CLI flag, environment variable, configuration, or default).
- **Installation Integration**: `InstallCommand` acquires a `ScopedPackageLockGuard` before touching staging directories or shims, surfaces wait feedback through the status reporter, and explicitly releases the guard to bubble up release failures (`src/commands/install.rs`).
- **Uninstall Integration**: `UninstallHandler` and batch/recovery paths resolve the same scoped lock identifiers as installs, enforce acquisition before destructive work begins, and reuse status reporter wait messaging to expose contention (`src/uninstall/mod.rs`, `src/uninstall/batch.rs`, `src/uninstall/cleanup.rs`).

## Storage Locations

- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/shims/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

## Configuration System

- Global config stored at `~/.kopi/config.toml`
- Loaded automatically by components via `KopiConfig::load()`
- Uses sensible defaults when config file is missing
