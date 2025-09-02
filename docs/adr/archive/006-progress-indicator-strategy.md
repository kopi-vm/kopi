# ADR-006: Progress Indicator Strategy for Downloads and Archive Extraction

## Status
Proposed

## Context
Kopi downloads JDK distributions from foojay.io and extracts tar/zip archives during the installation process. Users need visual feedback during these potentially long-running operations to understand progress and ensure the application hasn't frozen. We need a strategy for implementing progress indicators that works well with our existing HTTP client (attohttpc) and archive handling libraries.

## Decision

### Progress Indicator Library
We will use **indicatif** as our progress indicator library for the following reasons:

1. **Maturity and Maintenance**: Most popular Rust progress bar library with active maintenance
2. **Feature-Rich**: Supports multiple progress bar styles, spinners, multi-progress displays, and human-readable formatters
3. **Cross-Platform**: Works consistently across Windows, macOS, and Linux
4. **Terminal Detection**: Automatically handles non-TTY environments gracefully
5. **Performance**: Provides built-in throttling mechanisms to avoid performance overhead

### Architecture

#### Download Progress
Since attohttpc doesn't provide streaming response capabilities suitable for progress tracking, we'll implement a hybrid approach:

1. **Small Files (< 50MB)**: Download to memory, then write with progress
2. **Large Files**: Consider migrating to reqwest for specific download operations that require progress tracking
3. **Fallback**: Use spinners for indeterminate progress when size is unknown

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;

pub trait ProgressReporter: Send + Sync {
    fn start_download(&self, url: &str, size: Option<u64>) -> ProgressHandle;
    fn start_extraction(&self, archive_type: &str, total_files: Option<u64>) -> ProgressHandle;
    fn finish(&self, handle: ProgressHandle);
}

pub struct IndicatifProgressReporter {
    multi_progress: MultiProgress,
}

impl IndicatifProgressReporter {
    pub fn new() -> Self {
        Self {
            multi_progress: MultiProgress::new(),
        }
    }
    
    fn create_download_progress(&self, size: Option<u64>) -> ProgressBar {
        let pb = match size {
            Some(size) => {
                let pb = self.multi_progress.add(ProgressBar::new(size));
                pb.set_style(ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                    .progress_chars("#>-"));
                pb
            }
            None => {
                let pb = self.multi_progress.add(ProgressBar::new_spinner());
                pb.set_style(ProgressStyle::default_spinner()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] {bytes} ({bytes_per_sec})"));
                pb
            }
        };
        pb
    }
}
```

#### Archive Extraction Progress
For archive extraction, we'll wrap file readers with progress tracking:

```rust
// TAR.GZ extraction with progress
pub fn extract_tar_gz_with_progress<P: AsRef<Path>>(
    archive_path: P,
    dest_dir: P,
    progress: &dyn ProgressReporter,
) -> Result<()> {
    let file = File::open(&archive_path)?;
    let file_size = file.metadata()?.len();
    
    let handle = progress.start_extraction("tar.gz", Some(file_size));
    
    // Wrap the file reader with progress tracking
    let progress_reader = handle.wrap_read(file);
    let tar = GzDecoder::new(progress_reader);
    let mut archive = Archive::new(tar);
    
    archive.unpack(&dest_dir)?;
    progress.finish(handle);
    
    Ok(())
}

// ZIP extraction with per-file progress
pub fn extract_zip_with_progress<P: AsRef<Path>>(
    archive_path: P,
    dest_dir: P,
    progress: &dyn ProgressReporter,
) -> Result<()> {
    let file = File::open(&archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    let handle = progress.start_extraction("zip", Some(archive.len() as u64));
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        // Extract file with progress updates
        handle.inc(1);
    }
    
    progress.finish(handle);
    Ok(())
}
```

### Implementation Strategy

#### Phase 1: Basic Progress Indicators
- Implement spinners for all long-running operations
- Add elapsed time display
- Ensure proper cleanup on errors

#### Phase 2: Detailed Progress Tracking
- Implement byte-level progress for downloads (where possible)
- Add file count progress for archive extraction
- Show current file being extracted

#### Phase 3: Enhanced User Experience
- Multi-progress for concurrent operations
- ETA calculations for large downloads
- Transfer speed display
- Customizable verbosity levels

### Performance Considerations

1. **Update Throttling**: Update progress bars at most every 100ms to avoid terminal flooding
2. **Separate Thread**: For CPU-intensive operations, update progress in a separate thread
3. **Conditional Display**: Only show progress bars for operations taking longer than 1 second
4. **Resource Cleanup**: Ensure progress bars are properly finished even on errors

### Error Handling

Progress indicators must not interfere with error handling:

```rust
// Ensure cleanup on all paths
let result = download_with_progress(url, path, &progress_reporter);
match result {
    Ok(_) => progress_reporter.finish_with_message("Download complete"),
    Err(e) => {
        progress_reporter.finish_with_error();
        return Err(e);
    }
}
```

### Configuration

Progress indicator behavior will be configurable through the global configuration file (`~/.kopi/config.toml`):

```toml
[progress]
# Enable/disable progress indicators (default: true)
enabled = true

# Progress indicator style: "bar", "spinner", or "simple" (default: "bar")
style = "bar"

# Show transfer speeds for downloads (default: true)
show_speed = true

# Show ETA for long operations (default: true)
show_eta = true

# Minimum operation duration before showing progress (milliseconds, default: 1000)
min_duration = 1000
```

The configuration will be loaded at startup and applied to all progress operations. Users can modify these settings to customize their experience without needing to restart their shell or set environment variables.

## Consequences

### Positive
- Improved user experience with visual feedback for long operations
- Cross-platform compatibility maintained
- Minimal performance overhead with proper throttling
- Graceful degradation in non-TTY environments

### Negative
- Additional dependency (indicatif and its transitive dependencies)
- Slightly increased binary size
- Need to carefully manage progress bar lifecycle to avoid visual artifacts
- attohttpc limitations may require workarounds or partial migration to reqwest

### Alternatives Considered

1. **No Progress Indicators**: Simpler but poor user experience
2. **Custom Implementation**: More control but significant development effort
3. **progress-streams**: Lower level but requires more boilerplate
4. **linenoise/termion**: Too low-level for our needs

## References
- [indicatif documentation](https://docs.rs/indicatif/)
- [attohttpc streaming limitations](https://github.com/sbstp/attohttpc/issues)
- [Rust CLI book - Output for humans and machines](https://rust-cli.github.io/book/tutorial/output.html)