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

use crate::indicator::{ProgressConfig, ProgressIndicator, ProgressStyle};
use indicatif::ProgressBar;
use std::time::Duration;

pub struct IndicatifProgress {
    progress_bar: Option<ProgressBar>,
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self { progress_bar: None }
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
            (None, _) => "{prefix}\n{spinner:.green} [{elapsed_precise}] {msg}",
        }
        .to_string()
    }
}

impl Default for IndicatifProgress {
    fn default() -> Self {
        Self::new()
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
                .progress_chars("█▓░")
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
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
            pb.abandon_with_message(format!("✗ {message}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let mut progress = IndicatifProgress::new();

        let config =
            ProgressConfig::new("Downloading", "temurin@21", ProgressStyle::Bytes).with_total(1024);
        progress.start(config);

        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_spinner_creation() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new("Loading", "metadata", ProgressStyle::Count);
        progress.start(config);

        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_template_selection_bytes_with_total() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new("Test", "operation", ProgressStyle::Bytes).with_total(100);

        let template = progress.create_template(&config);
        assert!(template.contains("{bytes}"));
        assert!(template.contains("{total_bytes}"));
        assert!(template.contains("{bytes_per_sec}"));
    }

    #[test]
    fn test_template_selection_count_with_total() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new("Test", "operation", ProgressStyle::Count).with_total(100);

        let template = progress.create_template(&config);
        assert!(template.contains("{pos}"));
        assert!(template.contains("{len}"));
        assert!(!template.contains("{bytes}"));
    }

    #[test]
    fn test_template_selection_indeterminate() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new("Test", "operation", ProgressStyle::Bytes);

        let template = progress.create_template(&config);
        assert!(!template.contains("{bar:"));
        assert!(template.contains("{spinner"));
        assert!(!template.contains("{bytes}"));
    }

    #[test]
    fn test_update_behavior() {
        let mut progress = IndicatifProgress::new();

        let config =
            ProgressConfig::new("Processing", "files", ProgressStyle::Count).with_total(100);
        progress.start(config);

        // Should not panic
        progress.update(25, Some(100));
        progress.update(50, None);
        progress.update(75, Some(100));
    }

    #[test]
    fn test_message_updates() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new("Working", "task", ProgressStyle::Count);
        progress.start(config);

        // Should not panic
        progress.set_message("Step 1".to_string());
        progress.set_message("Step 2".to_string());
        progress.set_message("Step 3".to_string());
    }

    #[test]
    fn test_complete_with_message() {
        let mut progress = IndicatifProgress::new();

        let config =
            ProgressConfig::new("Installing", "package", ProgressStyle::Count).with_total(100);
        progress.start(config);

        progress.update(100, None);
        progress.complete(Some("Installation successful".to_string()));

        // Progress bar should still exist after completion
        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_complete_without_message() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new("Building", "project", ProgressStyle::Count);
        progress.start(config);

        progress.complete(None);

        // Progress bar should still exist after completion
        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_error_handling() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new("Fetching", "data", ProgressStyle::Bytes);
        progress.start(config);

        progress.error("Network timeout".to_string());

        // Progress bar should still exist after error
        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_multiple_operations() {
        let mut progress = IndicatifProgress::new();

        // First operation
        let config1 = ProgressConfig::new("Op1", "target1", ProgressStyle::Bytes).with_total(1000);
        progress.start(config1);
        progress.update(500, None);
        progress.complete(None);

        // Second operation (reuses same struct)
        let config2 = ProgressConfig::new("Op2", "target2", ProgressStyle::Count).with_total(50);
        progress.start(config2);
        progress.update(25, None);
        progress.complete(Some("Done".to_string()));

        assert!(progress.progress_bar.is_some());
    }
}
