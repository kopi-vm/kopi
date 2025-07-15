/// Progress reporting abstraction for uninstall operations
///
/// This module provides a unified interface for creating and managing progress bars
/// during uninstall operations, eliminating code duplication across uninstall modules.
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// A progress reporter that creates standardized progress bars for different types of operations
pub struct ProgressReporter {
    multi_progress: Option<MultiProgress>,
}

impl ProgressReporter {
    /// Creates a new progress reporter for single operations
    pub fn new() -> Self {
        Self {
            multi_progress: None,
        }
    }

    /// Creates a new progress reporter for batch operations
    pub fn new_batch() -> Self {
        Self {
            multi_progress: Some(MultiProgress::new()),
        }
    }

    /// Creates a spinner progress bar for long-running operations
    ///
    /// # Arguments
    /// * `message` - The message to display with the spinner
    ///
    /// # Returns
    /// A configured spinner progress bar
    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let spinner = match &self.multi_progress {
            Some(multi) => multi.add(ProgressBar::new_spinner()),
            None => ProgressBar::new_spinner(),
        };

        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.set_message(message.to_string());
        spinner
    }

    /// Creates a progress bar for operations with known total steps
    ///
    /// # Arguments
    /// * `total` - The total number of items to process
    /// * `message` - The message template to display
    ///
    /// # Returns
    /// A configured progress bar
    pub fn create_bar(&self, total: u64, message: &str) -> ProgressBar {
        let pb = match &self.multi_progress {
            Some(multi) => multi.add(ProgressBar::new(total)),
            None => ProgressBar::new(total),
        };

        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Creates a spinner for JDK removal operations with standardized message format
    ///
    /// # Arguments
    /// * `jdk_path` - The path to the JDK being removed
    /// * `formatted_size` - The formatted size string (e.g., "512 MB")
    ///
    /// # Returns
    /// A configured spinner for JDK removal
    pub fn create_jdk_removal_spinner(&self, jdk_path: &str, formatted_size: &str) -> ProgressBar {
        let message = format!("Removing {jdk_path} ({formatted_size})...");
        self.create_spinner(&message)
    }

    /// Creates a progress bar for batch JDK removal operations
    ///
    /// # Arguments
    /// * `total_jdks` - The total number of JDKs to remove
    ///
    /// # Returns
    /// A configured progress bar for batch removal
    pub fn create_batch_removal_bar(&self, total_jdks: u64) -> ProgressBar {
        self.create_bar(total_jdks, "JDKs removed")
    }

    /// Gets the underlying MultiProgress instance for advanced usage
    ///
    /// # Returns
    /// An optional reference to the MultiProgress instance
    pub fn multi_progress(&self) -> Option<&MultiProgress> {
        self.multi_progress.as_ref()
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_progress_reporter() {
        let reporter = ProgressReporter::new();
        assert!(reporter.multi_progress.is_none());
    }

    #[test]
    fn test_new_batch_progress_reporter() {
        let reporter = ProgressReporter::new_batch();
        assert!(reporter.multi_progress.is_some());
    }

    #[test]
    fn test_create_spinner() {
        let reporter = ProgressReporter::new();
        let spinner = reporter.create_spinner("Test message");
        assert!(!spinner.is_finished());
        spinner.finish();
    }

    #[test]
    fn test_create_bar() {
        let reporter = ProgressReporter::new();
        let bar = reporter.create_bar(10, "Test items");
        assert!(!bar.is_finished());
        bar.finish();
    }

    #[test]
    fn test_create_jdk_removal_spinner() {
        let reporter = ProgressReporter::new();
        let spinner = reporter.create_jdk_removal_spinner("/test/jdk", "512 MB");
        assert!(!spinner.is_finished());
        spinner.finish();
    }

    #[test]
    fn test_create_batch_removal_bar() {
        let reporter = ProgressReporter::new_batch();
        let bar = reporter.create_batch_removal_bar(5);
        assert!(!bar.is_finished());
        bar.finish();
    }
}
