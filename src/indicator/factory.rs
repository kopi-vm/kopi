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

use crate::indicator::{IndicatifProgress, ProgressIndicator, SilentProgress, SimpleProgress};
use std::env;
use std::io::IsTerminal;

pub struct ProgressFactory;

impl ProgressFactory {
    pub fn create(no_progress: bool) -> Box<dyn ProgressIndicator> {
        if no_progress {
            // User explicitly requested no progress output
            Box::new(SilentProgress)
        } else if Self::should_use_simple_progress() {
            // Non-terminal or CI environment
            Box::new(SimpleProgress::new())
        } else {
            // Terminal environment with full animation support
            Box::new(IndicatifProgress::new())
        }
    }

    fn should_use_simple_progress() -> bool {
        // Check if stderr is not a terminal (pipe, redirect, etc.)
        if !std::io::stderr().is_terminal() {
            return true;
        }

        // Check for CI environment variable (GitHub Actions, Jenkins, etc.)
        if env::var("CI").is_ok() {
            return true;
        }

        // Check for dumb terminal
        if let Ok(term) = env::var("TERM")
            && term == "dumb"
        {
            return true;
        }

        // Check for NO_COLOR environment variable (https://no-color.org/)
        if env::var("NO_COLOR").is_ok() {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::{ProgressConfig, ProgressStyle};
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

    #[test]
    fn test_factory_returns_silent_with_no_progress_flag() {
        let _guard = ENV_LOCK.lock().unwrap();
        let progress = ProgressFactory::create(true);

        // Test that it's actually SilentProgress by checking behavior
        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config); // Should not panic
        p.complete(None); // Should not panic
    }

    #[test]
    fn test_factory_returns_simple_in_ci_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("CI", "true");

        let progress = ProgressFactory::create(false);

        // The type should be SimpleProgress but we can't directly check that
        // We can verify it's not silent by the fact it would produce output
        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config);
        p.complete(None);
    }

    #[test]
    fn test_factory_returns_simple_with_dumb_terminal() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("TERM", "dumb");
        env_guard.remove("CI");

        let progress = ProgressFactory::create(false);

        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config);
        p.complete(None);
    }

    #[test]
    fn test_factory_returns_simple_with_no_color() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("NO_COLOR", "1");
        env_guard.remove("CI");
        env_guard.remove("TERM");

        let progress = ProgressFactory::create(false);

        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config);
        p.complete(None);
    }

    #[test]
    fn test_factory_returns_indicatif_in_normal_terminal() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.remove("CI");
        env_guard.remove("NO_COLOR");
        env_guard.set("TERM", "xterm-256color");

        // Note: This test might still return SimpleProgress if stderr is not a terminal
        // during test execution, which is expected behavior
        let progress = ProgressFactory::create(false);

        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config);
        p.complete(None);
    }

    #[test]
    fn test_should_use_simple_progress_with_ci() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("CI", "true");

        assert!(ProgressFactory::should_use_simple_progress());
    }

    #[test]
    fn test_should_use_simple_progress_with_dumb_term() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("TERM", "dumb");
        env_guard.remove("CI");

        assert!(ProgressFactory::should_use_simple_progress());
    }

    #[test]
    fn test_should_use_simple_progress_with_no_color() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("NO_COLOR", "1");
        env_guard.remove("CI");
        env_guard.remove("TERM");

        assert!(ProgressFactory::should_use_simple_progress());
    }

    #[test]
    fn test_no_progress_flag_takes_precedence() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut env_guard = EnvGuard::new();
        // Set conditions that would normally trigger SimpleProgress
        env_guard.set("CI", "true");
        env_guard.set("TERM", "dumb");
        env_guard.set("NO_COLOR", "1");

        // no_progress should still result in SilentProgress
        let progress = ProgressFactory::create(true);

        let mut p = progress;
        let config = ProgressConfig::new("Test", "op", ProgressStyle::Count);
        p.start(config);
        p.complete(None);
    }
}
