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

use kopi::indicator::{ProgressConfig, ProgressIndicator, ProgressRendererKind, ProgressStyle};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ProgressMessage {
    pub message: String,
    #[allow(dead_code)]
    pub style: Option<ProgressStyle>,
}

/// Test helper that captures progress indicator updates for verification
#[derive(Clone)]
pub struct TestProgressCapture {
    messages: Arc<Mutex<Vec<ProgressMessage>>>,
    current_style: Option<ProgressStyle>,
    total: Option<u64>,
    position: u64,
}

impl TestProgressCapture {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            current_style: None,
            total: None,
            position: 0,
        }
    }

    #[allow(dead_code)]
    pub fn get_messages(&self) -> Vec<ProgressMessage> {
        self.messages.lock().unwrap().clone()
    }

    pub fn get_last_message(&self) -> Option<String> {
        self.messages
            .lock()
            .unwrap()
            .last()
            .map(|m| m.message.clone())
    }

    pub fn message_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    pub fn clear(&mut self) {
        self.messages.lock().unwrap().clear();
        self.position = 0;
        self.total = None;
        self.current_style = None;
    }

    pub fn get_position(&self) -> u64 {
        self.position
    }

    pub fn get_total(&self) -> Option<u64> {
        self.total
    }

    pub fn contains_message(&self, text: &str) -> bool {
        self.messages
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.message.contains(text))
    }

    pub fn set_position(&mut self, pos: u64) {
        self.position = pos;
    }

    pub fn with_total(&mut self, total: u64) -> &mut Self {
        self.total = Some(total);
        self
    }

    #[allow(dead_code)]
    pub fn finish_with_message(&mut self, message: &str) {
        self.set_message(message.to_string());
    }
}

impl Default for TestProgressCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressIndicator for TestProgressCapture {
    fn start(&mut self, config: ProgressConfig) {
        self.current_style = Some(config.style);
        self.total = config.total;
        self.position = 0;
    }

    fn update(&mut self, current: u64, total: Option<u64>) {
        self.position = current;
        if total.is_some() {
            self.total = total;
        }
    }

    fn set_message(&mut self, message: String) {
        self.messages.lock().unwrap().push(ProgressMessage {
            message,
            style: self.current_style,
        });
    }

    fn complete(&mut self, message: Option<String>) {
        if let Some(msg) = message {
            self.set_message(msg);
        }
    }

    fn success(&self, message: &str) -> std::io::Result<()> {
        self.messages.lock().unwrap().push(ProgressMessage {
            message: format!("✓ {message}"),
            style: self.current_style,
        });
        Ok(())
    }

    fn error(&mut self, message: String) {
        self.set_message(format!("[ERROR] {message}"));
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        Box::new(TestProgressCapture::new())
    }

    fn suspend(&self, f: &mut dyn FnMut()) {
        // TestProgressCapture doesn't need to suspend anything
        f();
    }

    fn println(&self, message: &str) -> std::io::Result<()> {
        // Capture println messages as regular messages
        self.messages.lock().unwrap().push(ProgressMessage {
            message: message.to_string(),
            style: self.current_style,
        });
        Ok(())
    }

    fn renderer_kind(&self) -> ProgressRendererKind {
        ProgressRendererKind::NonTty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_capture() {
        let mut capture = TestProgressCapture::new();

        capture.set_message("Starting".to_string());
        capture.set_message("Processing".to_string());
        capture.set_message("Completed".to_string());

        assert_eq!(capture.message_count(), 3);
        assert_eq!(capture.get_last_message(), Some("Completed".to_string()));
        assert!(capture.contains_message("Processing"));
    }

    #[test]
    fn test_progress_with_steps() {
        let mut capture = TestProgressCapture::new();

        capture.with_total(10);
        capture.set_position(1);
        capture.set_message("Step 1".to_string());

        capture.set_position(2); // Changed from increment
        capture.set_message("Step 2".to_string());

        assert_eq!(capture.get_position(), 2);
        assert_eq!(capture.get_total(), Some(10));
        assert_eq!(capture.message_count(), 2);
    }

    #[test]
    fn test_error_message() {
        let mut capture = TestProgressCapture::new();

        capture.error("Something went wrong".to_string());

        assert_eq!(
            capture.get_last_message(),
            Some("[ERROR] Something went wrong".to_string())
        );
    }

    #[test]
    fn test_clear() {
        let mut capture = TestProgressCapture::new();

        capture.set_message("Test".to_string());
        capture.with_total(5);
        capture.set_position(3);

        assert_eq!(capture.message_count(), 1);

        capture.clear();

        assert_eq!(capture.message_count(), 0);
        assert_eq!(capture.get_position(), 0);
        assert_eq!(capture.get_total(), None);
    }
}
