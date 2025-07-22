use crate::error::{KopiError, Result};
use std::path::Path;

pub struct DiskSpaceChecker {
    min_disk_space_mb: u64,
}

impl DiskSpaceChecker {
    pub fn new(min_disk_space_mb: u64) -> Self {
        Self { min_disk_space_mb }
    }

    pub fn check_disk_space(&self, path: &Path, kopi_home: &Path) -> Result<()> {
        let mut target_dir = path.to_path_buf();
        while !target_dir.exists() {
            if let Some(parent) = target_dir.parent() {
                target_dir = parent.to_path_buf();
            } else {
                target_dir = kopi_home.to_path_buf();
                break;
            }
        }

        log::debug!("Checking disk space for path {path:?} (using {target_dir:?})");

        // Use fs2 for platform-independent disk space checking
        let space_info = fs2::available_space(&target_dir).map_err(|e| {
            log::error!("Failed to check disk space at {target_dir:?}: {e}");
            KopiError::SystemError(format!("Failed to check disk space at {target_dir:?}: {e}"))
        })?;

        let available_mb = space_info / (1024 * 1024);
        log::debug!(
            "Disk space check: available={available_mb}MB, required={}MB",
            self.min_disk_space_mb
        );

        if available_mb < self.min_disk_space_mb {
            return Err(KopiError::DiskSpaceError(format!(
                "Insufficient disk space at {target_dir:?}. Required: {}MB, Available: \
                 {available_mb}MB",
                self.min_disk_space_mb
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_disk_space_check_path_selection() {
        let temp_dir = TempDir::new().unwrap();
        let checker = DiskSpaceChecker::new(500);

        let non_existent = temp_dir.path().join("non/existent/path");
        let result = checker.check_disk_space(&non_existent, temp_dir.path());

        assert!(result.is_ok() || matches!(result.unwrap_err(), KopiError::DiskSpaceError(_)));
    }
}
