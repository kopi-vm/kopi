# ADR-009: Logging Strategy

## Status

Proposed

## Context

Kopi requires a robust logging strategy to support debugging, monitoring, and operational visibility. The current codebase uses the `log` crate facade but lacks a logger implementation, resulting in no actual log output.

After researching popular CLI tools and version managers, we found that most tools use a combination of CLI flags (-v/--verbose) and environment variables for debug output. Structured logging is rarely used in CLI applications, and logging configuration in project files is considered an anti-pattern.

We need to select and implement a logging solution that meets the following requirements:

1. **Development Requirements**
   - Easy debugging of JDK downloads, installations, and version switching
   - Clear visibility into API interactions with foojay.io
   - Ability to trace shell integration issues
   - Module-level log filtering for focused debugging

2. **Production Requirements**
   - Minimal performance overhead
   - Configurable log levels via environment variables
   - Reasonable binary size impact
   - No logs in release builds unless explicitly enabled

3. **User Experience Requirements**
   - Silent by default (no log output for normal operations)
   - Clear error messages separate from debug logs
   - Discoverable via CLI flags (-v/--verbose)
   - Optional verbose output for troubleshooting

4. **Operational Requirements**
   - Simple debug output for troubleshooting
   - Security audit trail for sensitive operations
   - NO logging configuration in project files (security risk)

## Decision Drivers

1. **Ecosystem Compatibility**: Should work well with Rust ecosystem expectations
2. **Simplicity**: Easy to implement and maintain for a CLI tool
3. **Performance**: Minimal overhead when logging is disabled
4. **Flexibility**: Ability to add more features later without major refactoring
5. **Binary Size**: Keep the CLI tool lightweight

## Research Findings

### CLI Tools Logging Patterns

We researched popular CLI tools to understand common patterns:

**Version Managers:**

- **nvm**: No built-in verbose/debug option
- **pyenv**: Uses `PYENV_DEBUG=1` environment variable
- **rbenv**: Uses `RBENV_DEBUG` or `rbenv --debug <command>`
- **volta**: No documented debug environment variable
- **fnm**: Supports both `FNM_LOGLEVEL=info` and `--log-level` flag

**Package Managers:**

- **npm**: Uses `-d/-dd/-ddd` flags and `NPM_CONFIG_LOGLEVEL` environment variable
- **pip**: Uses `-v/-vv/-vvv` for verbosity levels
- **cargo**: Separates user-facing verbosity (-v/-vv) from internal debug logging (CARGO_LOG)
- **brew**: Uses `-v/--verbose` and `-d/--debug` flags plus `HOMEBREW_VERBOSE`

**Key Insights:**

1. Most tools default to minimal output
2. CLI flags (-v/--verbose) are the primary interface for users
3. Environment variables are used for persistent configuration or advanced debugging
4. NO major tool puts logging configuration in project config files
5. Structured logging is rare in CLI tools - simple debug prints are the norm

### Why NOT Project-Level Logging Configuration

After analyzing tools like npm, cargo, and gradle, we found that NONE put logging config in project files:

- **Security Risk**: Verbose logging can expose sensitive information
- **Team Friction**: Different developers want different log levels
- **Git Problems**: Accidentally committing debug configuration
- **User Control**: Logging is a user preference, not a project requirement

## Considered Options

### Option 1: log + env_logger

The standard combination used by most Rust CLI tools.

**Implementation:**

```rust
// In main.rs
use env_logger::Builder;
use log::LevelFilter;

fn main() -> Result<()> {
    // Initialize with custom defaults
    Builder::from_env(env_logger::Env::default()
        .default_filter_or("warn"))
        .format_timestamp(None)  // No timestamps for CLI
        .format_target(false)     // Cleaner output
        .init();

    // Existing code...
}
```

**Advantages:**

- Industry standard for Rust CLI tools
- Users expect RUST_LOG to work
- Minimal dependencies (env_logger = ~200KB)
- Well-documented with extensive examples
- Zero overhead when disabled
- Compile-time optimization removes disabled levels

**Disadvantages:**

- Limited to console output without extensions
- No built-in file logging or rotation
- Basic formatting options
- No structured logging without additional work

**Usage Example:**

```bash
# Normal operation - no output
kopi install 21

# Debug specific module
RUST_LOG=kopi::download=debug kopi install 21

# Verbose mode
RUST_LOG=debug kopi install 21

# Trace everything
RUST_LOG=trace kopi install 21
```

### Option 2: log + flexi_logger

More flexible logger with file output capabilities.

**Implementation:**

```rust
use flexi_logger::{Logger, LogSpecification};

fn main() -> Result<()> {
    Logger::with_env_or_str("warn")
        .log_to_stdout()
        .format(flexi_logger::opt_format)
        .start()?;

    // Existing code...
}
```

**Advantages:**

- File logging with rotation support
- Runtime log level changes
- Multiple output targets
- More formatting options
- Can write different levels to different outputs

**Disadvantages:**

- Larger dependency (~400KB)
- More complex API
- Overkill for simple CLI logging
- File I/O overhead even when not used

### Option 3: tracing + tracing-subscriber

Modern async-first logging and diagnostics.

**Implementation:**

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Existing code...
}
```

**Advantages:**

- Structured logging with spans
- Better for async code
- Rich ecosystem
- Modern approach gaining adoption
- Excellent performance

**Disadvantages:**

- Larger dependency footprint (~600KB)
- More complex mental model
- Overkill for synchronous CLI tool
- Different from standard log macros

### Option 4: log + fern

Lightweight, performance-focused logger.

**Implementation:**

```rust
use fern;

fn setup_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}",
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .level_for("kopi", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
```

**Advantages:**

- Zero-allocation design
- Very fast
- Clean configuration API
- Small dependency

**Disadvantages:**

- No RUST_LOG support out of the box
- Less community adoption
- Manual environment variable handling needed

### Option 5: log + simple_logger

Minimal logger implementation.

**Advantages:**

- Extremely simple
- Tiny dependency
- Good for basic needs

**Disadvantages:**

- Too basic for production use
- No module filtering
- Limited configuration

## Comparison Matrix

| Aspect                 | env_logger | flexi_logger | tracing    | fern       | simple_logger |
| ---------------------- | ---------- | ------------ | ---------- | ---------- | ------------- |
| **Setup Complexity**   | ⭐⭐⭐⭐⭐ | ⭐⭐⭐       | ⭐⭐       | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐    |
| **Performance**        | ⭐⭐⭐⭐   | ⭐⭐⭐       | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐      |
| **Features**           | ⭐⭐⭐     | ⭐⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐ | ⭐⭐⭐     | ⭐⭐          |
| **Binary Size Impact** | ⭐⭐⭐⭐   | ⭐⭐⭐       | ⭐⭐       | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐    |
| **Ecosystem Standard** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐       | ⭐⭐⭐⭐   | ⭐⭐       | ⭐⭐          |
| **Module Filtering**   | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐   | ❌            |
| **Future Flexibility** | ⭐⭐⭐     | ⭐⭐⭐⭐     | ⭐⭐⭐⭐⭐ | ⭐⭐⭐     | ⭐            |

## Decision

We will use **log + env_logger** as our logging solution with a focus on CLI flags for discoverability.

### Rationale

1. **Industry Patterns**: Our research shows that CLI flags are the primary interface for verbosity control in successful CLI tools.

2. **Simplicity**: Simple debug prints are sufficient for CLI tools - structured logging is unnecessary complexity.

3. **User Experience**: CLI flags (-v/--verbose) are discoverable through --help, while environment variables require documentation reading.

4. **Flexibility**: Supporting both CLI flags AND environment variables gives users choice:
   - Quick debugging: `kopi install 21 -vv`
   - Session debugging: `export RUST_LOG=debug`
   - CI/CD configuration: Set environment variables once

5. **Security**: By NOT supporting project-level logging configuration, we avoid security risks and team friction.

6. **Performance**: Near-zero overhead when logging is disabled, which is the default case.

### Implementation Strategy

#### Phase 1: CLI-First Implementation

```rust
// Cargo.toml
[dependencies]
env_logger = "0.11"

// src/cli.rs - Global args for all commands
#[derive(Parser)]
struct Cli {
    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[clap(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

// src/main.rs
fn setup_logger(cli: &Cli) {
    // CLI flags set the default level
    let default_level = match cli.verbose {
        0 => "warn",   // Default: only warnings and errors
        1 => "info",   // -v: show info messages
        2 => "debug",  // -vv: show debug messages
        _ => "trace",  // -vvv or more: show everything
    };

    // RUST_LOG can override if set
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(default_level)
    )
    .format_timestamp(None)  // No timestamps for CLI output
    .format_module_path(false)  // Cleaner output
    .init();
}
```

#### Phase 2: Usage Examples

```bash
# Normal operation - warnings and errors only (silent by default)
kopi install 21

# See what kopi is doing
kopi install 21 -v

# Debug installation issues
kopi install 21 -vv

# Trace everything (very verbose)
kopi install 21 -vvv


# Advanced: debug specific module via environment variable
RUST_LOG=kopi::download=debug kopi install 21

# CI/CD: set once for all commands
export RUST_LOG=info
kopi install 21
kopi use 21
```

#### Phase 3: Simple Logging Patterns

```rust
// Keep logging simple - no structured logging needed
log::info!("Installing JDK {}", version);
log::debug!("Downloading from {}", url);
log::trace!("Response headers: {:?}", headers);

// For operations that take time
log::info!("Extracting archive...");
// ... extraction code ...
log::info!("Extracted {} files", count);
```

### Logging Guidelines

1. **Default Behavior**:
   - No `--quiet` flag needed - the tool is quiet by default
   - Only warnings and errors are shown without flags
   - This follows the Unix philosophy of "silence is golden"

2. **Log Levels**:
   - `ERROR`: Unrecoverable errors that prevent operation completion
   - `WARN`: Recoverable issues or potential problems (shown by default)
   - `INFO`: Major operations (install, uninstall, version switch)
   - `DEBUG`: Detailed flow information, API calls
   - `TRACE`: Very detailed debugging, including data dumps

3. **What to Log**:
   - JDK downloads (start, progress milestones, completion)
   - Version resolution and selection logic
   - Shell integration changes
   - API calls to foojay.io
   - Cache operations
   - Security validations

4. **What NOT to Log**:
   - User credentials or tokens
   - Full file paths containing user information
   - Sensitive system information
   - Excessive detail in normal operation

5. **NO Project-Level Logging Configuration**:
   - Logging configuration will NOT be supported in `kopi.toml`
   - This is a security best practice - prevents accidental verbose logging in production
   - Logging is a user preference, not a project requirement
   - Use CLI flags or environment variables instead

## Consequences

### Positive

- Discoverable through `--help` (CLI flags)
- Follows industry patterns from cargo, npm, pip
- Simple implementation without structured logging complexity
- Secure by default (no project-level config)
- Flexible with both CLI flags and environment variables
- Good performance with minimal overhead
- Small binary size impact

### Negative

- No structured logging (but not needed for CLI tools)
- No project-specific logging config (intentional security choice)
- Users must specify verbosity per invocation or set environment variable

### Neutral

- RUST_LOG serves as advanced/power-user feature
- Simple debug prints instead of structured logs
- No log file output (can add later if needed)

## Future Considerations

1. **File Logging**: If needed, we can add a file output using the `log4rs` or custom implementation.

2. **Structured Logging**: Can add structured fields using the `log` crate's key-value feature.

3. **Migration to Tracing**: If we add significant async code, migrating to `tracing` would be straightforward.

4. **Metrics**: Consider adding metrics collection separately from logging.

## References

- [log crate documentation](https://docs.rs/log/)
- [env_logger documentation](https://docs.rs/env_logger/)
- [The Rust Programming Language - Using env_logger](https://rust-cli.github.io/book/tutorial/output.html)
- [Comparing Rust Logging Crates](https://www.lpalmieri.com/posts/2020-09-27-zero-to-production-4-are-we-observable-yet/)
