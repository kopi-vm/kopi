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

        // Create directory under target/home
        let path = PathBuf::from("target/home").join(random_name);
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
        fs::create_dir_all(kopi_home.join("jdks")).expect("Failed to create jdks directory");
        fs::create_dir_all(kopi_home.join("cache")).expect("Failed to create cache directory");
        fs::create_dir_all(kopi_home.join("bin")).expect("Failed to create bin directory");
        self
    }
}

impl Drop for TestHomeGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to cleanup test directory {}: {}",
                    self.path.display(),
                    e
                );
            });
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
            assert!(path.starts_with("target/home"));
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
