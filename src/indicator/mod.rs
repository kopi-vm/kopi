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

pub mod factory;
pub mod indicatif;
pub mod silent;
pub mod simple;
pub mod status;
pub mod types;

pub use factory::ProgressFactory;
pub use indicatif::IndicatifProgress;
pub use silent::SilentProgress;
pub use simple::SimpleProgress;
pub use status::StatusReporter;
pub use types::{ProgressConfig, ProgressStyle};

pub trait ProgressIndicator: Send + Sync {
    fn start(&mut self, config: ProgressConfig);
    fn update(&mut self, current: u64, total: Option<u64>);
    fn set_message(&mut self, message: String);
    fn complete(&mut self, message: Option<String>);
    fn success(&self, message: &str) -> std::io::Result<()>;
    fn error(&mut self, message: String);
    fn create_child(&mut self) -> Box<dyn ProgressIndicator>;
    fn suspend(&self, f: &mut dyn FnMut());
    fn println(&self, message: &str) -> std::io::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProgress {
        started: bool,
        current: u64,
        total: Option<u64>,
        message: String,
        completed: bool,
        errored: bool,
    }

    impl MockProgress {
        fn new() -> Self {
            Self {
                started: false,
                current: 0,
                total: None,
                message: String::new(),
                completed: false,
                errored: false,
            }
        }
    }

    impl ProgressIndicator for MockProgress {
        fn start(&mut self, config: ProgressConfig) {
            self.started = true;
            self.total = config.total;
        }

        fn update(&mut self, current: u64, total: Option<u64>) {
            self.current = current;
            if total.is_some() {
                self.total = total;
            }
        }

        fn set_message(&mut self, message: String) {
            self.message = message;
        }

        fn complete(&mut self, message: Option<String>) {
            self.completed = true;
            if let Some(msg) = message {
                self.message = msg;
            }
        }

        fn success(&self, message: &str) -> std::io::Result<()> {
            println!("✓ {message}");
            Ok(())
        }

        fn error(&mut self, message: String) {
            self.errored = true;
            self.message = message;
        }

        fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
            Box::new(MockProgress::new())
        }

        fn suspend(&self, f: &mut dyn FnMut()) {
            f();
        }

        fn println(&self, message: &str) -> std::io::Result<()> {
            println!("{message}");
            Ok(())
        }
    }

    #[test]
    fn test_trait_implementation() {
        let mut progress = MockProgress::new();

        // Test start
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        progress.start(config);
        assert!(progress.started);
        assert_eq!(progress.total, Some(100));

        // Test update
        progress.update(50, None);
        assert_eq!(progress.current, 50);
        assert_eq!(progress.total, Some(100));

        // Test update with new total
        progress.update(60, Some(200));
        assert_eq!(progress.current, 60);
        assert_eq!(progress.total, Some(200));

        // Test set_message
        progress.set_message("Processing item".to_string());
        assert_eq!(progress.message, "Processing item");

        // Test complete
        progress.complete(Some("Done!".to_string()));
        assert!(progress.completed);
        assert_eq!(progress.message, "Done!");
    }

    #[test]
    fn test_error_handling() {
        let mut progress = MockProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count);
        progress.start(config);

        progress.error("Something went wrong".to_string());
        assert!(progress.errored);
        assert_eq!(progress.message, "Something went wrong");
    }

    #[test]
    fn test_indeterminate_progress() {
        let mut progress = MockProgress::new();

        // Start without total (indeterminate)
        let config = ProgressConfig::new(ProgressStyle::Bytes);
        progress.start(config);
        assert!(progress.started);
        assert_eq!(progress.total, None);

        // Update without total should work
        progress.update(1000, None);
        assert_eq!(progress.current, 1000);
        assert_eq!(progress.total, None);
    }

    #[test]
    fn test_trait_object() {
        // Verify the trait can be used as a trait object
        let progress: Box<dyn ProgressIndicator> = Box::new(MockProgress::new());

        // This should compile, proving Send + Sync bounds work
        fn accept_progress(_p: Box<dyn ProgressIndicator>) {}
        accept_progress(progress);
    }
}
