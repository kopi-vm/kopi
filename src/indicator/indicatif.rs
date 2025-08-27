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
use indicatif::{MultiProgress, ProgressBar};
use std::sync::Arc;
use std::time::Duration;

pub struct IndicatifProgress {
    progress_bar: Option<ProgressBar>,
    multi_progress: Arc<MultiProgress>,
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            progress_bar: None,
            multi_progress: Arc::new(MultiProgress::new()),
        }
    }

    fn new_with_parent(multi_progress: Arc<MultiProgress>) -> Self {
        Self {
            progress_bar: None,
            multi_progress,
        }
    }

    fn create_template(&self, config: &ProgressConfig) -> String {
        match (&config.total, &config.style) {
            // Progress bar with bytes display
            (Some(_), ProgressStyle::Bytes) => {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                 {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})"
            }
            // Progress bar with count display
            (Some(_), ProgressStyle::Count) => {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                 {pos}/{len} {msg}"
            }
            // Indeterminate operations (spinner only when total is None)
            (None, _) => "{spinner:.green} [{elapsed_precise}] {msg}",
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

        let pb = self.multi_progress.add(pb);
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

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        Box::new(IndicatifProgress::new_with_parent(Arc::clone(
            &self.multi_progress,
        )))
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

    #[test]
    fn test_create_child_shares_multiprogress() {
        let mut parent = IndicatifProgress::new();

        let parent_config =
            ProgressConfig::new("Parent", "operation", ProgressStyle::Count).with_total(100);
        parent.start(parent_config);

        let mut child = parent.create_child();
        let child_config =
            ProgressConfig::new("Child", "task", ProgressStyle::Bytes).with_total(1024);
        child.start(child_config);

        parent.update(50, Some(100));
        child.update(512, Some(1024));

        child.complete(Some("Child done".to_string()));
        parent.complete(Some("All done".to_string()));
    }

    #[test]
    fn test_multiple_children() {
        let mut parent = IndicatifProgress::new();

        let parent_config =
            ProgressConfig::new("Parent", "main", ProgressStyle::Count).with_total(3);
        parent.start(parent_config);

        let mut child1 = parent.create_child();
        let config1 =
            ProgressConfig::new("Child1", "download1", ProgressStyle::Bytes).with_total(1000);
        child1.start(config1);

        let mut child2 = parent.create_child();
        let config2 =
            ProgressConfig::new("Child2", "download2", ProgressStyle::Bytes).with_total(2000);
        child2.start(config2);

        let mut child3 = parent.create_child();
        let config3 =
            ProgressConfig::new("Child3", "download3", ProgressStyle::Count).with_total(50);
        child3.start(config3);

        parent.update(1, Some(3));
        child1.update(500, Some(1000));
        child2.update(1000, Some(2000));
        child3.update(25, Some(50));

        child1.complete(Some("Download 1 complete".to_string()));
        parent.update(2, Some(3));

        child2.complete(Some("Download 2 complete".to_string()));
        child3.complete(Some("Download 3 complete".to_string()));
        parent.update(3, Some(3));

        parent.complete(Some("All downloads complete".to_string()));
    }

    #[test]
    fn test_child_with_error() {
        let mut parent = IndicatifProgress::new();

        let parent_config =
            ProgressConfig::new("Parent", "batch", ProgressStyle::Count).with_total(2);
        parent.start(parent_config);

        let mut child1 = parent.create_child();
        let config1 = ProgressConfig::new("Child1", "task1", ProgressStyle::Count);
        child1.start(config1);

        let mut child2 = parent.create_child();
        let config2 = ProgressConfig::new("Child2", "task2", ProgressStyle::Count);
        child2.start(config2);

        child1.complete(Some("Success".to_string()));
        child2.error("Failed to process".to_string());

        parent.complete(Some("Batch completed with errors".to_string()));
    }

    #[test]
    fn test_nested_progress_depth() {
        let mut parent = IndicatifProgress::new();

        let config = ProgressConfig::new("Level1", "parent", ProgressStyle::Count);
        parent.start(config);

        let mut child = parent.create_child();
        let child_config = ProgressConfig::new("Level2", "child", ProgressStyle::Count);
        child.start(child_config);

        let mut grandchild = child.create_child();
        let grandchild_config = ProgressConfig::new("Level3", "grandchild", ProgressStyle::Count);
        grandchild.start(grandchild_config);

        grandchild.complete(Some("Grandchild done".to_string()));
        child.complete(Some("Child done".to_string()));
        parent.complete(Some("Parent done".to_string()));
    }

    #[test]
    fn test_child_spinner() {
        let mut parent = IndicatifProgress::new();

        let parent_config =
            ProgressConfig::new("Parent", "processing", ProgressStyle::Count).with_total(100);
        parent.start(parent_config);

        let mut child = parent.create_child();
        let child_config = ProgressConfig::new("Child", "searching", ProgressStyle::Count);
        child.start(child_config);

        child.set_message("Searching files...".to_string());
        child.set_message("Found 10 files".to_string());
        child.complete(Some("Search complete".to_string()));

        parent.complete(Some("Processing complete".to_string()));
    }
}
