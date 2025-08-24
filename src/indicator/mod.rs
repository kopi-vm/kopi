//! Progress indicator module for unified progress feedback
//!
//! This module provides a consistent interface for displaying progress
//! indicators across all Kopi operations. It supports different display
//! styles (animated progress bars, simple text, or silent) based on the
//! environment and user preferences.

pub mod types;

pub use types::{ProgressConfig, ProgressStyle};

/// Core trait for progress indicator implementations
///
/// This trait defines the interface that all progress indicator implementations
/// must provide. Implementations include:
/// - `IndicatifProgress` - Full animated progress bars and spinners for terminal environments
/// - `SimpleProgress` - Simple text output for non-terminal environments (CI/CD, logs)
/// - `SilentProgress` - No output (Null Object pattern) for --no-progress flag
pub trait ProgressIndicator: Send + Sync {
    /// Start a new progress operation
    ///
    /// This method initializes the progress indicator with the given configuration.
    /// For determinate operations (with total), a progress bar is shown.
    /// For indeterminate operations (without total), a spinner is shown.
    fn start(&mut self, config: ProgressConfig);

    /// Update progress for determinate operations
    ///
    /// # Arguments
    /// * `current` - Current progress value
    /// * `total` - Optional total value (can override the initial total)
    fn update(&mut self, current: u64, total: Option<u64>);

    /// Update the status message
    ///
    /// Changes the message displayed alongside the progress indicator.
    /// Useful for showing which item is currently being processed.
    fn set_message(&mut self, message: String);

    /// Complete the progress operation successfully
    ///
    /// # Arguments
    /// * `message` - Optional completion message (defaults to "Complete")
    fn complete(&mut self, message: Option<String>);

    /// Handle error completion
    ///
    /// Marks the operation as failed and displays an error message.
    /// The implementation should ensure the error is visible even in silent modes.
    fn error(&mut self, message: String);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock implementation for testing the trait
    struct MockProgress {
        started: bool,
        current: u64,
        total: Option<u64>,
        message: String,
        completed: bool,
        errored: bool,
    }

    impl MockProgress {
        fn new() -> Self {
            Self {
                started: false,
                current: 0,
                total: None,
                message: String::new(),
                completed: false,
                errored: false,
            }
        }
    }

    impl ProgressIndicator for MockProgress {
        fn start(&mut self, config: ProgressConfig) {
            self.started = true;
            self.total = config.total;
            self.message = format!("{} {}", config.operation, config.context);
        }

        fn update(&mut self, current: u64, total: Option<u64>) {
            self.current = current;
            if total.is_some() {
                self.total = total;
            }
        }

        fn set_message(&mut self, message: String) {
            self.message = message;
        }

        fn complete(&mut self, message: Option<String>) {
            self.completed = true;
            if let Some(msg) = message {
                self.message = msg;
            }
        }

        fn error(&mut self, message: String) {
            self.errored = true;
            self.message = message;
        }
    }

    #[test]
    fn test_trait_implementation() {
        let mut progress = MockProgress::new();

        // Test start
        let config = ProgressConfig::new("Testing", "mock", ProgressStyle::Count).with_total(100);
        progress.start(config);
        assert!(progress.started);
        assert_eq!(progress.total, Some(100));
        assert_eq!(progress.message, "Testing mock");

        // Test update
        progress.update(50, None);
        assert_eq!(progress.current, 50);
        assert_eq!(progress.total, Some(100));

        // Test update with new total
        progress.update(60, Some(200));
        assert_eq!(progress.current, 60);
        assert_eq!(progress.total, Some(200));

        // Test set_message
        progress.set_message("Processing item".to_string());
        assert_eq!(progress.message, "Processing item");

        // Test complete
        progress.complete(Some("Done!".to_string()));
        assert!(progress.completed);
        assert_eq!(progress.message, "Done!");
    }

    #[test]
    fn test_error_handling() {
        let mut progress = MockProgress::new();

        let config = ProgressConfig::new("Testing", "error", ProgressStyle::Count);
        progress.start(config);

        progress.error("Something went wrong".to_string());
        assert!(progress.errored);
        assert_eq!(progress.message, "Something went wrong");
    }

    #[test]
    fn test_indeterminate_progress() {
        let mut progress = MockProgress::new();

        // Start without total (indeterminate)
        let config = ProgressConfig::new("Loading", "data", ProgressStyle::Bytes);
        progress.start(config);
        assert!(progress.started);
        assert_eq!(progress.total, None);

        // Update without total should work
        progress.update(1000, None);
        assert_eq!(progress.current, 1000);
        assert_eq!(progress.total, None);
    }

    #[test]
    fn test_trait_object() {
        // Verify the trait can be used as a trait object
        let progress: Box<dyn ProgressIndicator> = Box::new(MockProgress::new());

        // This should compile, proving Send + Sync bounds work
        fn accept_progress(_p: Box<dyn ProgressIndicator>) {}
        accept_progress(progress);
    }
}
