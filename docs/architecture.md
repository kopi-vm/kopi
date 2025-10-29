# Architecture

## Project Structure

```text
kopi/
├── src/
│   ├── api/                  # Foojay and metadata HTTP clients
│   ├── archive/              # TAR/ZIP extraction utilities
│   ├── bin/                  # Auxiliary binaries (kopi-shim, kopi-metadata-gen)
│   ├── cache/                # Metadata cache models, conversion, search
│   ├── commands/             # CLI subcommands (install, cache, shell, etc.)
│   ├── config.rs             # Global configuration loader and overrides
│   ├── doctor/               # Doctor command orchestration
│   │   └── checks/           # Individual system diagnostics
│   ├── download/             # Download manager and progress hooks
│   ├── error/                # KopiError definitions and context formatting
│   ├── indicator/            # Progress indicator factory and renderers
│   ├── installation/         # Auto-install flow for missing versions
│   ├── locking/              # Advisory/fallback locking infrastructure
│   ├── logging.rs            # Logging setup and verbosity control
│   ├── metadata/             # Metadata provider abstraction and ingestion
│   │   └── generator/        # Offline metadata generation tooling
│   ├── models/               # Shared domain models
│   ├── paths/                # Canonical Kopi home registry and enforcement tests
│   ├── platform/             # Platform-specific helpers
│   ├── security/             # TLS validation and input sanitisation
│   ├── shim/                 # Shim discovery, installation, verification
│   ├── storage/              # Disk probes and repository operations
│   ├── test/                 # Shared test fixtures
│   ├── uninstall/            # Uninstall workflows and cleanup paths
│   ├── user_agent.rs         # Standardised HTTP User-Agent helpers
│   └── version/              # Version parsing, validation, and resolution
├── benches/                  # Criterion benchmark definitions
├── benchmarks/               # Stored benchmark baselines
├── coverage/                 # Coverage artefacts (lcov)
├── docs/
│   ├── analysis/             # TDL analysis documents
│   ├── requirements/         # Functional and non-functional requirements
│   ├── adr/                  # Architecture Decision Records
│   ├── tasks/                # Task packages (design and plan)
│   ├── templates/            # Document templates for TDL artefacts
│   ├── archive/              # Archived analyses, ADRs, tasks, reviews
│   ├── architecture.md       # Architecture overview (this file)
│   ├── development.md        # Developer workflow guide
│   ├── error_handling.md     # Error handling reference
│   ├── parallel-development.md # Multi-worktree guidance
│   ├── performance.md        # Performance strategy and benchmarks
│   ├── reference.md          # CLI and subsystem reference manual
│   ├── tdl.md                # Traceable Development Lifecycle overview
│   └── traceability.md       # Auto-generated traceability matrix
├── pkg/
│   ├── nfpm/                 # Linux packaging config
│   └── wix/                  # Windows installer config
├── scripts/                  # Development and CI automation scripts
├── tests/                    # Integration tests (with common fixtures)
├── config/                   # remark/markdown lint configuration
├── .cargo/                   # Cargo build configuration
├── .github/                  # GitHub Actions workflows
├── AGENTS.md                 # AI assistant workflow instructions
├── CLAUDE.md                 # Claude assistant guidance
├── Cargo.toml                # Project manifest
├── Cargo.lock                # Dependency lockfile
├── README.md                 # Project overview
├── rust-toolchain.toml       # Toolchain specification
├── bunfig.toml               # Bun task runner configuration
└── package.json              # JavaScript tooling metadata
```

## Key Files

### Core Entry Points

- `src/main.rs` – CLI definition using `clap` 4.5.40, logging setup, lock hygiene kick-off, and subcommand dispatch.
- `src/lib.rs` – Library entry point wiring modules for use by binaries and integration tests.
- `src/bin/kopi-shim.rs` – Lightweight shim executable that resolves the active JDK and execs the requested tool.
- `src/bin/kopi-metadata-gen.rs` – Offline metadata generator for bundling Foojay responses.

### Configuration & Models

- `src/config.rs` – Builds `KopiConfig` by merging defaults, `~/.kopi/config.toml`, `KOPI_*` environment overrides (using `__` as nesting separators), CLI flags, and derived values such as lock hygiene settings and auto-install preferences.
- `src/models/` – Core data models:
  - `api.rs` – Foojay response structures.
  - `distribution.rs` – Distribution metadata definitions.
  - `metadata.rs` – Cached metadata schema.
  - `package.rs` – Package descriptors used during installs and search.
  - `platform.rs` – Platform and architecture descriptors.

### Filesystem Paths

- `src/paths/mod.rs` – Module root re-exporting Kopi home helpers.
- `src/paths/home.rs` – Base directory constants and ensure helpers (`jdks`, `cache`, `shims`, `bin`, `locks`).
- `src/paths/install.rs` – Installation directory utilities and metadata helpers.
- `src/paths/cache.rs` – Cache directory helpers including metadata cache paths.
- `src/paths/shims.rs` – Shim directory helpers and executable resolution.
- `src/paths/locking.rs` – Lock directory helpers aligned with ADR-8mnaz.
- `src/paths/shared.rs` – Shared sanitisation and directory creation utilities guarded by `tests/paths_enforcement.rs`.

### Command Implementations

- `src/commands/install.rs` – JDK installation flow covering metadata lookup, downloads, extraction, verification, and lock acquisition.
- `src/commands/uninstall.rs` – Safe removal, cleanup, and lock hand-off for uninstall scenarios.
- `src/commands/list.rs` – Lists installed distributions and versions.
- `src/commands/shell.rs` – Session-scoped switching (`kopi shell` / alias `use`) with auto-install prompts.
- `src/commands/env.rs` – Emits shell-specific environment exports for evaluation.
- `src/commands/global.rs` – Sets the global default version, including auto-install support.
- `src/commands/local.rs` – Pins the project version by updating `.kopi-version`.
- `src/commands/which.rs` – Locates tools or homes with JSON and quiet output modes.
- `src/commands/cache.rs` – Implements `kopi cache` subcommands (`refresh`, `info`, `clear`, `search`, `list-distributions`) and backs the top-level `refresh`/`search` aliases.
- `src/commands/setup.rs` – Bootstraps shims, verifies prerequisites, and optionally recreates binaries.
- `src/commands/shim.rs` – Manages shim definitions (add/remove/list/verify).
- `src/commands/current.rs` – Reports the active JDK (`--quiet`, `--json`).
- `src/commands/doctor.rs` – Runs diagnostic suites with optional JSON output.

### Runtime Infrastructure

- `src/logging.rs` – Maps CLI verbosity (`-v`) to `env_logger` filters and formatting.
- `src/indicator/` – Progress indicator factory (`ProgressFactory`), renderers (indicatif, simple, silent), and status reporting utilities.
- `src/locking/wait_observer.rs` – Lock wait callbacks consumed by `LockFeedbackBridge` and `StatusReporterObserver`.
- `src/installation/auto.rs` – Auto install orchestration used by CLI commands.
- `src/download/` – Download manager with progress plumbing.
- `src/storage/` – Disk space probes, repository operations, and metadata manifest handling.

### Documentation & Process

- `AGENTS.md` / `CLAUDE.md` – Repository conventions and AI assistant guidance.
- `docs/tdl.md` – Traceable Development Lifecycle (TDL) documentation.
- `docs/reference.md` – CLI reference, cache/search behaviour, shim rules.
- `docs/development.md` – Developer workflow, logging, and security practices.
- `docs/error_handling.md` – Error taxonomy and exit codes.
- `docs/performance.md` – Benchmarking workflow and targets.
- `docs/parallel-development.md` – Parallel worktree guidance.
- `docs/archive/` – Historical analyses, ADRs, tasks, and reviews.

### Development & Build

- `Cargo.toml` / `Cargo.lock` – Project dependencies and lock state.
- `.cargo/config.toml` – Build profiles, incremental settings, and test threading.
- `scripts/` – Helper scripts (traceability, benchmarking, packaging).
- `benches/` / `benchmarks/` – Criterion benchmark suites and baselines documented in `docs/performance.md`.
- `bunfig.toml` / `package.json` – Bun-based formatting and linting tasks for documentation and TypeScript utilities.
- `config/` – remark/markdown lint configuration shared by documentation commands.
- `.github/workflows/` – CI/CD pipelines.

## Key Architectural Components

### Command System

- **CLI Interface**: `src/main.rs` uses `clap` 4.5.40 derive macros to register global flags `-v/--verbose`, `--no-progress`, and `--lock-timeout`, ensuring logging, progress rendering, and locking strategy are configured before command execution.
- **Subcommand Inventory**: Supports version management (`install`, `uninstall`, `list`, `shell`/`use`, `env`, `global`, `local`, `which`), metadata operations (`cache` with `refresh`, `info`, `clear`, `search`, `list-distributions`; hidden `refresh` and `search` aliases), environment setup (`setup`, `shim` add/remove/list/verify), and diagnostics (`doctor`).
- **Alias Delegation**: `kopi refresh` and `kopi search` map directly to `CacheCommand::Refresh` and `CacheCommand::Search`, preserving shared output controls documented in `docs/reference.md`.
- **Auto-Install Orchestration**: `installation::AutoInstaller` integrates with `global`, `local`, and `shell` flows to optionally fetch missing JDKs, honouring configuration flags (`auto_install.enabled`, `auto_install.prompt`, timeouts).

### Metadata & Cache Management

- **Provider Abstraction**: `metadata::provider::MetadataProvider` merges Foojay API sources, local indexes, and generator output, delivering a consolidated view for cache writes and offline usage.
- **Cache Lifecycle**: `cache::metadata_cache` persists aggregated metadata to `~/.kopi/cache/metadata.json` with timestamping; `CacheCommand::Refresh` performs multi-source fetches with progress instrumentation and summarises distribution counts.
- **Search & Filtering**: `CacheCommand::Search` supports compact, detailed, JSON, LTS-only, and field-forced lookups; hidden aliases share the implementation so automation can rely on consistent output options.
- **Metadata Manifests**: During installs, `storage::repository` writes `<distribution>-<version>.meta.json` descriptors alongside each JDK under `~/.kopi/jdks/`, enabling fast tool discovery and avoiding repeated filesystem scans as highlighted in `docs/reference.md`.
- **Offline Generation**: `src/metadata/generator` and the `kopi-metadata-gen` binary allow precomputing metadata bundles for air-gapped environments.

### JDK Installation & Storage

- **Installation Pathing**: JDKs live under `~/.kopi/jdks/<vendor>-<version>/`; helper modules derive the layout to satisfy FR-hq1ns/NFR-4sxdr.
- **Preflight & Validation**: `storage::disk_space` checks satisfy FR-x63pa by verifying capacity before downloads; archive extraction in `archive/` handles TAR/ZIP formats with checksum validation.
- **Lock Integration**: `install.rs` acquires `locking::ScopedPackageLockGuard` resources before touching staging directories, coordinating with the lock controller.
- **Metadata Generation**: Post-install, metadata manifests are generated to speed up future version resolution and shim updates.
- **Auto Install**: `installation::AutoInstaller` prompts users (if configured) and shells out to `kopi install`, tracking elapsed time and respecting command timeouts.
- **Uninstallation**: `uninstall::*` modules reuse lock scopes, handle cleanup (`cleanup.rs`), batch operations, and emit feedback via shared status reporters.

### Filesystem Path Registry

- **Canonical Helpers**: `paths::home`, `paths::install`, `paths::cache`, `paths::shims`, and `paths::locking` centralise directory naming and creation.
- **Enforcement**: `tests/paths_enforcement.rs` guards against hard-coded path segments, enforcing use of helper APIs per T-wn8p3.
- **Shared Utilities**: `paths::shared` handles sanitisation, ensuring directories stay within Kopi home and preventing traversal issues.

### Version Resolution

- **Version Files**: Supports `.kopi-version` (native format) and `.java-version` compatibility files with vendor qualifiers (`vendor@version`).
- **Precedence**: Resolution order is environment variable (`KOPI_JAVA_VERSION`), project file, then global default, mirroring `docs/reference.md`.
- **Parser & Requests**: `version::parser::VersionParser` normalises user input, while `version::VersionRequest` carries distribution, build, and JavaFX flags through install flows.
- **Validation**: Accepts safe character sets, enforces length, and rejects injection patterns as described in `docs/development.md`.

### Shell Integration

- **Shim System**: `shim::installer` and `shim::tools` manage symlinked proxies under `~/.kopi/shims/`, validating targets via `shim::security`.
- **Setup Automation**: `kopi setup` provisions shims, detects shell environments, and can rebuild proxies (`--force`).
- **Session Switching**: `ShellCommand` (alias `use`) updates shell environments with optional auto-install; `EnvCommand` emits export statements for Bash, Zsh, Fish, and PowerShell.
- **Tool Discovery**: Shim registry automatically exposes common Java tools, with verification commands (`kopi shim verify`) documented in `docs/reference.md`.

### User Feedback & Progress

- **Progress Abstraction**: `indicator::ProgressFactory` selects indicatif, simple, or silent renderers based on TTY detection, quiet/JSON modes, and the `--no-progress` flag.
- **Status Reporter**: `indicator::StatusReporter` offers step-based messaging for commands that need textual updates (auto install, uninstall cleanup).
- **Lock Wait Instrumentation**: `locking::wait_observer` plus `LockFeedbackBridge` translate lock lifecycle events into progress updates, meeting FR-c04js latency targets and offering cancellation/timeout guidance.
- **Consistency**: Shared formatting ensures cache refresh, installs, downloads, and locking share consistent messaging, as reinforced in `docs/development.md`.

### Diagnostics & Health

- **Doctor Command**: `doctor::DoctorCommand` aggregates checks across installation, JDK inventory, shell configuration, and network/cache health.
- **Check Modules**: `doctor/checks` provides targeted validators with reusable formatters; outputs can be rendered as JSON (`kopi doctor --json`) for machine consumption.
- **Guidance**: Diagnostic messaging mirrors recommendations in `docs/development.md`, providing actionable remediation steps.

### Platform Abstraction

- **OS Detection**: `platform::detection` (re-exported via `platform::{get_current_platform, get_current_os, get_current_architecture, matches_foojay_libc_type}`) identifies platform triples for metadata filtering and download selection.
- **Path Handling**: `platform::filesystem` and `platform::file_ops` provide platform-aware path utilities (Windows drive normalisation, Unix permission fixes).
- **Symlink & Process Management**: `platform::symlink` and `platform::process` abstract symlink creation and process inspection, ensuring consistent behaviour across Unix and Windows when managing shims and detecting in-use installations.

### Security & Validation

- **Path Safety**: `paths::shared` and shim validation guard against traversal outside `KOPI_HOME`.
- **Version Sanitisation**: `version::parser::VersionParser::validate_version_semantics` and `shim::security::SecurityValidator::validate_version` enforce safe character sets and length limits aligned with `docs/development.md`.
- **Tool Registry**: Shim commands verify allowed tool names, rejecting arbitrary executables.
- **Archive & Network**: `security/` validates HTTPS certificates, checksums, and enforces secure download transports; archives undergo integrity checks before extraction.
- **Permissions**: Commands validate permissions for shims and JDK directories, rejecting world-writable executables where unsafe.

### Error Handling

- **Structured Errors**: `error::KopiError` enumerates error domains (network, locking, system, user).
- **Context System**: `error::context` builds rich suggestions and formats; `format_error_chain` renders chained causes with optional colour.
- **Exit Codes**: `error::exit_codes` maps common scenarios (`invalid input`, `no local version`, `locking timeout`, `disk space`, `command not found`) to stable codes documented in `docs/error_handling.md`.
- **CLI Integration**: `src/main.rs` centralises error printing and exit handling so subcommands can return `Result<()>` without duplicating formatting.

### Locking Subsystem

- **Controller**: `locking::controller::LockController` coordinates advisory (fcntl/File) and fallback (`create_new` marker) strategies per ADR-8mnaz.
- **Scopes & Guards**: `locking::scope`, `locking::package_coordinate`, and `locking::ScopedPackageLockGuard` provide typed identifiers for installations, cache writers, and shim updates, fulfilling FR-v7ql4 and FR-ui8x2.
- **Timeout Resolution**: `locking::timeout::LockTimeoutResolver` merges CLI, environment, config, and defaults, emitting provenance for observability.
- **Feedback & Cancellation**: `locking::wait_observer`, `locking::cancellation`, `LockFeedbackBridge`, and `StatusReporterObserver` expose wait states, user cancellations (exit code 75), and timeout guidance with shared progress renderers.
- **Hygiene**: `locking::hygiene` cleans stale markers on startup; results are logged and surfaced without failing the CLI path.
- **Cache Writer**: `locking::cache_writer` serialises metadata cache writes to prevent concurrent corruption.

### Storage Locations

- JDKs: `~/.kopi/jdks/<vendor>-<version>/`.
- JDK metadata manifests: `~/.kopi/jdks/<vendor>-<version>.meta.json`.
- Shims: `~/.kopi/shims/`.
- Config: `~/.kopi/config.toml`.
- Cache: `~/.kopi/cache/metadata.json` plus auxiliary cache artefacts.
- Locks: `~/.kopi/locks/`.
- Logs and binaries reside under the same `KOPI_HOME`, configurable via `KOPI_HOME`.

### Configuration System

- **Loader**: `config::new_kopi_config` constructs `KopiConfig`, ensuring directories exist and defaults are applied.
- **Overrides**: Environment variables use `KOPI_*` with double underscores for nested fields (e.g., `KOPI_LOCKING__TIMEOUT`), while CLI `--lock-timeout` overrides are processed via `apply_lock_timeout_overrides`.
- **Locking**: `KopiConfig.locking` encapsulates mode (`auto`, `advisory`, `fallback`) and timeout; resolved values propagate to the lock controller.
- **Auto Install**: `KopiConfig.auto_install` toggles automatic installs, prompts, and command timeouts consumed by `AutoInstaller`.
- **Home Directory**: `KOPI_HOME` allows relocating the base directory; path helpers and config gracefully fall back to `~/.kopi` if unset.
- **Persistence**: Config writes respect locking and path helpers to prevent corruption across processes.
