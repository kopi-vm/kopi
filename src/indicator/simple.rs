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

use crate::indicator::{ProgressConfig, ProgressIndicator};

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

impl Default for SimpleProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressIndicator for SimpleProgress {
    fn start(&mut self, config: ProgressConfig) {
        // Don't print on start to avoid duplication with StatusReporter
        // The complete() method will show the final status
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::ProgressStyle;
    use serial_test::serial;
    use std::sync::Mutex;

    // Helper to capture stdout/stderr for testing
    static OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub struct TestProgress {
        inner: SimpleProgress,
    }

    impl TestProgress {
        pub fn new() -> Self {
            Self {
                inner: SimpleProgress::new(),
            }
        }

        pub fn get_output() -> Vec<String> {
            OUTPUT.lock().unwrap().clone()
        }

        pub fn clear_output() {
            OUTPUT.lock().unwrap().clear();
        }
    }

    impl ProgressIndicator for TestProgress {
        fn start(&mut self, config: ProgressConfig) {
            let msg = format!("{} {}...", config.operation, config.context);
            OUTPUT.lock().unwrap().push(msg);
            self.inner.operation = config.operation;
            self.inner.context = config.context;
        }

        fn update(&mut self, current: u64, total: Option<u64>) {
            self.inner.update(current, total);
        }

        fn set_message(&mut self, message: String) {
            self.inner.set_message(message);
        }

        fn complete(&mut self, message: Option<String>) {
            let msg = message.unwrap_or_else(|| "Complete".to_string());
            let output = format!(
                "✓ {} {} - {}",
                self.inner.operation, self.inner.context, msg
            );
            OUTPUT.lock().unwrap().push(output);
        }

        fn error(&mut self, message: String) {
            let output = format!(
                "✗ {} {} - {}",
                self.inner.operation, self.inner.context, message
            );
            OUTPUT.lock().unwrap().push(output);
        }
    }

    #[test]
    #[serial]
    fn test_message_output_format() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Installing", "temurin@21", ProgressStyle::Count);
        progress.start(config);
        progress.complete(Some("Done".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "Installing temurin@21...");
        assert_eq!(output[1], "✓ Installing temurin@21 - Done");
    }

    #[test]
    fn test_state_management() {
        let mut progress = SimpleProgress::new();

        assert_eq!(progress.operation, "");
        assert_eq!(progress.context, "");

        let config = ProgressConfig::new("Downloading", "JDK", ProgressStyle::Bytes);
        progress.start(config);

        assert_eq!(progress.operation, "Downloading");
        assert_eq!(progress.context, "JDK");
    }

    #[test]
    #[serial]
    fn test_error_handling() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Extracting", "archive", ProgressStyle::Count);
        progress.start(config);
        progress.error("Failed to extract".to_string());

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "✗ Extracting archive - Failed to extract");
    }

    #[test]
    fn test_complete_with_message() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Caching", "metadata", ProgressStyle::Count);
        progress.start(config);
        progress.complete(Some("Successfully cached".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "✓ Caching metadata - Successfully cached");
    }

    #[test]
    fn test_complete_without_message() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Processing", "data", ProgressStyle::Count);
        progress.start(config);
        progress.complete(None);

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "✓ Processing data - Complete");
    }

    #[test]
    fn test_update_no_output() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Loading", "files", ProgressStyle::Count).with_total(100);
        progress.start(config);

        // Updates should not produce output
        for i in 0..10 {
            progress.update(i * 10, Some(100));
        }

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 1); // Only the start message
    }

    #[test]
    fn test_set_message_no_output() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new("Scanning", "directories", ProgressStyle::Count);
        progress.start(config);

        // Set message should not produce output
        progress.set_message("Processing file 1".to_string());
        progress.set_message("Processing file 2".to_string());

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 1); // Only the start message
    }

    #[test]
    fn test_multiple_operations() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        // First operation
        let config1 = ProgressConfig::new("Operation1", "target1", ProgressStyle::Bytes);
        progress.start(config1);
        progress.complete(None);

        // Second operation
        let config2 = ProgressConfig::new("Operation2", "target2", ProgressStyle::Count);
        progress.start(config2);
        progress.complete(Some("Done".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], "Operation1 target1...");
        assert_eq!(output[1], "✓ Operation1 target1 - Complete");
        assert_eq!(output[2], "Operation2 target2...");
        assert_eq!(output[3], "✓ Operation2 target2 - Done");
    }
}
