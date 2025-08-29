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

pub struct SilentProgress;

impl SilentProgress {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SilentProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressIndicator for SilentProgress {
    fn start(&mut self, _config: ProgressConfig) {
        // No output
    }

    fn update(&mut self, _current: u64, _total: Option<u64>) {
        // No output
    }

    fn set_message(&mut self, _message: String) {
        // No output
    }

    fn complete(&mut self, _message: Option<String>) {
        // No output
    }

    fn error(&mut self, _message: String) {
        // No output - errors are handled separately by the error system
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        Box::new(SilentProgress::new())
    }

    fn suspend(&self, f: &mut dyn FnMut()) {
        // SilentProgress doesn't need to suspend anything
        f();
    }

    fn println(&self, _message: &str) -> std::io::Result<()> {
        // SilentProgress doesn't output anything
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::ProgressStyle;

    #[test]
    fn test_no_panic_on_calls() {
        let mut progress = SilentProgress::new();

        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);

        // None of these should panic
        progress.start(config);
        progress.update(50, None);
        progress.update(75, Some(200));
        progress.set_message("Processing".to_string());
        progress.complete(Some("Done".to_string()));
        progress.error("Error occurred".to_string());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let progress = Arc::new(Mutex::new(SilentProgress::new()));
        let mut handles = vec![];

        for i in 0..10 {
            let progress_clone = progress.clone();
            let handle = thread::spawn(move || {
                let mut p = progress_clone.lock().unwrap();
                p.update(i * 10, Some(100));
                p.set_message(format!("Thread {i}"));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_memory_usage() {
        use std::mem;

        let progress = SilentProgress::new();

        // SilentProgress should be zero-sized
        assert_eq!(mem::size_of_val(&progress), 0);

        // Verify it can be used as trait object
        let _boxed: Box<dyn ProgressIndicator> = Box::new(progress);
    }

    #[test]
    fn test_default_implementation() {
        let progress1 = SilentProgress::new();
        let progress2 = SilentProgress;

        // Both should be equivalent (zero-sized types)
        assert_eq!(
            std::mem::size_of_val(&progress1),
            std::mem::size_of_val(&progress2)
        );
    }

    #[test]
    fn test_multiple_operations() {
        let mut progress = SilentProgress::new();

        // Simulate multiple operations
        for _i in 0..3 {
            let config = ProgressConfig::new(ProgressStyle::Bytes);
            progress.start(config);

            for j in 0..10 {
                progress.update(j * 10, Some(100));
            }

            progress.complete(None);
        }

        // Should complete without issues
    }
}
