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

use kopi::config::KopiConfig;
use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};

/// Creates a test home directory under target/home with a random 8-character name
/// Returns the path to the created directory
/// The directory will be automatically cleaned up when the returned guard is dropped
pub struct TestHomeGuard {
    path: PathBuf,
}

impl TestHomeGuard {
    pub fn new() -> Self {
        // Generate random 8-character string with letters and numbers
        let random_name: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        // Create directory under target/home with absolute path
        let relative_path = PathBuf::from("target/home").join(random_name);
        let path = std::env::current_dir()
            .expect("Failed to get current directory")
            .join(relative_path);
        fs::create_dir_all(&path).expect("Failed to create test home directory");

        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kopi_home(&self) -> PathBuf {
        self.path.join(".kopi")
    }

    pub fn setup_kopi_structure(&self) -> &Self {
        let kopi_home = self.kopi_home();
        fs::create_dir_all(&kopi_home).expect("Failed to create .kopi directory");

        // Use KopiConfig to get directory paths (directories are created automatically)
        let config = KopiConfig::new(kopi_home.clone()).expect("Failed to create KopiConfig");
        config.jdks_dir().expect("Failed to create jdks directory");
        config
            .cache_dir()
            .expect("Failed to create cache directory");
        config.bin_dir().expect("Failed to create bin directory");

        self
    }
}

impl Drop for TestHomeGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            // On Windows, sometimes file handles are still open when we try to delete
            // Retry a few times with a small delay
            let mut attempts = 0;
            const MAX_ATTEMPTS: u32 = 3;

            while attempts < MAX_ATTEMPTS && self.path.exists() {
                match fs::remove_dir_all(&self.path) {
                    Ok(_) => break,
                    Err(e) => {
                        attempts += 1;
                        if attempts < MAX_ATTEMPTS {
                            eprintln!(
                                "Attempt {}/{} to cleanup test directory {} failed: {}. Retrying...",
                                attempts,
                                MAX_ATTEMPTS,
                                self.path.display(),
                                e
                            );
                            // Small delay before retry (especially helpful on Windows)
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        } else {
                            eprintln!(
                                "Failed to cleanup test directory {} after {} attempts: {}",
                                self.path.display(),
                                MAX_ATTEMPTS,
                                e
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creates_and_cleans_up_directory() {
        let test_path = {
            let guard = TestHomeGuard::new();
            let path = guard.path().to_path_buf();
            assert!(path.exists());
            assert!(path.ends_with(path.file_name().unwrap())); // Should end with random name
            assert!(path.to_string_lossy().contains("target/home")); // Should contain target/home in path
            path
        };
        // After guard is dropped, directory should be cleaned up
        assert!(!test_path.exists());
    }

    #[test]
    fn test_setup_kopi_structure() {
        let guard = TestHomeGuard::new();
        let guard = guard.setup_kopi_structure();
        let kopi_home = guard.kopi_home();

        assert!(kopi_home.exists());
        assert!(kopi_home.join("jdks").exists());
        assert!(kopi_home.join("cache").exists());
        assert!(kopi_home.join("bin").exists());
    }
}
