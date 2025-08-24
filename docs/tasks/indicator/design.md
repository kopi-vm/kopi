# Kopi Progress Indicator Module Design

## Implementation Status

**Current Status**: Not implemented (design phase)

### Design Goals
- ✅ Unified progress indicator interface across all modules
- ✅ Consistent visual feedback for all long-running operations
- ✅ Support for both determinate (progress bar) and indeterminate (spinner) operations
- ✅ Simple command-line control (--no-progress flag)
- ✅ Terminal-aware output handling

### Components to Implement
- ❌ Core progress indicator trait and implementations
- ❌ Unified progress indicator factory
- ❌ Command-line flag integration (--no-progress)
- ❌ Migration of existing progress implementations
- ❌ Status message management system

## Overview

The progress indicator module provides a unified interface for displaying progress feedback across all Kopi operations. This module consolidates the currently fragmented progress implementations into a single, consistent system that ensures uniform user experience.

### Visual Elements

The module uses fixed visual elements for consistency:
- **Progress Bar Characters**: `█▓░` (full, partial, empty)
- **Spinner Characters**: `⣾⣽⣻⢿⡿⣟⣯⣷` (smooth braille animation)
- **Colors**: Green spinners, cyan/blue progress bars
- **Tick Speed**: 100ms for smooth animation

## Architecture

### Core Components

```rust
// src/indicator/mod.rs
pub trait ProgressIndicator: Send + Sync {
    /// Start a new progress operation
    fn start(&mut self, config: ProgressConfig);
    
    /// Update progress (for determinate operations)
    fn update(&mut self, current: u64, total: Option<u64>);
    
    /// Update status message
    fn set_message(&mut self, message: String);
    
    /// Complete the progress operation
    fn complete(&mut self, message: Option<String>);
    
    /// Handle error completion
    fn error(&mut self, message: String);
}

pub struct ProgressConfig {
    /// Operation name (e.g., "Downloading", "Installing", "Extracting")
    pub operation: String,
    
    /// Context-specific message (e.g., "temurin@21", "JDK archive")
    pub context: String,
    
    /// Total units for determinate operations (None for indeterminate/spinner)
    pub total: Option<u64>,
    
    /// Display style
    pub style: ProgressStyle,
}

pub enum ProgressStyle {
    /// Progress bar with bytes display (for downloads)
    Bytes,
    /// Progress bar with count display (for batch operations)
    Count,
}
```

### Implementation Strategy

#### 1. Indicatif-based Implementation

Primary implementation using the `indicatif` library:

```rust
// src/indicator/indicatif.rs
pub struct IndicatifProgress {
    progress_bar: Option<ProgressBar>,
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            progress_bar: None,
        }
    }
    
    fn create_template(&self, config: &ProgressConfig) -> String {
        match (&config.total, &config.style) {
            // Progress bar with bytes display
            (Some(_), ProgressStyle::Bytes) => {
                "{prefix}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                 {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})"
            }
            // Progress bar with count display
            (Some(_), ProgressStyle::Count) => {
                "{prefix}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                 {pos}/{len} {msg}"
            }
            // Indeterminate operations (spinner only when total is None)
            (None, _) => {
                "{prefix}\n{spinner:.green} [{elapsed_precise}] {msg}"
            }
        }.to_string()
    }
}

impl ProgressIndicator for IndicatifProgress {
    fn start(&mut self, config: ProgressConfig) {
        let prefix = format!("{} {}", config.operation, config.context);
        
        let pb = match config.total {
            Some(total) => ProgressBar::new(total),
            None => ProgressBar::new_spinner(),
        };
        
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(&self.create_template(&config))
                .unwrap()
                .progress_chars("█▓░")  // Fixed characters for progress bar
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")  // Fixed characters for spinner
        );
        
        pb.set_prefix(prefix);
        pb.enable_steady_tick(Duration::from_millis(100));
        
        self.progress_bar = Some(pb);
    }
    
    fn update(&mut self, current: u64, _total: Option<u64>) {
        if let Some(pb) = &self.progress_bar {
            pb.set_position(current);
        }
    }
    
    fn set_message(&mut self, message: String) {
        if let Some(pb) = &self.progress_bar {
            pb.set_message(message);
        }
    }
    
    fn complete(&mut self, message: Option<String>) {
        if let Some(pb) = &self.progress_bar {
            let msg = message.unwrap_or_else(|| "Complete".to_string());
            pb.finish_with_message(msg);
        }
    }
    
    fn error(&mut self, message: String) {
        if let Some(pb) = &self.progress_bar {
            pb.abandon_with_message(format!("✗ {}", message));
        }
    }
}
```

#### 2. Silent Implementation (Null Object Pattern)

Implementation for complete silence (--no-progress flag):

```rust
// src/indicator/silent.rs
pub struct SilentProgress;

impl ProgressIndicator for SilentProgress {
    fn start(&mut self, _config: ProgressConfig) {
        // No output
    }
    
    fn update(&mut self, _current: u64, _total: Option<u64>) {
        // No output
    }
    
    fn set_message(&mut self, _message: String) {
        // No output
    }
    
    fn complete(&mut self, _message: Option<String>) {
        // No output
    }
    
    fn error(&mut self, _message: String) {
        // No output - errors are handled separately by the error system
    }
}
```

#### 3. Simple Text Implementation

Implementation for non-terminal environments (CI/CD, logs, pipes):

```rust
// src/indicator/simple.rs
pub struct SimpleProgress {
    operation: String,
    context: String,
}

impl SimpleProgress {
    pub fn new() -> Self {
        Self {
            operation: String::new(),
            context: String::new(),
        }
    }
}

impl ProgressIndicator for SimpleProgress {
    fn start(&mut self, config: ProgressConfig) {
        println!("{} {}...", config.operation, config.context);
        self.operation = config.operation;
        self.context = config.context;
    }
    
    fn update(&mut self, _current: u64, _total: Option<u64>) {
        // No update output in simple mode to avoid log spam
    }
    
    fn set_message(&mut self, _message: String) {
        // No intermediate messages to keep logs clean
    }
    
    fn complete(&mut self, message: Option<String>) {
        let msg = message.unwrap_or_else(|| "Complete".to_string());
        println!("✓ {} {} - {}", self.operation, self.context, msg);
    }
    
    fn error(&mut self, message: String) {
        eprintln!("✗ {} {} - {}", self.operation, self.context, message);
    }
}
```

### Factory Pattern

```rust
// src/indicator/factory.rs
use std::env;

pub struct ProgressFactory;

impl ProgressFactory {
    /// Creates a progress indicator based on environment and user preferences
    pub fn create(no_progress: bool) -> Box<dyn ProgressIndicator> {
        if no_progress {
            // User explicitly requested no progress output
            Box::new(SilentProgress)
        } else if Self::should_use_simple_progress() {
            // Non-terminal or CI environment
            Box::new(SimpleProgress::new())
        } else {
            // Terminal environment with full animation support
            Box::new(IndicatifProgress::new())
        }
    }
    
    /// Determines if simple progress should be used based on environment
    fn should_use_simple_progress() -> bool {
        // Check if stderr is not a terminal (pipe, redirect, etc.)
        if !std::io::stderr().is_terminal() {
            return true;
        }
        
        // Check for CI environment variable (GitHub Actions, Jenkins, etc.)
        if env::var("CI").is_ok() {
            return true;
        }
        
        // Check for dumb terminal
        if let Ok(term) = env::var("TERM") {
            if term == "dumb" {
                return true;
            }
        }
        
        // Check for NO_COLOR environment variable (https://no-color.org/)
        if env::var("NO_COLOR").is_ok() {
            return true;
        }
        
        false
    }
}
```

### Environment Variable Support

The progress indicator system respects the following environment variables:

| Variable | Effect | Example |
|----------|--------|---------|
| `CI` | Forces simple text output | `CI=true kopi install temurin@21` |
| `TERM` | `dumb` value forces simple output | `TERM=dumb kopi cache refresh` |
| `NO_COLOR` | Forces simple output without colors | `NO_COLOR=1 kopi list` |

Additionally, the system automatically detects:
- **Pipe/Redirect**: When stderr is redirected (`kopi install 2>&1 | tee log.txt`)
- **Non-TTY**: When running without a terminal (cron jobs, systemd services)

## Status Message Management

### Design Comparison: Individual vs Centralized

#### Option 1: Individual Module Management (Current Approach)

**Pros:**
- Module-specific context awareness
- Flexible message customization
- No dependency on progress module for simple messages
- Easier to maintain module-specific message logic

**Cons:**
- Inconsistent message formatting
- Duplicated println! patterns
- No unified control over message output

#### Option 2: Centralized in Progress Module

**Pros:**
- Consistent message formatting
- Single point of control for all output
- Easier to implement quiet/verbose modes
- Better integration with progress indicators

**Cons:**
- All modules depend on progress module
- May overcomplicate simple status messages
- Additional abstraction layer

### Recommended Approach: Hybrid Solution

```rust
// src/indicator/status.rs
use std::env;

pub struct StatusReporter {
    silent: bool,
    use_color: bool,
}

impl StatusReporter {
    pub fn new(silent: bool) -> Self {
        Self { 
            silent,
            use_color: Self::should_use_color(),
        }
    }
    
    /// Determine if colors should be used in output
    fn should_use_color() -> bool {
        // Respect NO_COLOR environment variable
        if env::var("NO_COLOR").is_ok() {
            return false;
        }
        
        // Disable colors for dumb terminals
        if let Ok(term) = env::var("TERM") {
            if term == "dumb" {
                return false;
            }
        }
        
        // Check if stderr supports colors
        std::io::stderr().is_terminal()
    }
    
    /// Print a major operation status (always shown unless silent)
    pub fn operation(&self, operation: &str, context: &str) {
        if !self.silent {
            println!("{} {}...", operation, context);
        }
    }
    
    /// Print a step within an operation
    pub fn step(&self, message: &str) {
        if !self.silent {
            println!("  {}", message);
        }
    }
    
    /// Print success message
    pub fn success(&self, message: &str) {
        if !self.silent {
            let symbol = if self.use_color { "✓" } else { "[OK]" };
            println!("{} {}", symbol, message);
        }
    }
    
    /// Print error message (always shown)
    pub fn error(&self, message: &str) {
        let symbol = if self.use_color { "✗" } else { "[ERROR]" };
        eprintln!("{} {}", symbol, message);
    }
}
```

**Note**: Verbose logging is handled by the existing Rust logging infrastructure using `log` crate and controlled via `RUST_LOG` environment variable. The `StatusReporter` only handles user-facing status messages.

This hybrid approach allows modules to:
1. Use `StatusReporter` for consistent simple messages
2. Use `ProgressIndicator` for operations requiring progress feedback
3. Maintain module-specific messages where context is critical

## Migration Plan

### Phase 1: Core Implementation
1. Implement core trait and structs in `src/indicator/`
2. Implement `IndicatifProgress` with unified template for terminal environments
3. Implement `SilentProgress` for --no-progress flag (Null Object pattern)
4. Implement `SimpleProgress` for non-terminal environments
5. Implement `ProgressFactory` with three-case logic

### Phase 2: Migration of Existing Implementations
1. **Download Module** (`src/download/progress.rs`)
   - Replace `IndicatifProgressReporter` with new `ProgressIndicator`
   - Update `HttpFileDownloader` to use factory

2. **Cache Module** (`src/commands/cache.rs`)
   - Replace direct `ProgressBar` usage with `ProgressIndicator`
   - Use factory for spinner creation

3. **Uninstall Module** (`src/uninstall/progress.rs`)
   - Replace custom `ProgressReporter` with new system

4. **Metadata Generator** (`src/metadata/generator.rs`)
   - Replace direct progress bar creation
   - Use unified progress configuration

5. **Doctor Module** (`src/doctor/mod.rs`)
   - Integrate with new progress system
   - Respect global progress preferences

### Phase 3: Status Message Standardization
1. Implement `StatusReporter`
2. Migrate println! statements in:
   - `src/commands/install.rs`
   - `src/commands/setup.rs`
   - `src/commands/shim.rs`
   - `src/installation/auto.rs`

### Phase 4: Command-Line Integration
Progress display is controlled via global command-line options:
- `--no-progress`: Disable all progress indicators (uses `SilentProgress` - complete silence)
- Default behavior: Automatically selects implementation based on environment

The progress system automatically adapts:
- **Terminal environment**: `IndicatifProgress` with full animations
- **Non-terminal environment**: `SimpleProgress` with text-only output
- **--no-progress flag**: `SilentProgress` with no output at all

## Usage Examples

### Basic Usage
```rust
// In install command
let mut progress = ProgressFactory::create(no_progress);

progress.start(ProgressConfig {
    operation: "Downloading".to_string(),
    context: format!("{} {}", distribution.name(), version),
    total: Some(download_size),
    style: ProgressStyle::Bytes,
});

// During download
progress.update(bytes_downloaded, Some(total_bytes));

// On completion
progress.complete(Some("Download complete".to_string()));
```

### With Status Messages
```rust
let status = StatusReporter::new(no_progress);
let mut progress = ProgressFactory::create(no_progress);

status.operation("Installing", "temurin@21");

// Download phase
progress.start(ProgressConfig {
    operation: "Downloading".to_string(),
    context: "JDK archive".to_string(),
    total: Some(size),
    style: ProgressStyle::Bytes,
});
// ... download ...
progress.complete(None);

status.step("Verifying checksum...");
// ... verify ...

status.step("Extracting archive...");
// ... extract ...

status.success("Installation complete");
```

### Batch Operations Example
```rust
// For operations counting items (e.g., uninstalling multiple JDKs)
let mut progress = ProgressFactory::create(no_progress);

progress.start(ProgressConfig {
    operation: "Uninstalling".to_string(),
    context: "JDK versions".to_string(),
    total: Some(jdk_count),
    style: ProgressStyle::Count,
});

// During batch processing
for (index, jdk) in jdks.iter().enumerate() {
    progress.set_message(format!("Removing {}", jdk.name));
    progress.update(index as u64 + 1, Some(jdk_count));
    // ... uninstall logic ...
}

progress.complete(Some("All JDKs uninstalled".to_string()));
```

## Benefits

### Consistency
- All progress indicators use the same visual style
- Standardized message formatting
- Unified configuration management

### Maintainability
- Single source of truth for progress display logic
- Easier to update visual styles globally
- Reduced code duplication

### User Experience
- Predictable progress feedback across all commands
- Simple control via --no-progress flag
- Automatic adaptation to terminal capabilities
- Better accessibility with terminal detection

### Extensibility
- Easy to add new progress styles
- Simple to integrate into new commands
- Pluggable implementations for different environments

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_progress_indicator_lifecycle() {
    let mut progress = ProgressFactory::create(false);  // Not in no-progress mode
    
    progress.start(ProgressConfig {
        operation: "Testing".to_string(),
        context: "unit test".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    });
    
    progress.update(50, Some(100));
    progress.complete(Some("Test complete".to_string()));
}

#[test]
fn test_no_progress_mode() {
    let mut progress = ProgressFactory::create(true);  // no-progress mode
    
    progress.start(ProgressConfig {
        operation: "Testing".to_string(),
        context: "silent mode".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    });
    
    // Should not panic or output
    progress.update(50, Some(100));
    progress.complete(Some("Should not appear".to_string()));
}

#[test]
fn test_status_reporter_silent_mode() {
    let reporter = StatusReporter::new(true); // silent mode
    // Should not panic or output
    reporter.operation("Test", "operation");
    reporter.step("Should not appear");
    reporter.success("Should not appear");
}

#[test]
fn test_environment_detection() {
    // Test CI environment
    env::set_var("CI", "true");
    let progress = ProgressFactory::create(false);
    assert!(matches!(progress.as_ref(), &SimpleProgress { .. }));
    env::remove_var("CI");
    
    // Test TERM=dumb
    env::set_var("TERM", "dumb");
    let progress = ProgressFactory::create(false);
    assert!(matches!(progress.as_ref(), &SimpleProgress { .. }));
    env::remove_var("TERM");
    
    // Test NO_COLOR
    env::set_var("NO_COLOR", "1");
    let reporter = StatusReporter::new(false);
    assert!(!reporter.use_color);
    env::remove_var("NO_COLOR");
}
```

### Integration Tests
- Test progress display with actual terminal
- Verify no-progress mode works correctly
- Test environment variable handling:
  - `CI=true cargo test` - Should use simple progress
  - `TERM=dumb cargo test` - Should use simple progress
  - `NO_COLOR=1 cargo test` - Should use simple progress without colors
- Test output redirection: `cargo run 2>&1 | grep progress`
- Verify configuration loading and application

## Compatibility Considerations

### Terminal Support
- Detect terminal capabilities at runtime using `std::io::IsTerminal`
- Fallback to simple text for non-terminal environments
- Respect environment variables:
  - `NO_COLOR` - Disable colors and use simple output
  - `TERM=dumb` - Use simple text output
  - `CI=true` - Use simple text output for CI environments

### Platform Differences
- Windows console compatibility (automatic with `indicatif`)
- Unix terminal emulator support (xterm, iTerm2, etc.)
- CI/CD environment detection (GitHub Actions, Jenkins, GitLab CI, etc.)

### Environment Detection Priority
1. `--no-progress` flag (highest priority) → `SilentProgress`
2. `CI` environment variable → `SimpleProgress`
3. `TERM=dumb` → `SimpleProgress`
4. `NO_COLOR` → `SimpleProgress` (with color symbols replaced)
5. Non-terminal stderr → `SimpleProgress`
6. Default → `IndicatifProgress` (full animations)

## Future Enhancements

1. **Nested Progress**: Support for hierarchical progress tracking
2. **Progress Persistence**: Save/restore progress for resumable operations
3. **Progress Hooks**: Allow external tools to monitor progress via IPC or file-based progress tracking
4. **Machine-Readable Output**: JSON progress output for programmatic consumption
5. **Adaptive Display**: Smart switching between spinner and progress bar based on operation duration