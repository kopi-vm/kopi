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

use crate::indicator::{ProgressFactory, ProgressIndicator, ProgressRendererKind};
use std::sync::{Arc, Mutex};

/// Provides high-level status messaging that reuses the shared progress indicator stack.
pub struct StatusReporter {
    progress: Arc<Mutex<Box<dyn ProgressIndicator>>>,
    renderer_kind: ProgressRendererKind,
}

impl StatusReporter {
    /// Creates a status reporter using the shared progress factory.
    pub fn new(no_progress: bool) -> Self {
        Self::with_indicator(ProgressFactory::create(no_progress))
    }

    /// Creates a status reporter backed by the provided indicator (primarily for tests).
    pub fn with_indicator(indicator: Box<dyn ProgressIndicator>) -> Self {
        let renderer_kind = indicator.renderer_kind();
        Self {
            progress: Arc::new(Mutex::new(indicator)),
            renderer_kind,
        }
    }

    /// Exposes the underlying progress handle for lock feedback integration.
    pub fn progress_handle(&self) -> Arc<Mutex<Box<dyn ProgressIndicator>>> {
        Arc::clone(&self.progress)
    }

    fn is_silent(&self) -> bool {
        matches!(self.renderer_kind, ProgressRendererKind::Silent)
    }

    fn with_progress<R>(&self, f: impl FnOnce(&dyn ProgressIndicator) -> R) -> Option<R> {
        if self.is_silent() {
            return None;
        }

        let guard = self.progress.lock().ok()?;
        Some(f(guard.as_ref()))
    }

    fn with_progress_mut<R>(&self, f: impl FnOnce(&mut dyn ProgressIndicator) -> R) -> Option<R> {
        if self.is_silent() {
            return None;
        }

        let mut guard = self.progress.lock().ok()?;
        Some(f(guard.as_mut()))
    }

    pub fn operation(&self, operation: &str, context: &str) {
        let message = format!("{operation} {context}...");
        let _ = self.with_progress(|indicator| indicator.println(&message));
    }

    pub fn step(&self, message: &str) {
        let message = format!("  {message}");
        let _ = self.with_progress(|indicator| indicator.println(&message));
    }

    pub fn success(&self, message: &str) {
        let _ = self.with_progress(|indicator| indicator.success(message));
    }

    pub fn error(&self, message: &str) {
        let _ = self.with_progress_mut(|indicator| indicator.error(message.to_string()));
    }

    /// Emits the initial lock-wait message using the shared indicator infrastructure.
    pub fn lock_feedback_start(&self, message: &str) {
        let _ = self.with_progress(|indicator| indicator.println(message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::{ProgressConfig, SilentProgress};
    use std::sync::Mutex;

    static OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ERROR_OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct RecordingIndicator;

    impl RecordingIndicator {
        fn new() -> Self {
            RecordingIndicator
        }

        fn push_output(message: String) {
            OUTPUT.lock().unwrap().push(message);
        }

        fn push_error(message: String) {
            ERROR_OUTPUT.lock().unwrap().push(message);
        }

        fn take_messages() -> (Vec<String>, Vec<String>) {
            let output = OUTPUT.lock().unwrap().drain(..).collect();
            let errors = ERROR_OUTPUT.lock().unwrap().drain(..).collect();
            (output, errors)
        }
    }

    impl ProgressIndicator for RecordingIndicator {
        fn start(&mut self, _config: ProgressConfig) {}

        fn update(&mut self, _current: u64, _total: Option<u64>) {}

        fn set_message(&mut self, message: String) {
            Self::push_output(format!("set:{message}"));
        }

        fn complete(&mut self, message: Option<String>) {
            if let Some(msg) = message {
                Self::push_output(format!("complete:{msg}"));
            }
        }

        fn success(&self, message: &str) -> std::io::Result<()> {
            Self::push_output(format!("success:{message}"));
            Ok(())
        }

        fn error(&mut self, message: String) {
            Self::push_error(format!("error:{message}"));
        }

        fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
            Box::new(RecordingIndicator::new())
        }

        fn suspend(&self, f: &mut dyn FnMut()) {
            f();
        }

        fn println(&self, message: &str) -> std::io::Result<()> {
            Self::push_output(message.to_string());
            Ok(())
        }

        fn renderer_kind(&self) -> ProgressRendererKind {
            ProgressRendererKind::NonTty
        }
    }

    #[test]
    fn operation_and_step_emit_messages() {
        let reporter = StatusReporter::with_indicator(Box::new(RecordingIndicator::new()));

        reporter.operation("Installing", "temurin@21");
        reporter.step("Downloading archive");

        let (output, errors) = RecordingIndicator::take_messages();
        assert!(errors.is_empty());
        assert_eq!(
            output,
            vec![
                "Installing temurin@21...".to_string(),
                "  Downloading archive".to_string()
            ]
        );
    }

    #[test]
    fn success_and_error_route_through_indicator() {
        let reporter = StatusReporter::with_indicator(Box::new(RecordingIndicator::new()));

        reporter.success("Installation complete");
        reporter.error("Failed to extract archive");

        let (output, errors) = RecordingIndicator::take_messages();
        assert_eq!(output, vec!["success:Installation complete".to_string()]);
        assert_eq!(errors, vec!["error:Failed to extract archive".to_string()]);
    }

    #[test]
    fn progress_handle_exposes_shared_indicator() {
        let reporter = StatusReporter::with_indicator(Box::new(RecordingIndicator::new()));
        let handle = reporter.progress_handle();

        {
            let indicator = handle.lock().unwrap();
            let _ = indicator.println("from-handle");
        }

        let (output, errors) = RecordingIndicator::take_messages();
        assert!(errors.is_empty());
        assert_eq!(output, vec!["from-handle".to_string()]);
    }

    #[test]
    fn lock_feedback_start_uses_shared_output() {
        let reporter = StatusReporter::with_indicator(Box::new(RecordingIndicator::new()));
        reporter.lock_feedback_start("Waiting for lock on cache (timeout: 30s)");

        let (output, errors) = RecordingIndicator::take_messages();
        assert!(errors.is_empty());
        assert_eq!(
            output,
            vec!["Waiting for lock on cache (timeout: 30s)".to_string()]
        );
    }

    #[test]
    fn silent_indicator_suppresses_output() {
        let reporter = StatusReporter::with_indicator(Box::new(SilentProgress::new()));

        reporter.operation("Installing", "temurin@21");
        reporter.step("Downloading archive");
        reporter.success("Installation complete");
        reporter.error("Failed to extract archive");

        let (output, errors) = RecordingIndicator::take_messages();
        assert!(output.is_empty());
        assert!(errors.is_empty());
    }
}
