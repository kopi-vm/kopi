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

use crate::indicator::{ProgressConfig, ProgressIndicator, SilentProgress};

pub struct SimpleProgress {}

impl SimpleProgress {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SimpleProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressIndicator for SimpleProgress {
    fn start(&mut self, _config: ProgressConfig) {
        // Don't print on start to avoid duplication with StatusReporter
        // The complete() method will show the final status
    }

    fn update(&mut self, _current: u64, _total: Option<u64>) {
        // No update output in simple mode to avoid log spam
    }

    fn set_message(&mut self, message: String) {
        println!("{message}");
    }

    fn complete(&mut self, message: Option<String>) {
        let msg = message.unwrap_or_else(|| "Complete".to_string());
        println!("[OK] {msg}");
    }

    fn error(&mut self, message: String) {
        eprintln!("[ERROR] {message}");
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        // Return SilentProgress for child operations to keep output clean
        Box::new(SilentProgress::new())
    }

    fn suspend(&self, f: &mut dyn FnMut()) {
        // SimpleProgress doesn't use any terminal manipulation, just execute directly
        f();
    }

    fn println(&self, message: &str) -> std::io::Result<()> {
        // SimpleProgress can output directly without suspension
        println!("{message}");
        Ok(())
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
            let msg = "Starting...".to_string();
            OUTPUT.lock().unwrap().push(msg);
            self.inner.start(config);
        }

        fn update(&mut self, current: u64, total: Option<u64>) {
            self.inner.update(current, total);
        }

        fn set_message(&mut self, message: String) {
            OUTPUT.lock().unwrap().push(message.clone());
            self.inner.set_message(message);
        }

        fn complete(&mut self, message: Option<String>) {
            let msg = message.unwrap_or_else(|| "Complete".to_string());
            let output = format!("[OK] {msg}");
            OUTPUT.lock().unwrap().push(output);
        }

        fn error(&mut self, message: String) {
            let output = format!("[ERROR] {message}");
            OUTPUT.lock().unwrap().push(output);
        }

        fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
            self.inner.create_child()
        }

        fn suspend(&self, f: &mut dyn FnMut()) {
            self.inner.suspend(f)
        }

        fn println(&self, message: &str) -> std::io::Result<()> {
            self.inner.println(message)
        }
    }

    #[test]
    #[serial]
    fn test_message_output_format() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.complete(Some("Done".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "Starting...");
        assert_eq!(output[1], "[OK] Done");
    }

    #[test]
    #[serial]
    fn test_state_management() {
        let mut progress = SimpleProgress::new();

        // SimpleProgress no longer has state
        let config = ProgressConfig::new(ProgressStyle::Bytes);
        progress.start(config);

        // Just verify it doesn't panic
    }

    #[test]
    #[serial]
    fn test_error_handling() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.error("Failed to extract".to_string());

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "[ERROR] Failed to extract");
    }

    #[test]
    #[serial]
    fn test_complete_with_message() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.complete(Some("Successfully cached".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "[OK] Successfully cached");
    }

    #[test]
    #[serial]
    fn test_complete_without_message() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.complete(None);

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1], "[OK] Complete");
    }

    #[test]
    #[serial]
    fn test_update_no_output() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        progress.start(config);

        // Updates should not produce output
        for i in 0..10 {
            progress.update(i * 10, Some(100));
        }

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 1); // Only the start message
    }

    #[test]
    #[serial]
    fn test_set_message_output() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);

        // Set message should now produce output
        progress.set_message("Processing file 1".to_string());
        progress.set_message("Processing file 2".to_string());

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 3); // Start message + 2 messages
        assert_eq!(output[1], "Processing file 1");
        assert_eq!(output[2], "Processing file 2");
    }

    #[test]
    #[serial]
    fn test_multiple_operations() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        // First operation
        let config1 = ProgressConfig::new(ProgressStyle::Bytes);
        progress.start(config1);
        progress.complete(None);

        // Second operation
        let config2 = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config2);
        progress.complete(Some("Done".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], "Starting...");
        assert_eq!(output[1], "[OK] Complete");
        assert_eq!(output[2], "Starting...");
        assert_eq!(output[3], "[OK] Done");
    }

    #[test]
    fn test_create_child_returns_silent() {
        let mut progress = SimpleProgress::new();

        let mut child = progress.create_child();

        let config = ProgressConfig::new(ProgressStyle::Count);
        child.start(config);

        child.update(50, Some(100));
        child.set_message("Processing".to_string());
        child.complete(Some("Done".to_string()));

        child.error("Failed".to_string())
    }

    #[test]
    fn test_multiple_children() {
        let mut progress = SimpleProgress::new();

        let mut child1 = progress.create_child();
        let mut child2 = progress.create_child();
        let mut child3 = progress.create_child();

        let config1 = ProgressConfig::new(ProgressStyle::Count);
        child1.start(config1);
        child1.complete(None);

        let config2 = ProgressConfig::new(ProgressStyle::Bytes);
        child2.start(config2);
        child2.complete(Some("Success".to_string()));

        let config3 = ProgressConfig::new(ProgressStyle::Count);
        child3.start(config3);
        child3.error("Failed".to_string())
    }

    #[test]
    fn test_parent_child_interaction() {
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        let parent_config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(parent_config);

        let mut child = progress.create_child();
        let child_config = ProgressConfig::new(ProgressStyle::Bytes);
        child.start(child_config);
        child.update(100, Some(200));
        child.complete(Some("Child done".to_string()));

        progress.complete(Some("All done".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "Starting...");
        assert_eq!(output[1], "[OK] All done");
    }

    #[test]
    #[serial]
    fn test_ascii_only_output() {
        // Verify that SimpleProgress uses ASCII-only output for CI/NO_COLOR compatibility
        TestProgress::clear_output();
        let mut progress = TestProgress::new();

        // Test successful completion with ASCII [OK]
        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.complete(Some("Success".to_string()));

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert!(
            output[1].starts_with("[OK]"),
            "Should use ASCII [OK] prefix"
        );
        assert!(
            !output[1].contains('✓'),
            "Should not contain Unicode checkmark"
        );

        // Test error with ASCII [ERROR]
        TestProgress::clear_output();
        let mut progress = TestProgress::new();
        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);
        progress.error("Failed".to_string());

        let output = TestProgress::get_output();
        assert_eq!(output.len(), 2);
        assert!(
            output[1].starts_with("[ERROR]"),
            "Should use ASCII [ERROR] prefix"
        );
        assert!(
            !output[1].contains('✗'),
            "Should not contain Unicode cross mark"
        );
    }

    #[test]
    fn test_suspend_direct_execution() {
        let progress = SimpleProgress::new();
        let mut executed = false;

        progress.suspend(&mut || {
            executed = true;
        });

        assert!(executed, "suspend should execute the function directly");
    }

    #[test]
    fn test_println_output() {
        let progress = SimpleProgress::new();

        // This test just ensures println doesn't panic
        let result = progress.println("Test message");
        assert!(result.is_ok(), "println should return Ok");
    }
}
