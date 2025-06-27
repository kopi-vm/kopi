use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default timeout for download operations
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum allowed download size (1GB)
pub const MAX_DOWNLOAD_SIZE: u64 = 1_073_741_824;

/// Options for configuring download behavior
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Expected checksum of the downloaded file (SHA256)
    pub checksum: Option<String>,

    /// Whether to resume interrupted downloads
    pub resume: bool,

    /// Timeout for the download operation
    pub timeout: Duration,

    /// Maximum allowed file size
    pub max_size: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            checksum: None,
            resume: true,
            timeout: DEFAULT_TIMEOUT,
            max_size: MAX_DOWNLOAD_SIZE,
        }
    }
}

/// Result of a JDK download operation
pub struct DownloadResult {
    /// Path to the downloaded file
    pub path: PathBuf,

    /// Temporary directory containing the file (will be cleaned up when dropped)
    pub(crate) _temp_dir: tempfile::TempDir,
}

impl DownloadResult {
    /// Get the path to the downloaded file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a new download result
    pub(crate) fn new(path: PathBuf, temp_dir: tempfile::TempDir) -> Self {
        Self {
            path,
            _temp_dir: temp_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_options_default() {
        let options = DownloadOptions::default();
        assert_eq!(options.checksum, None);
        assert!(options.resume);
        assert_eq!(options.timeout, DEFAULT_TIMEOUT);
        assert_eq!(options.max_size, MAX_DOWNLOAD_SIZE);
    }
}
