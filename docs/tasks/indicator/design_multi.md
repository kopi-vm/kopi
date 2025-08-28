# Multi-Progress Support Design

**Last Updated**: 2025-08-28 (Updated with investigation findings and StatusReporter coexistence strategy)

## Overview

This document outlines the design for adding multi-progress bar support to Kopi's ProgressIndicator system. The implementation focuses on providing nested progress bars for operations that have clear parent-child relationships, improving user visibility into long-running operations.

**Status**: Design validated through spike implementation. See `multiprogress_spike_report.md` for detailed findings and `multi_progress_spike.rs` for the working prototype.

## Goals

- Provide better visibility into nested operations (e.g., download progress during installation)
- Maintain simplicity in implementation and usage
- Avoid visual clutter by showing child progress only for significant operations
- Ensure consistent behavior across different terminal environments

## Non-Goals

- Supporting arbitrary nesting depth (only 1 level of parent-child)
- Maintaining backward compatibility
- Adding new command-line options or configuration flags
- Supporting parallel progress bars (only hierarchical parent-child relationships)

## Current State Analysis

### Current Implementation Limitations

1. **Install Command**: Forces `no_progress=true` for downloads to prevent overlapping progress bars
2. **Cache Refresh**: Uses `SilentProgress` for nested operations
3. **Terminal Corruption**: Risk of overlapping output when multiple progress bars are active

### Operations That Would Benefit

1. **Install Command** (High Impact)
   - Parent: Overall installation steps (1-8)
   - Child: Download progress (bytes/transfer rate) when file > 10MB
   - Child: Cache refresh progress when using Foojay API

2. **Cache Refresh Command** (Medium Impact)
   - Parent: Overall refresh steps (1-N)
   - Child: Individual source fetching progress

## Known Issues and Solutions

### Progress Bar Display Issues

#### 1. Duplicate Output
**Problem**: StatusReporter and IndicatifProgress display the same content, causing visual redundancy.
```
Installing Eclipse Temurin 21...
⠋ Installing Eclipse Temurin 21 [████░░░░] 3/7 Downloading
```

**Root Cause**: Both StatusReporter::operation() and IndicatifProgress display installation status.

**Solution**: Use conditional output based on progress mode, or integrate StatusReporter functionality into ProgressIndicator.

#### 2. Split Progress Bar
**Problem**: Progress bar appears on two lines with incomplete rendering.
```
Installing Eclipse Temurin 21...
⠋ Installing Eclipse Temurin 21 [
⠋ Installing Eclipse Temurin 21 [████░░░░] 3/7 Downloading...
```

**Root Cause**: MultiProgress is created lazily when create_child() is called, but the parent progress bar is not added to the MultiProgress instance.

**Solution**: Initialize IndicatifProgress with MultiProgress from the start, not on-demand.

#### 3. Log Output Interference
**Problem**: Log statements (log::info!, log::debug!) interrupt progress bar rendering.
```
⠋ Installing Eclipse Temurin 21 [[2025-08-28T00:25:11Z INFO  log_impact] Downloading JDK package
```

**Root Cause**: Log output to stderr conflicts with indicatif's terminal control.

**Solution**: 
- Use MultiProgress.suspend() for safe log output
- Consider console library for better terminal control

## Design Decisions

### Display Criteria

**Unified 10MB Threshold**: Use 10MB as the consistent threshold for displaying child progress bars across all operations.

### Per-Operation Rules

#### Download Operations
- **Content-Length >= 10MB**: Display child progress bar with bytes/transfer rate
- **Content-Length < 10MB**: No child progress bar
- **Content-Length unknown**: No child progress bar

#### Cache Operations
- **Foojay API**: Always display child progress (typically 5-10 seconds)
- **HTTP Metadata**: Display child progress if file size >= 10MB
- **Local Files**: Never display child progress (too fast)

### Terminal Environment Handling

Environment-based behavior selection is critical for proper display. The system uses three different implementations:

- **SilentProgress**: Completely silent, no output at all (for `--no-progress`)
- **SimpleProgress**: Minimal text-based output (for CI, non-TTY, etc.)
- **IndicatifProgress**: Rich animated progress bars with MultiProgress (for interactive terminals)

#### Display Mode Selection Matrix

| Condition | Implementation | Output Behavior |
|-----------|----------------|-----------------|
| `--no-progress` flag | SilentProgress | No output at all |
| Non-TTY (pipe, redirect) | SimpleProgress | ASCII text only ("[OK]", "[ERROR]") |
| CI environment (`CI` env var) | SimpleProgress | ASCII text only ("[OK]", "[ERROR]") |
| Dumb terminal (`TERM=dumb`) | SimpleProgress | ASCII text only ("[OK]", "[ERROR]") |
| NO_COLOR environment | SimpleProgress | ASCII text only ("[OK]", "[ERROR]") |
| TTY Terminal (normal) | IndicatifProgress | Rich progress bars with animations |

#### Implementation

The existing `ProgressFactory::create` method already provides smart selection:

```rust
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

fn should_use_simple_progress() -> bool {
    // Check if stderr is not a terminal
    if !std::io::stderr().is_terminal() {
        return true;
    }
    // Check for CI environment
    if env::var("CI").is_ok() {
        return true;
    }
    // Check for dumb terminal
    if env::var("TERM").ok() == Some("dumb".to_string()) {
        return true;
    }
    // Check for NO_COLOR environment variable
    if env::var("NO_COLOR").is_ok() {
        return true;
    }
    false
}
```

#### Safe Logging with Progress Indicators

The ProgressIndicator trait provides safe output methods:

```rust
// Use the suspend method for multi-line or complex output
progress.suspend(&mut || {
    info!("Processing step");
    debug!("Detailed information");
    println!("Additional output");
});

// Use the println method for single-line output
progress.println("Status update")?;
```

This ensures that log output doesn't interfere with progress bar rendering when using IndicatifProgress.

## Implementation Plan

### Phase 1: Core Infrastructure

#### 1.1 Update ProgressIndicator Trait

The ProgressIndicator trait has been extended with the following required methods:

- `create_child()`: Returns a boxed ProgressIndicator instance for nested operations ✅
- `suspend(&self, f: &mut dyn FnMut())`: Temporarily suspends progress display for safe log output ✅
- `println(&self, message: &str) -> Result<()>`: Safely outputs a line of text with the progress display ✅

#### 1.2 Implementation Strategy

**SilentProgress**: 
- `create_child()`: Returns a new SilentProgress instance, maintaining silent behavior
- `suspend()`: Executes the closure without any special handling
- `println()`: No-op, produces no output

**SimpleProgress**: 
- `create_child()`: Returns a SilentProgress instance to avoid log spam in CI environments
- `suspend()`: Executes the closure directly (no progress bars to suspend)
- `println()`: Direct println! output (safe since no progress bars are active)
- **Output format**: Use ASCII symbols only ("[OK]", "[ERROR]") - no Unicode symbols ("✓", "✗")

**IndicatifProgress**: 
- `create_child()`: Creates a child progress bar using indicatif's MultiProgress functionality
- `suspend()`: Uses `MultiProgress::suspend()` to safely pause progress bars during output
- `println()`: Uses `MultiProgress::println()` to output text above progress bars

### Phase 2: IndicatifProgress MultiProgress Implementation

The IndicatifProgress struct will be enhanced to support MultiProgress operations:

1. **Shared MultiProgress**: All IndicatifProgress instances (parent and children) share a single `Arc<MultiProgress>` instance
2. **Individual Bar Ownership**: Each IndicatifProgress owns exactly one `ProgressBar` (named `owned_bar`)
3. **Uniform Structure**: Parent and child instances use the same struct - no `is_child` flag needed
4. **Bar Creation Timing**: ProgressBars are created and added to MultiProgress in the `start()` method, not in `new()`

**Critical Design Principle**: The shared MultiProgress manages all bars' display coordination, while each IndicatifProgress is responsible only for its own bar.

#### Implementation Details (Validated by Spike)

**Template Configuration**:
- Place `{spinner}` at the beginning of templates for line-start spinner display
- Use template pattern: `{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}`
- Enable steady tick with `enable_steady_tick(Duration::from_millis(80))`

**Visual Hierarchy**:
- Use `insert_after()` instead of `insert_before()` for logical parent-child relationships
- Child bars appear below parent bars, indented with spaces: `"  └─ {prefix}"`
- Simplify progress chars to `██░` to reduce visual noise

**API Considerations**:
- No `join()` method on MultiProgress (removed from newer versions)
- Progress bars are automatically rendered when added to MultiProgress
- Use `finish_and_clear()` to remove bars from display entirely

**Best Practices**:
- Keep titles short to prevent interference with progress bar display
- Add blank line after titles for better separation
- Place dynamic messages at the end of template: `{pos}/{len} {msg}`
- This keeps the progress bar layout stable while providing dynamic feedback

## Critical Implementation Requirements

Based on the investigation of display issues, the following requirements are mandatory for correct implementation:

### 1. Always Use MultiProgress for IndicatifProgress

```rust
impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            // CRITICAL: Always create MultiProgress upfront
            // Never use lazy creation as it causes display issues
            multi: Arc::new(MultiProgress::new()),
            owned_bar: None,
            template: "{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}".to_string(),
        }
    }
}
```

**Rationale**: Even single progress bars benefit from MultiProgress management for stability and consistent rendering.

### 2. Safe Log Output

The ProgressIndicator trait provides built-in methods for safe output:

```rust
// Safe log output using suspend method
progress.suspend(&mut || {
    log::info!("Processing step");
    log::debug!("Detailed information");
    println!("Additional output");
});

// Safe single-line output using println method
progress.println("Status: Processing item 5 of 10")?;
```

Implementation behavior:
- **IndicatifProgress**: Uses `MultiProgress::suspend()` and `MultiProgress::println()` for safe output
- **SimpleProgress**: Direct output without special handling
- **SilentProgress**: No output (methods are no-ops)

### 3. StatusReporter Coexistence Strategy

During the transition period:
1. **Keep StatusReporter intact** - No immediate removal
2. **Test ProgressIndicator in Install command first** - Validate the approach
3. **Gradual migration** - Update other commands after validation

#### StatusReporter Functionality Migration

StatusReporter's key features are migrated to ProgressIndicator as follows:

**Color/Symbol Detection Logic**:
- **StatusReporter**: `should_use_color()` checks `NO_COLOR`, `TERM=dumb`, and terminal support
- **ProgressIndicator**: `ProgressFactory::create()` performs the same checks to select appropriate implementation
  - Terminal with color support → `IndicatifProgress` (rich progress bars)
  - CI/NO_COLOR/dumb terminal → `SimpleProgress` (plain text output)
  - `--no-progress` flag → `SilentProgress` (no output)

**Success/Error Symbols**:
- **StatusReporter**: Switches between "✓"/"✗" (Unicode) and "[OK]"/"[ERROR]" (ASCII)
- **SimpleProgress**: Should use only ASCII symbols "[OK]"/"[ERROR]"
  - **Important**: Remove Unicode symbols ("✓"/"✗") from SimpleProgress implementation
  - SimpleProgress is selected only in environments where Unicode is problematic
  - The factory already ensures SimpleProgress is used in NO_COLOR/dumb terminal environments
- **IndicatifProgress**: Uses progress bar styling and animations (no explicit symbols needed)
- **SilentProgress**: No output (symbols not applicable)

This design ensures that StatusReporter's environment-aware behavior is preserved while simplifying the implementation.

### 4. Safe Log Output Methods

When using MultiProgress, utilize its safe output methods:
- **MultiProgress::println()** - Output text above progress bars
- **MultiProgress::suspend()** - Temporarily hide bars for external output
- Both methods prevent rendering conflicts

### Phase 3: Install Command Integration

The install command will be the first to test the ProgressIndicator approach while maintaining StatusReporter for compatibility.

#### Integration Steps

1. **Conditional StatusReporter usage**:
```rust
// Only use StatusReporter when using SilentProgress (--no-progress flag)
// SimpleProgress and IndicatifProgress handle their own output
if no_progress {
    self.status.operation("Installing", &format!("{} {}", distribution.name(), version));
}
```

2. **Download Operation**:
- When starting a download, check the Content-Length header
- If the size is 10MB or larger, create a child progress bar showing bytes downloaded and transfer rate
- If the size is smaller or unknown, only update the parent progress message
- The child progress bar updates continuously during the download and completes when finished

3. **Cache Refresh Operation**:
- When refreshing cache with Foojay API, always create a child progress bar
- Show the number of packages being processed
- Complete the child when cache refresh finishes

4. **Log Output Handling**:
- Use MultiProgress::suspend() for all log output (log::info!, log::debug!, println!, etc.)

### Phase 4: Cache Refresh Integration

The cache refresh command will implement child progress based on the metadata source type:

**Foojay API Source**:
- Always creates a child progress bar due to the operation typically taking 5-10 seconds
- Shows package processing count and updates as packages are fetched
- Completes with a summary of packages fetched

**HTTP Metadata Source**:
- First checks the file size through HEAD request or similar mechanism
- If 10MB or larger, creates a child progress bar showing download progress
- If smaller, processes without a child progress bar

**Local File Source**:
- Never creates a child progress bar as local file operations are typically instantaneous
- Updates only the parent progress message

## Visual Examples

### Problematic Patterns (To Avoid)

#### Duplicate Output
```
Installing Eclipse Temurin 21...
⠋ Installing Eclipse Temurin 21 [████░░░░░░░░░░░░░░] 3/7 Downloading
```
Both StatusReporter and ProgressIndicator showing the same information.

#### Split Progress Bar
```
Installing Eclipse Temurin 21...
⠋ Installing Eclipse Temurin 21 [
⠋ Installing Eclipse Temurin 21 [████░░░░░░░░░░░░░░] 3/7 Downloading temurin 21.0.8+9...
```
Progress bar split across two lines due to MultiProgress creation issue.

#### Log Interference
```
⠋ Installing Eclipse Temurin 21 [[2025-08-28T00:25:11Z INFO  kopi::install] Downloading package
```
Log output interrupting progress bar rendering.

### Correct Patterns (Target State)

#### Install Command with Large Download
```
⣾ Installing temurin@21 [████████████░░░░░░] 3/8 Downloading
  └─ ⣟ Downloading temurin-21.0.5: 124.5MB / 256.3MB [48%] 2.3MB/s
```

### Cache Refresh with Foojay
```
⣽ Refreshing metadata cache [██████████░░░░░░░░] 2/5 Fetching sources
  └─ ⣻ Fetching Foojay packages: 1543 / 3217 [47%]
```

### Small File (No Child Progress)
```
⣿ Installing temurin@21 [████████████░░░░░░] 3/8 Downloading package (5.2MB)
```

### Nested Progress with Multiple Steps
```
⡿ Parent Task [██████████████████░] 2/3 Processing step 2 of 3
  └─ Subtask 2 [███████████░░░░] 25/50
```

## ProgressIndicator Trait Extensions

The ProgressIndicator trait has been extended with safe output methods:

```rust
pub trait ProgressIndicator: Send + Sync {
    // Existing methods
    fn start(&mut self, config: ProgressConfig);
    fn update(&mut self, current: u64, total: Option<u64>);
    fn set_message(&mut self, message: String);
    fn complete(&mut self, message: Option<String>);
    fn error(&mut self, message: String);
    fn create_child(&mut self) -> Box<dyn ProgressIndicator>;
    
    // New safe output methods
    fn suspend(&self, f: &mut dyn FnMut());
    fn println(&self, message: &str) -> std::io::Result<()>;
}
```

These methods ensure safe output during progress operations without display corruption.

### Rationale for New Methods

1. **suspend()**: Allows safe execution of log output operations by temporarily suspending progress bar rendering
2. **println()**: Provides a safe alternative to `println!` that works correctly with active progress bars

### Usage Patterns

```rust
// During download operation with logging
progress.suspend(&mut || {
    log::info!("Download started from: {}", url);
    log::debug!("Headers: {:?}", headers);
});

// Quick status updates
progress.println("Cache hit - using local file")?;

// Error reporting with context
progress.suspend(&mut || {
    eprintln!("Warning: Retrying download (attempt {} of {})", attempt, max_retries);
});
```

### Implementation Details by Type

**IndicatifProgress**:
- `suspend()`: Delegates to `MultiProgress::suspend()` when MultiProgress is active
- `println()`: Delegates to `MultiProgress::println()` when MultiProgress is active

**SimpleProgress**:
- `suspend()`: Direct execution (no progress bars to suspend)
- `println()`: Direct `println!` (safe since no animated progress bars)

**SilentProgress**:
- `suspend()`: Direct execution (no output to manage)
- `println()`: No-op (maintains silent behavior)

## Reference Implementation

Based on the validated spike, the following structure is recommended:

```rust
pub struct IndicatifProgress {
    multi: Arc<MultiProgress>,          // Shared across all instances
    owned_bar: Option<ProgressBar>,     // This instance's progress bar
    template: String,                    // Template determined at construction
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            owned_bar: None,
            template: "{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}".to_string(),
        }
    }
    
    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        Box::new(IndicatifProgress {
            multi: Arc::clone(&self.multi),  // Share parent's MultiProgress
            owned_bar: None,                 // Will be created in start()
            template: "  └─ {spinner} {prefix} [{bar:25}] {pos}/{len} {msg}".to_string(),
        })
    }
    
    fn start(&mut self, config: ProgressConfig) {
        let pb = match config.total {
            Some(total) => ProgressBar::new(total),
            None => ProgressBar::new_spinner(),
        };
        
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&self.template)
                .unwrap()
                .progress_chars("██░")
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        );
        
        pb.enable_steady_tick(Duration::from_millis(80));
        
        // Add to MultiProgress
        self.multi.add(pb.clone());
        
        self.owned_bar = Some(pb);
    }
    
    fn update(&mut self, progress: u64) {
        if let Some(ref pb) = self.owned_bar {
            pb.set_position(progress);
        }
    }
    
    fn finish(&mut self) {
        if let Some(ref pb) = self.owned_bar {
            pb.finish_with_message("Complete");
        }
    }
}
```

## Testing Strategy

### Unit Tests
1. Test `create_child()` for each ProgressIndicator implementation
2. Test that SilentProgress children remain silent
3. Test that SimpleProgress children don't produce output
4. Test MultiProgress sharing between parent and child

### Integration Tests
1. Test Install command with various file sizes (< 10MB, > 10MB, unknown)
2. Test Cache refresh with different source types
3. Test in CI environment (should use SimpleProgress)
4. **Test with logging** - Verify suspend() prevents display corruption
5. **Test log output during progress** - Ensure no display corruption

### Manual Testing
1. Visual inspection of multi-progress display in terminal
2. Test terminal resize during multi-progress operation
3. Test Ctrl+C interruption handling
4. Test log output using suspend() method
5. Verify StatusReporter and ProgressIndicator don't conflict

## Best Practices

### 1. Always Use MultiProgress for Single Bars

Even when displaying a single progress bar, use MultiProgress for stability:

```rust
impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),  // Always create
            owned_bar: None,                        // Created in start()
            template: "{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}".to_string(),
        }
    }
```

### 2. Log Output Handling

Always use `suspend()` when logging during progress operations:

```rust
// Safe log output with suspend()
progress.multi.suspend(|| {
    log::info!("Processing step");
    log::debug!("Detailed information");
    println!("Additional output");
});
```

This temporarily pauses progress bar rendering to ensure clean output.

### 3. Completion Message Handling

Choose the appropriate finish method:

```rust
// Keep result visible
pb.finish_with_message("✓ Installation complete");

// Clear temporary progress
pb_child.finish_and_clear();
```

### 4. Post-Completion Output

After progress bars complete, both methods work:

```rust
// Via MultiProgress (if still available)
multi.println("Additional information").unwrap();

// Direct println (safe after completion)
println!("Installation details:");
println!("  Path: ~/.kopi/jdks/...");
```

### 5. StatusReporter Integration

During transition, avoid dual output:

```rust
// StatusReporter only needed when using SilentProgress
if no_progress {
    status_reporter.operation("Installing", "package");
} else {
    // SimpleProgress and IndicatifProgress handle their own output
}
```

## Migration Strategy

### Phased Approach with StatusReporter Coexistence

#### Phase 1: Infrastructure and Install Command (Current Focus)

**Timeline**: Immediate

1. **Update ProgressIndicator trait** - Add create_child(), suspend(), and println() methods ✅
2. **Fix IndicatifProgress** - Always initialize with MultiProgress
3. **Fix SimpleProgress** - Remove Unicode symbols ("✓"/"✗"), use ASCII only ("[OK]"/"[ERROR]")
4. **Use existing factory** - ProgressFactory::create already handles smart selection
5. **Test in Install command** - Validate ProgressIndicator approach
6. **Keep StatusReporter** - No removal, maintain compatibility

**Validation Criteria**:
- No display corruption in Install command
- Safe log output using suspend()
- StatusReporter doesn't conflict with ProgressIndicator

#### Phase 2: Expand to Other Commands

**Timeline**: After Install command validation

Commands to update (in order):
1. **Cache command** - Similar progress needs
2. **List command** - Simpler, good for testing
3. **Use/Local commands** - Minimal progress usage
4. **Shell/Current commands** - StatusReporter only

**Approach**:
```rust
// Gradual replacement pattern
if progress_indicator_validated {
    use_progress_indicator();
} else {
    keep_status_reporter();
}
```

#### Phase 3: Optimization and Cleanup

**Timeline**: After all commands updated

1. **Evaluate StatusReporter usage** - Identify remaining uses
2. **Consider deprecation** - Only if truly redundant
3. **Optimize ProgressIndicator usage** - Based on real usage patterns
4. **Document final patterns** - Update guidelines

### Rollback Plan

If issues arise:
1. **Keep StatusReporter active** - It's not being removed
2. **Revert to SimpleProgress** - For problematic scenarios
3. **Disable MultiProgress features** - Fall back to basic progress

### Success Metrics

- Zero display corruption issues
- Proper log/progress separation
- No performance regression
- Maintainable code structure

## Future Considerations

While not in current scope, the following could be considered later:

1. Configurable threshold (currently hardcoded to 10MB)
2. More than 1 level of nesting
3. Parallel progress bars for concurrent operations
4. Progress persistence for resumable operations

## Success Criteria

1. Users can see download progress during installation for large files
2. No terminal corruption or overlapping output
3. Performance overhead is negligible (< 1% CPU usage for progress updates)
4. CI/non-TTY environments continue to work without animations
5. Code complexity remains manageable

## Troubleshooting

### Common Issues and Solutions

#### Issue: Progress bar appears on two lines
```
⠋ Installing Eclipse Temurin 21 [
⠋ Installing Eclipse Temurin 21 [████░░░░] 3/7 Downloading...
```

**Cause**: MultiProgress was created lazily in create_child(), parent bar not added.

**Solution**: Ensure IndicatifProgress::new() creates MultiProgress immediately:
```rust
multi: Some(Arc::new(MultiProgress::new()))  // Not None
```

#### Issue: Log messages corrupt progress display
```
⠋ Installing [INFO kopi] Downloading package
```

**Cause**: log::info!/debug! outputs interfere with indicatif rendering.

**Solution**:
Use MultiProgress::suspend() for safe log output:
```rust
progress.multi.suspend(|| {
    log::info!("Safe output");
});
```

#### Issue: Both StatusReporter and progress bar show
```
Installing Eclipse Temurin 21...
⠋ Installing Eclipse Temurin 21 [████░░░░] 3/7
```

**Cause**: StatusReporter::operation() called when progress bar is active.

**Solution**: Conditionally use StatusReporter:
```rust
if no_progress {
    status_reporter.operation(...);
}
```

#### Issue: Progress bar doesn't update smoothly

**Cause**: Missing steady tick configuration.

**Solution**: Enable steady tick:
```rust
pb.enable_steady_tick(Duration::from_millis(80));
```

#### Issue: Child progress bars don't appear

**Cause**: Parent's MultiProgress not shared with child.

**Solution**: Ensure child receives parent's MultiProgress:
```rust
Box::new(IndicatifProgress {
    multi: Arc::clone(&self.multi),  // Share parent's MultiProgress
    owned_bar: None,
    template: "  └─ {spinner} {prefix} [{bar:25}] {pos}/{len} {msg}".to_string(),
})
```

### Debug Commands

```bash
# Test progress display
cargo run -- install temurin@21

# Test without progress
cargo run -- install temurin@21 --no-progress

# Test in CI mode
CI=1 cargo run -- install temurin@21
```

## Spike Validation Results

A comprehensive spike was conducted to validate this design. Key outcomes:

### Validated Capabilities
✅ **Thread Safety**: Concurrent updates work correctly with Arc<MultiProgress>
✅ **Dynamic Management**: Progress bars can be added/removed during execution
✅ **Visual Hierarchy**: Parent-child relationships display correctly with indentation
✅ **Performance**: No significant overhead observed
✅ **Template Stability**: Dynamic messages don't disrupt layout

### Key Implementation Insights
- `insert_after()` provides more logical parent-child positioning than `insert_before()`
- Spinner placement at template start (`{spinner}`) creates clean visual hierarchy
- Simplified progress chars (`██░`) reduce visual noise
- Short titles with blank line separation prevent display interference
- `enable_steady_tick()` essential for smooth spinner animation

### Ready for Implementation
The design has been thoroughly validated and is ready for Phase 3 implementation in the actual Kopi codebase. The spike demonstrates that all success criteria can be met with the proposed architecture.