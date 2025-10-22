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

use colored::Colorize;
use std::env;
use std::io::IsTerminal;

pub struct StatusReporter {
    silent: bool,
    use_color: bool,
}

impl StatusReporter {
    pub fn new(silent: bool) -> Self {
        Self {
            silent,
            use_color: Self::should_use_color(),
        }
    }

    fn should_use_color() -> bool {
        // Respect NO_COLOR environment variable
        if env::var("NO_COLOR").is_ok() {
            return false;
        }

        // Disable colors for dumb terminals
        if let Ok(term) = env::var("TERM")
            && term == "dumb"
        {
            return false;
        }

        // Check if stderr supports colors
        std::io::stderr().is_terminal()
    }

    pub fn operation(&self, operation: &str, context: &str) {
        if !self.silent {
            println!("{operation} {context}...");
        }
    }

    pub fn step(&self, message: &str) {
        if !self.silent {
            println!("  {message}");
        }
    }

    pub fn success(&self, message: &str) {
        if !self.silent {
            let symbol = if self.use_color {
                "✓".green().bold().to_string()
            } else {
                "[OK]".to_string()
            };
            println!("{symbol} {message}");
        }
    }

    pub fn error(&self, message: &str) {
        let symbol = if self.use_color {
            "✗".red().bold().to_string()
        } else {
            "[ERROR]".to_string()
        };
        eprintln!("{symbol} {message}");
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Mutex;

    // Helper to temporarily set environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        vars: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self { vars: Vec::new() }
        }

        fn set(&mut self, key: &str, value: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            unsafe {
                env::set_var(key, value);
            }
        }

        fn remove(&mut self, key: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            unsafe {
                env::remove_var(key);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.vars.iter().rev() {
                match value {
                    Some(v) => unsafe { env::set_var(key, v) },
                    None => unsafe { env::remove_var(key) },
                }
            }
        }
    }

    // Helper to capture stdout/stderr for testing
    static OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ERROR_OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub struct TestReporter {
        inner: StatusReporter,
    }

    impl TestReporter {
        pub fn new(silent: bool) -> Self {
            Self {
                inner: StatusReporter::new(silent),
            }
        }

        pub fn operation(&self, operation: &str, context: &str) {
            if !self.inner.silent {
                OUTPUT
                    .lock()
                    .unwrap()
                    .push(format!("{operation} {context}..."));
            }
        }

        pub fn step(&self, message: &str) {
            if !self.inner.silent {
                OUTPUT.lock().unwrap().push(format!("  {message}"));
            }
        }

        pub fn success(&self, message: &str) {
            if !self.inner.silent {
                let symbol = if self.inner.use_color {
                    "✓".green().bold().to_string()
                } else {
                    "[OK]".to_string()
                };
                OUTPUT.lock().unwrap().push(format!("{symbol} {message}"));
            }
        }

        pub fn error(&self, message: &str) {
            let symbol = if self.inner.use_color {
                "✗".red().bold().to_string()
            } else {
                "[ERROR]".to_string()
            };
            ERROR_OUTPUT
                .lock()
                .unwrap()
                .push(format!("{symbol} {message}"));
        }

        pub fn get_output() -> Vec<String> {
            OUTPUT.lock().unwrap().clone()
        }

        pub fn get_error_output() -> Vec<String> {
            ERROR_OUTPUT.lock().unwrap().clone()
        }

        pub fn clear_output() {
            OUTPUT.lock().unwrap().clear();
            ERROR_OUTPUT.lock().unwrap().clear();
        }
    }

    #[serial]
    #[test]
    fn test_message_formatting() {
        TestReporter::clear_output();
        let reporter = TestReporter::new(false);

        reporter.operation("Installing", "temurin@21");
        reporter.step("Downloading JDK");
        reporter.success("Installation complete");

        let output = TestReporter::get_output();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], "Installing temurin@21...");
        assert_eq!(output[1], "  Downloading JDK");
        assert!(output[2].contains("Installation complete"));
    }

    #[serial]
    #[test]
    fn test_silent_mode() {
        TestReporter::clear_output();
        let reporter = TestReporter::new(true);

        reporter.operation("Installing", "JDK");
        reporter.step("Step 1");
        reporter.success("Done");

        let output = TestReporter::get_output();
        assert_eq!(output.len(), 0);

        // Error messages should still appear
        reporter.error("Something went wrong");
        let error_output = TestReporter::get_error_output();
        assert_eq!(error_output.len(), 1);
        assert!(error_output[0].contains("Something went wrong"));
    }

    #[serial]
    #[test]
    fn test_error_always_shown() {
        // Test silent mode error reporting
        TestReporter::clear_output();
        let silent_reporter = TestReporter::new(true);
        silent_reporter.error("Error in silent mode");
        let error_output = TestReporter::get_error_output();
        assert!(
            error_output
                .iter()
                .any(|s| s.contains("Error in silent mode")),
            "Silent mode should still show errors"
        );

        // Test normal mode error reporting
        TestReporter::clear_output();
        let normal_reporter = TestReporter::new(false);
        normal_reporter.error("Error in normal mode");
        let error_output = TestReporter::get_error_output();
        assert!(
            error_output
                .iter()
                .any(|s| s.contains("Error in normal mode")),
            "Normal mode should show errors"
        );
    }

    #[test]
    fn test_should_use_color_with_no_color_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("NO_COLOR", "1");

        assert!(!StatusReporter::should_use_color());
    }

    #[test]
    fn test_should_use_color_with_dumb_term() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("TERM", "dumb");
        env_guard.remove("NO_COLOR");

        assert!(!StatusReporter::should_use_color());
    }

    #[test]
    fn test_color_symbols() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();

        // Remove NO_COLOR to potentially enable colors
        env_guard.remove("NO_COLOR");
        env_guard.set("TERM", "xterm-256color");

        TestReporter::clear_output();

        // Create reporter that would use colors if terminal is available
        let test_reporter = TestReporter {
            inner: StatusReporter {
                silent: false,
                use_color: true, // Force color mode for testing
            },
        };

        test_reporter.success("With color");
        let output = TestReporter::get_output();
        assert!(output[0].contains("✓"));

        // Test without color
        TestReporter::clear_output();
        let test_reporter = TestReporter {
            inner: StatusReporter {
                silent: false,
                use_color: false, // Force no-color mode for testing
            },
        };

        test_reporter.success("Without color");
        let output = TestReporter::get_output();
        assert!(output[0].starts_with("[OK]"));
    }

    #[test]
    fn test_error_symbols() {
        TestReporter::clear_output();

        // Test with color
        let test_reporter = TestReporter {
            inner: StatusReporter {
                silent: false,
                use_color: true,
            },
        };

        test_reporter.error("Color error");
        let output = TestReporter::get_error_output();
        assert!(output[0].contains("✗"));

        // Test without color
        TestReporter::clear_output();
        let test_reporter = TestReporter {
            inner: StatusReporter {
                silent: false,
                use_color: false,
            },
        };

        test_reporter.error("No color error");
        let output = TestReporter::get_error_output();
        assert!(output[0].starts_with("[ERROR]"));
    }

    #[test]
    fn test_environment_detection() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();

        // Clean environment should use terminal detection
        env_guard.remove("NO_COLOR");
        env_guard.remove("CI");
        env_guard.set("TERM", "xterm-256color");

        // This will return based on actual terminal detection
        // During tests, stderr is typically not a terminal
        let _result = StatusReporter::should_use_color();

        // Just verify it doesn't panic - the result depends on test environment
    }

    #[serial]
    #[test]
    fn test_message_consistency() {
        TestReporter::clear_output();
        let reporter = TestReporter::new(false);

        // Test a complete workflow
        reporter.operation("Processing", "batch job");
        reporter.step("Step 1: Initialize");
        reporter.step("Step 2: Process data");
        reporter.step("Step 3: Cleanup");
        reporter.success("Batch job completed successfully");

        let output = TestReporter::get_output();
        assert_eq!(output.len(), 5);
        assert_eq!(output[0], "Processing batch job...");
        assert_eq!(output[1], "  Step 1: Initialize");
        assert_eq!(output[2], "  Step 2: Process data");
        assert_eq!(output[3], "  Step 3: Cleanup");
        assert!(output[4].contains("Batch job completed successfully"));
    }
}
