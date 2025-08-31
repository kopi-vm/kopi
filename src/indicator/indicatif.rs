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
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct IndicatifProgress {
    multi: Arc<MultiProgress>,      // Always initialized, no Option
    owned_bar: Option<ProgressBar>, // This instance's progress bar
    is_child: bool,                 // Whether this is a child progress
    last_update: Option<Instant>,   // Track last update time for throttling
    update_threshold: Duration,     // Minimum time between updates
}

impl IndicatifProgress {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            owned_bar: None,
            is_child: false,
            last_update: None,
            update_threshold: Duration::from_millis(50), // Update at most 20 times per second
        }
    }

    fn get_template_for_config(&self, config: &ProgressConfig) -> String {
        let bar_width = if self.is_child { 25 } else { 30 };

        match (&config.total, &config.style) {
            // Progress bar with bytes display
            (Some(_), ProgressStyle::Bytes) => {
                if self.is_child {
                    format!(
                        "  └─ {{spinner:.green}} [{{elapsed_precise}}] [{{bar:{bar_width}.cyan/blue}}] {{bytes}}/{{total_bytes}} {{msg}}"
                    )
                } else {
                    format!(
                        "{{spinner:.green}} [{{elapsed_precise}}] [{{bar:{bar_width}.cyan/blue}}] {{bytes}}/{{total_bytes}} {{msg}} ({{bytes_per_sec}}, {{eta}})"
                    )
                }
            }
            // Progress bar with count display
            (Some(_), ProgressStyle::Count) => {
                if self.is_child {
                    format!(
                        "  └─ {{spinner:.green}} [{{elapsed_precise}}] [{{bar:{bar_width}.cyan/blue}}] {{pos}}/{{len}} {{msg}}"
                    )
                } else {
                    format!(
                        "{{spinner:.green}} [{{elapsed_precise}}] [{{bar:{bar_width}.cyan/blue}}] {{pos}}/{{len}} {{msg}}"
                    )
                }
            }
            // Indeterminate operations (spinner only when total is None)
            (None, _) => {
                if self.is_child {
                    "  └─ {spinner:.green} [{elapsed_precise}] {msg}".to_string()
                } else {
                    "{spinner:.green} [{elapsed_precise}] {msg}".to_string()
                }
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
        // Reset tracking state for new operation
        self.last_update = None;

        let pb = match config.total {
            Some(total) => ProgressBar::new(total),
            None => ProgressBar::new_spinner(),
        };

        // Add bar to MultiProgress first to get the managed reference
        let pb = self.multi.add(pb);

        // Use the appropriate template based on config
        let template = self.get_template_for_config(&config);

        // Apply styling to the MultiProgress-managed ProgressBar
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(&template)
                .unwrap()
                .progress_chars("██░")
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        );

        // Use different tick rates for parent vs child to reduce CPU usage
        let tick_rate = if self.is_child {
            Duration::from_millis(120) // Slower tick for children (8.3 Hz)
        } else {
            Duration::from_millis(80) // Normal tick for parent (12.5 Hz)
        };
        pb.enable_steady_tick(tick_rate);

        self.owned_bar = Some(pb);
    }

    fn update(&mut self, current: u64, _total: Option<u64>) {
        if let Some(pb) = &self.owned_bar {
            // Throttle updates for child progress bars
            if self.is_child {
                let now = Instant::now();
                if let Some(last) = self.last_update
                    && now.duration_since(last) < self.update_threshold
                {
                    // Skip this update, too soon since last one
                    return;
                }
                self.last_update = Some(now);
            }

            pb.set_position(current);
        }
    }

    fn set_message(&mut self, message: String) {
        if let Some(pb) = &self.owned_bar {
            // Throttle message updates for child progress bars as well
            if self.is_child {
                let now = Instant::now();
                if let Some(last) = self.last_update
                    && now.duration_since(last) < self.update_threshold
                {
                    // Skip this message update, too soon since last one
                    return;
                }
                self.last_update = Some(now);
            }

            pb.set_message(message);
        }
    }

    fn complete(&mut self, message: Option<String>) {
        if let Some(pb) = self.owned_bar.take() {
            // Force final update by resetting last_update
            self.last_update = None;

            let msg = message.unwrap_or_else(|| "Complete".to_string());
            if self.is_child {
                pb.finish_and_clear();
            } else {
                pb.finish_with_message(msg);
            }
            // Note: We don't call multi.remove() per user's request
        }
    }

    fn error(&mut self, message: String) {
        if let Some(pb) = self.owned_bar.take() {
            // Force final update by resetting last_update
            self.last_update = None;

            if self.is_child {
                pb.abandon();
            } else {
                pb.abandon_with_message(format!("{} {message}", "✗".red().bold()));
            }
            // Note: We don't call multi.remove() per user's request
        }
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        // Share parent's MultiProgress via Arc::clone()
        Box::new(IndicatifProgress {
            multi: Arc::clone(&self.multi),
            owned_bar: None,
            is_child: true,
            last_update: None,
            update_threshold: Duration::from_millis(100), // Children update less frequently (10 Hz)
        })
    }

    fn suspend(&self, f: &mut dyn FnMut()) {
        self.multi.suspend(f);
    }

    fn success(&self, message: &str) -> std::io::Result<()> {
        let formatted = format!("{} {message}", "✓".green().bold());
        self.println(&formatted)
    }

    fn println(&self, message: &str) -> std::io::Result<()> {
        if let Some(pb) = &self.owned_bar {
            pb.println(message);
        } else {
            // If no progress bar is active, print directly
            println!("{message}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Bytes).with_total(1024);
        progress.start(config);

        assert!(progress.owned_bar.is_some());
    }

    #[test]
    fn test_spinner_creation() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);

        assert!(progress.owned_bar.is_some());
    }

    #[test]
    fn test_template_selection_bytes_with_total() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new(ProgressStyle::Bytes).with_total(100);

        let template = progress.get_template_for_config(&config);
        assert!(template.contains("{bytes}"));
        assert!(template.contains("{total_bytes}"));
        assert!(template.contains("{bytes_per_sec}"));
        assert!(template.contains("[{elapsed_precise}]"));
        assert!(template.contains("{spinner:.green}"));
        assert!(template.contains(".cyan/blue}"));
    }

    #[test]
    fn test_template_selection_count_with_total() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);

        let template = progress.get_template_for_config(&config);
        assert!(template.contains("{pos}"));
        assert!(template.contains("{len}"));
        assert!(!template.contains("{bytes}"));
        assert!(template.contains("[{elapsed_precise}]"));
        assert!(template.contains("{spinner:.green}"));
        assert!(template.contains(".cyan/blue}"));
    }

    #[test]
    fn test_template_selection_indeterminate() {
        let progress = IndicatifProgress::new();
        let config = ProgressConfig::new(ProgressStyle::Bytes);

        let template = progress.get_template_for_config(&config);
        assert!(!template.contains("{bar:"));
        assert!(template.contains("{spinner:.green}"));
        assert!(!template.contains("{bytes}"));
        assert!(template.contains("[{elapsed_precise}]"));
    }

    #[test]
    fn test_update_behavior() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        progress.start(config);

        // Should not panic
        progress.update(25, Some(100));
        progress.update(50, None);
        progress.update(75, Some(100));
    }

    #[test]
    fn test_message_updates() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);

        // Should not panic
        progress.set_message("Step 1".to_string());
        progress.set_message("Step 2".to_string());
        progress.set_message("Step 3".to_string());
    }

    #[test]
    fn test_complete_with_message() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        progress.start(config);

        progress.update(100, None);
        progress.complete(Some("Installation successful".to_string()));

        // Progress bar should be None after completion (taken by complete())
        assert!(progress.owned_bar.is_none());
    }

    #[test]
    fn test_complete_without_message() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);

        progress.complete(None);

        // Progress bar should be None after completion (taken by complete())
        assert!(progress.owned_bar.is_none());
    }

    #[test]
    fn test_error_handling() {
        let mut progress = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Bytes);
        progress.start(config);

        progress.error("Network timeout".to_string());

        // Progress bar should be None after error (taken by error())
        assert!(progress.owned_bar.is_none());
    }

    #[test]
    fn test_multiple_operations() {
        let mut progress = IndicatifProgress::new();

        // First operation
        let config1 = ProgressConfig::new(ProgressStyle::Bytes).with_total(1000);
        progress.start(config1);
        progress.update(500, None);
        progress.complete(None);

        // Second operation (reuses same struct)
        let config2 = ProgressConfig::new(ProgressStyle::Count).with_total(50);
        progress.start(config2);
        progress.update(25, None);
        assert!(progress.owned_bar.is_some()); // Should exist before completion
        progress.complete(Some("Done".to_string()));

        assert!(progress.owned_bar.is_none()); // Should be None after completion (taken by complete())
    }

    #[test]
    fn test_create_child() {
        let mut parent = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        parent.start(config);

        let _child = parent.create_child();
        // Parent always has multi initialized
        assert!(Arc::strong_count(&parent.multi) > 1); // Child shares the Arc

        // Verify child shares the same MultiProgress
        // We can't directly check this due to Box<dyn> but we can start the child
        let mut child = parent.create_child();
        let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(50);
        child.start(child_config);

        // Both parent and child should have progress bars
        assert!(parent.owned_bar.is_some());
    }

    #[test]
    fn test_multiple_children() {
        let mut parent = IndicatifProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        parent.start(config);

        // Create multiple children
        let mut child1 = parent.create_child();
        let mut child2 = parent.create_child();
        let mut child3 = parent.create_child();

        // Start all children
        child1.start(ProgressConfig::new(ProgressStyle::Count).with_total(25));
        child2.start(ProgressConfig::new(ProgressStyle::Count).with_total(50));
        child3.start(ProgressConfig::new(ProgressStyle::Count));

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

        let config = ProgressConfig::new(ProgressStyle::Bytes).with_total(1024);
        parent.start(config);

        let mut child = parent.create_child();
        child.start(ProgressConfig::new(ProgressStyle::Bytes).with_total(512));

        child.update(256, None);
        child.error("Connection timeout".to_string());

        parent.error("Failed due to child error".to_string());
    }

    #[test]
    fn test_child_spinner_without_total() {
        let mut parent = IndicatifProgress::new();

        // Parent with progress bar
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
        parent.start(config);

        // Child with spinner (no total)
        let mut child = parent.create_child();
        child.start(ProgressConfig::new(ProgressStyle::Count));

        child.set_message("Processing files...".to_string());
        child.set_message("Nearly done...".to_string());
        child.complete(Some("Extraction complete".to_string()));

        parent.update(5, None);
        parent.complete(None);
    }

    #[test]
    fn test_nested_progress_depth() {
        let mut root = IndicatifProgress::new();

        root.start(ProgressConfig::new(ProgressStyle::Count).with_total(100));

        // Create first level child
        let mut level1 = root.create_child();
        level1.start(ProgressConfig::new(ProgressStyle::Count).with_total(50));

        // Although we support only single level nesting in practice,
        // test that creating a child of a child doesn't panic
        let mut level2 = level1.create_child();
        level2.start(ProgressConfig::new(ProgressStyle::Count));

        level2.complete(None);
        level1.complete(None);
        root.complete(None);
    }

    #[test]
    fn test_child_template_has_indent() {
        let parent = IndicatifProgress::new();
        let child = IndicatifProgress {
            multi: Arc::new(MultiProgress::new()),
            owned_bar: None,
            is_child: true,
            last_update: None,
            update_threshold: Duration::from_millis(100),
        };

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(10);
        let parent_template = parent.get_template_for_config(&config);
        let child_template = child.get_template_for_config(&config);

        // Parent template should not have indent
        assert!(!parent_template.starts_with("  └─"));

        // Child template should have indent
        assert!(child_template.contains("└─"));

        // Both should have green spinner and cyan/blue progress bar
        assert!(parent_template.contains("{spinner:.green}"));
        assert!(child_template.contains("{spinner:.green}"));
        assert!(parent_template.contains(".cyan/blue}"));
        assert!(child_template.contains(".cyan/blue}"));

        // Both should have brackets around elapsed time
        assert!(parent_template.contains("[{elapsed_precise}]"));
        assert!(child_template.contains("[{elapsed_precise}]"));
    }

    #[test]
    fn test_child_cleanup_on_completion() {
        let mut parent = IndicatifProgress::new();

        parent.start(ProgressConfig::new(ProgressStyle::Count).with_total(10));

        let mut child = parent.create_child();
        child.start(ProgressConfig::new(ProgressStyle::Count).with_total(5));

        child.update(5, None);
        // Child should use finish_and_clear
        child.complete(None);

        parent.update(10, None);
        parent.complete(Some("Done".to_string()));

        assert!(parent.owned_bar.is_none()); // Should be None after completion (taken by complete())
    }

    #[test]
    fn test_parent_always_has_multiprogress() {
        let parent = IndicatifProgress::new();
        // MultiProgress is always initialized in new()
        assert!(Arc::strong_count(&parent.multi) == 1);

        let mut parent = parent;
        // Create child
        let _child = parent.create_child();

        // After creating a child, reference count should increase
        assert!(Arc::strong_count(&parent.multi) > 1);
    }
}
