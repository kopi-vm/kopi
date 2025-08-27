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
    multi: Option<Arc<MultiProgress>>,
    progress_bar: Option<ProgressBar>,
    is_child: bool,
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            multi: None,
            progress_bar: None,
            is_child: false,
        }
    }

    fn create_template(&self, config: &ProgressConfig) -> String {
        let prefix = if self.is_child { "  └─ " } else { "" };
        let bar_width = if self.is_child { 25 } else { 30 };

        match (&config.total, &config.style) {
            // Progress bar with bytes display
            (Some(_), ProgressStyle::Bytes) => {
                if self.is_child {
                    format!(
                        "  └─ {{spinner}} {{prefix}} [{{bar:{bar_width}}}] {{bytes}}/{{total_bytes}} {{msg}}"
                    )
                } else {
                    format!(
                        "{{spinner}} {{prefix}} [{{bar:{bar_width}}}] {{bytes}}/{{total_bytes}} {{msg}} ({{bytes_per_sec}}, {{eta}})"
                    )
                }
            }
            // Progress bar with count display
            (Some(_), ProgressStyle::Count) => {
                format!(
                    "{prefix}{{spinner}} {{prefix}} [{{bar:{bar_width}}}] {{pos}}/{{len}} {{msg}}"
                )
            }
            // Indeterminate operations (spinner only when total is None)
            (None, _) => {
                format!("{prefix}{{spinner}} {{prefix}} {{msg}}")
            }
        }
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

        // Create MultiProgress lazily if needed
        if self.multi.is_none() && !self.is_child {
            self.multi = Some(Arc::new(MultiProgress::new()));
        }

        let pb = match config.total {
            Some(total) => ProgressBar::new(total),
            None => ProgressBar::new_spinner(),
        };

        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(&self.create_template(&config))
                .unwrap()
                .progress_chars("██░")
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        );

        pb.set_prefix(prefix);
        pb.enable_steady_tick(Duration::from_millis(80));

        // Add to MultiProgress if available
        let pb = if let Some(multi) = &self.multi {
            if self.is_child {
                // For child bars, use insert_after for logical positioning
                // Since we don't have a reference to the parent bar here,
                // we'll use add() which adds to the end
                multi.add(pb)
            } else {
                multi.add(pb)
            }
        } else {
            pb
        };

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
            if self.is_child {
                pb.finish_and_clear();
            } else {
                pb.finish_with_message(msg);
            }
        }
    }

    fn error(&mut self, message: String) {
        if let Some(pb) = &self.progress_bar {
            if self.is_child {
                pb.abandon();
            } else {
                pb.abandon_with_message(format!("✗ {message}"));
            }
        }
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        // Create or share the MultiProgress instance
        let multi = if let Some(ref multi) = self.multi {
            Arc::clone(multi)
        } else {
            // If parent doesn't have MultiProgress yet, create one
            let new_multi = Arc::new(MultiProgress::new());
            self.multi = Some(Arc::clone(&new_multi));
            new_multi
        };

        Box::new(IndicatifProgress {
            multi: Some(multi),
            progress_bar: None,
            is_child: true,
        })
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
    fn test_create_child() {
        let mut parent = IndicatifProgress::new();

        let config = ProgressConfig::new("Parent", "task", ProgressStyle::Count).with_total(100);
        parent.start(config);

        let child = parent.create_child();
        assert!(parent.multi.is_some());

        // Verify child shares the same MultiProgress
        // We can't directly check this due to Box<dyn> but we can start the child
        let mut child = child;
        let child_config =
            ProgressConfig::new("Child", "subtask", ProgressStyle::Count).with_total(50);
        child.start(child_config);

        // Both parent and child should have progress bars
        assert!(parent.progress_bar.is_some());
    }

    #[test]
    fn test_multiple_children() {
        let mut parent = IndicatifProgress::new();

        let config = ProgressConfig::new("Parent", "main", ProgressStyle::Count).with_total(100);
        parent.start(config);

        // Create multiple children
        let mut child1 = parent.create_child();
        let mut child2 = parent.create_child();
        let mut child3 = parent.create_child();

        // Start all children
        child1.start(ProgressConfig::new("Child1", "task1", ProgressStyle::Count).with_total(25));
        child2.start(ProgressConfig::new("Child2", "task2", ProgressStyle::Count).with_total(50));
        child3.start(ProgressConfig::new("Child3", "task3", ProgressStyle::Count));

        // Update and complete children
        child1.update(25, None);
        child1.complete(None);

        child2.update(50, None);
        child2.complete(Some("Child 2 done".to_string()));

        child3.set_message("Processing...".to_string());
        child3.complete(None);

        parent.complete(Some("All done".to_string()));
    }

    #[test]
    fn test_child_with_error() {
        let mut parent = IndicatifProgress::new();

        let config =
            ProgressConfig::new("Parent", "operation", ProgressStyle::Bytes).with_total(1024);
        parent.start(config);

        let mut child = parent.create_child();
        child.start(
            ProgressConfig::new("Download", "file.zip", ProgressStyle::Bytes).with_total(512),
        );

        child.update(256, None);
        child.error("Connection timeout".to_string());

        parent.error("Failed due to child error".to_string());
    }

    #[test]
    fn test_child_spinner_without_total() {
        let mut parent = IndicatifProgress::new();

        // Parent with progress bar
        let config =
            ProgressConfig::new("Installing", "package", ProgressStyle::Count).with_total(5);
        parent.start(config);

        // Child with spinner (no total)
        let mut child = parent.create_child();
        child.start(ProgressConfig::new(
            "Extracting",
            "archive",
            ProgressStyle::Count,
        ));

        child.set_message("Processing files...".to_string());
        child.set_message("Nearly done...".to_string());
        child.complete(Some("Extraction complete".to_string()));

        parent.update(5, None);
        parent.complete(None);
    }

    #[test]
    fn test_nested_progress_depth() {
        let mut root = IndicatifProgress::new();

        root.start(ProgressConfig::new("Root", "level0", ProgressStyle::Count).with_total(100));

        // Create first level child
        let mut level1 = root.create_child();
        level1.start(ProgressConfig::new("Level1", "child", ProgressStyle::Count).with_total(50));

        // Although we support only single level nesting in practice,
        // test that creating a child of a child doesn't panic
        let mut level2 = level1.create_child();
        level2.start(ProgressConfig::new(
            "Level2",
            "grandchild",
            ProgressStyle::Count,
        ));

        level2.complete(None);
        level1.complete(None);
        root.complete(None);
    }

    #[test]
    fn test_child_template_has_indent() {
        let parent = IndicatifProgress::new();
        let child = IndicatifProgress {
            multi: None,
            progress_bar: None,
            is_child: true,
        };

        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count).with_total(10);
        let parent_template = parent.create_template(&config);
        let child_template = child.create_template(&config);

        // Parent template should not have indent
        assert!(!parent_template.starts_with("  └─"));

        // Child template should have indent
        assert!(child_template.contains("└─"));
    }

    #[test]
    fn test_child_cleanup_on_completion() {
        let mut parent = IndicatifProgress::new();

        parent.start(ProgressConfig::new("Parent", "main", ProgressStyle::Count).with_total(10));

        let mut child = parent.create_child();
        child.start(ProgressConfig::new("Child", "sub", ProgressStyle::Count).with_total(5));

        child.update(5, None);
        // Child should use finish_and_clear
        child.complete(None);

        parent.update(10, None);
        parent.complete(Some("Done".to_string()));

        assert!(parent.progress_bar.is_some());
    }

    #[test]
    fn test_parent_without_multiprogress_creates_on_child() {
        let mut parent = IndicatifProgress::new();
        assert!(parent.multi.is_none());

        // Don't start parent yet
        let _child = parent.create_child();

        // Creating a child should initialize MultiProgress
        assert!(parent.multi.is_some());
    }
}
