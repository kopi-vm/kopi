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

        log::debug!(
            "Checking disk space for path {:?} (using {:?})",
            path,
            target_dir
        );

        #[cfg(unix)]
        {
            self.check_disk_space_unix(&target_dir)?;
        }

        #[cfg(windows)]
        {
            self.check_disk_space_windows(&target_dir)?;
        }

        Ok(())
    }

    #[cfg(unix)]
    fn check_disk_space_unix(&self, target_dir: &Path) -> Result<()> {
        use std::ffi::CString;
        use std::mem;

        let c_path = CString::new(target_dir.to_string_lossy().as_bytes())?;
        let mut stat: libc::statvfs = unsafe { mem::zeroed() };

        let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };

        if result == 0 {
            let available_mb = (stat.f_bavail * stat.f_frsize) / (1024 * 1024);
            log::debug!(
                "Disk space check: available={}MB, required={}MB",
                available_mb,
                self.min_disk_space_mb
            );

            if available_mb < self.min_disk_space_mb {
                return Err(KopiError::DiskSpaceError(format!(
                    "Insufficient disk space at {:?}. Required: {}MB, Available: {}MB",
                    target_dir, self.min_disk_space_mb, available_mb
                )));
            }
        } else {
            let errno = std::io::Error::last_os_error();
            log::error!("Failed to check disk space at {:?}: {}", target_dir, errno);
            return Err(KopiError::SystemError(format!(
                "Failed to check disk space at {:?}: {}",
                target_dir, errno
            )));
        }

        Ok(())
    }

    #[cfg(windows)]
    fn check_disk_space_windows(&self, target_dir: &Path) -> Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;
        use winapi::um::errhandlingapi::GetLastError;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;

        let path_wide: Vec<u16> = target_dir
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();

        let mut available_bytes: u64 = 0;
        let result = unsafe {
            GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut available_bytes as *mut u64,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if result != 0 {
            let available_mb = available_bytes / (1024 * 1024);
            log::debug!(
                "Disk space check: available={}MB, required={}MB",
                available_mb,
                self.min_disk_space_mb
            );

            if available_mb < self.min_disk_space_mb {
                return Err(KopiError::DiskSpaceError(format!(
                    "Insufficient disk space at {:?}. Required: {}MB, Available: {}MB",
                    target_dir, self.min_disk_space_mb, available_mb
                )));
            }
        } else {
            let error_code = unsafe { GetLastError() };
            log::error!(
                "Failed to check disk space at {:?}: Windows error code {}",
                target_dir,
                error_code
            );
            return Err(KopiError::SystemError(format!(
                "Failed to check disk space at {:?}: Windows error code {}",
                target_dir, error_code
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
