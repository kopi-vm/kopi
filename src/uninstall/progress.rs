// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::indicator::{ProgressConfig, ProgressFactory, ProgressIndicator, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Handle for a progress indicator that provides a ProgressBar-like interface
pub struct ProgressHandle {
    indicator: Arc<Mutex<Box<dyn ProgressIndicator>>>,
    is_finished: Arc<Mutex<bool>>,
    current_value: Arc<Mutex<u64>>,
    total: Option<u64>,
}

impl ProgressHandle {
    /// Enable steady tick updates (for compatibility)
    pub fn enable_steady_tick(&self, _interval: Duration) {
        // The new progress system handles ticking internally
    }

    /// Finish and clear the progress indicator
    pub fn finish_and_clear(&self) {
        if let Ok(mut indicator) = self.indicator.lock() {
            indicator.complete(None);
        }
        if let Ok(mut finished) = self.is_finished.lock() {
            *finished = true;
        }
    }

    /// Finish with a message
    pub fn finish_with_message(&self, message: String) {
        if let Ok(mut indicator) = self.indicator.lock() {
            indicator.complete(Some(message));
        }
        if let Ok(mut finished) = self.is_finished.lock() {
            *finished = true;
        }
    }

    /// Increment the progress (for progress bars)
    pub fn inc(&self, delta: u64) {
        if let Ok(mut current) = self.current_value.lock() {
            *current += delta;
            if let Ok(mut indicator) = self.indicator.lock() {
                indicator.update(*current, self.total);
            }
        }
    }

    /// Finish the progress indicator (for tests)
    pub fn finish(&self) {
        self.finish_and_clear();
    }

    /// Check if finished (for tests)
    pub fn is_finished(&self) -> bool {
        self.is_finished.lock().map(|f| *f).unwrap_or(false)
    }
}

/// Progress adapter for uninstall operations
pub struct ProgressReporter {
    no_progress: bool,
    progress_indicators: Vec<Arc<Mutex<Box<dyn ProgressIndicator>>>>,
}

impl ProgressReporter {
    /// Creates a new progress reporter for single operations
    pub fn new(no_progress: bool) -> Self {
        Self {
            no_progress,
            progress_indicators: Vec::new(),
        }
    }

    /// Creates a new progress reporter for batch operations
    pub fn new_batch(no_progress: bool) -> Self {
        Self {
            no_progress,
            progress_indicators: Vec::new(),
        }
    }

    /// Creates a progress indicator handle that mimics the old ProgressBar interface
    fn create_progress_handle(
        &mut self,
        indicator: Box<dyn ProgressIndicator>,
        total: Option<u64>,
    ) -> ProgressHandle {
        let arc_indicator = Arc::new(Mutex::new(indicator));
        self.progress_indicators.push(arc_indicator.clone());
        ProgressHandle {
            indicator: arc_indicator,
            is_finished: Arc::new(Mutex::new(false)),
            current_value: Arc::new(Mutex::new(0)),
            total,
        }
    }

    /// Creates a spinner progress bar for long-running operations
    pub fn create_spinner(&mut self, message: &str) -> ProgressHandle {
        let mut indicator = ProgressFactory::create(self.no_progress);
        let config = ProgressConfig::new("Processing", message, ProgressStyle::Count);
        indicator.start(config);
        self.create_progress_handle(indicator, None)
    }

    /// Creates a progress bar for operations with known total steps
    pub fn create_bar(&mut self, total: u64, message: &str) -> ProgressHandle {
        let mut indicator = ProgressFactory::create(self.no_progress);
        let config =
            ProgressConfig::new("Removing", message, ProgressStyle::Count).with_total(total);
        indicator.start(config);
        self.create_progress_handle(indicator, Some(total))
    }

    /// Creates a spinner for JDK removal operations with standardized message format
    pub fn create_jdk_removal_spinner(
        &mut self,
        jdk_path: &str,
        formatted_size: &str,
    ) -> ProgressHandle {
        let mut indicator = ProgressFactory::create(self.no_progress);
        let context = format!("{jdk_path} ({formatted_size})");
        let config = ProgressConfig::new("Removing", &context, ProgressStyle::Count);
        indicator.start(config);
        indicator.set_message("Preparing removal...".to_string());
        self.create_progress_handle(indicator, None)
    }

    /// Creates a progress bar for batch JDK removal operations
    pub fn create_batch_removal_bar(&mut self, total_jdks: u64) -> ProgressHandle {
        let mut indicator = ProgressFactory::create(self.no_progress);
        let config =
            ProgressConfig::new("Removing", "JDKs", ProgressStyle::Count).with_total(total_jdks);
        indicator.start(config);
        self.create_progress_handle(indicator, Some(total_jdks))
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_progress_reporter() {
        let reporter = ProgressReporter::new(false);
        assert!(!reporter.no_progress);
        assert!(reporter.progress_indicators.is_empty());
    }

    #[test]
    fn test_new_batch_progress_reporter() {
        let reporter = ProgressReporter::new_batch(false);
        assert!(!reporter.no_progress);
        assert!(reporter.progress_indicators.is_empty());
    }

    #[test]
    fn test_create_spinner() {
        let mut reporter = ProgressReporter::new(false);
        let spinner = reporter.create_spinner("Test message");
        assert!(!spinner.is_finished());
        spinner.finish();
        assert!(spinner.is_finished());
    }

    #[test]
    fn test_create_bar() {
        let mut reporter = ProgressReporter::new(false);
        let bar = reporter.create_bar(10, "Test items");
        assert!(!bar.is_finished());
        bar.finish();
        assert!(bar.is_finished());
    }

    #[test]
    fn test_create_jdk_removal_spinner() {
        let mut reporter = ProgressReporter::new(false);
        let spinner = reporter.create_jdk_removal_spinner("/test/jdk", "512 MB");
        assert!(!spinner.is_finished());
        spinner.finish();
        assert!(spinner.is_finished());
    }

    #[test]
    fn test_create_batch_removal_bar() {
        let mut reporter = ProgressReporter::new_batch(false);
        let bar = reporter.create_batch_removal_bar(5);
        assert!(!bar.is_finished());
        bar.finish();
        assert!(bar.is_finished());
    }
}
