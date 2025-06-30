use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

pub const MAX_DOWNLOAD_SIZE: u64 = 1_073_741_824;

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub checksum: Option<String>,

    pub resume: bool,

    pub timeout: Duration,

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

pub struct DownloadResult {
    pub path: PathBuf,

    pub(crate) _temp_dir: tempfile::TempDir,
}

impl DownloadResult {
    pub fn path(&self) -> &Path {
        &self.path
    }

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
