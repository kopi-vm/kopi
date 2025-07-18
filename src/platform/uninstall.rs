//! Platform-specific uninstall operations.

use crate::error::Result;
use log::debug;
use std::path::Path;

#[cfg(target_os = "windows")]
use std::ffi::OsStr;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW};

#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_READONLY;

#[cfg(not(target_os = "windows"))]
use std::process::Command;

/// Check if any files in the given path are currently in use
pub fn check_files_in_use(path: &Path) -> Result<Vec<String>> {
    debug!("Checking if files are in use at {}", path.display());

    #[cfg(target_os = "windows")]
    {
        check_files_in_use_windows(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        check_files_in_use_unix(path)
    }
}

/// Prepare path for removal with platform-specific handling
pub fn prepare_for_removal(path: &Path) -> Result<()> {
    debug!("Preparing {} for removal", path.display());

    #[cfg(target_os = "windows")]
    {
        prepare_windows_removal(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        prepare_unix_removal(path)
    }
}

/// Clean up after removal with platform-specific handling
pub fn post_removal_cleanup(path: &Path) -> Result<()> {
    debug!("Performing post-removal cleanup for {}", path.display());

    #[cfg(target_os = "windows")]
    {
        cleanup_windows(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        cleanup_unix(path)
    }
}

#[cfg(target_os = "windows")]
fn check_files_in_use_windows(path: &Path) -> Result<Vec<String>> {
    let mut files_in_use = Vec::new();

    // Use handle.exe if available to check for open handles
    if let Ok(output) = Command::new("handle.exe")
        .arg("-u")
        .arg(path.display().to_string())
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.trim().is_empty() {
                files_in_use.push(format!("Files in use detected: {}", stdout.trim()));
            }
        }
    } else {
        // Fallback: try to rename the directory to test if it's in use
        let temp_name = format!("{}.test", path.display());
        if std::fs::rename(path, &temp_name).is_ok() {
            // Rename back immediately
            let _ = std::fs::rename(&temp_name, path);
        } else {
            files_in_use.push("Directory appears to be in use".to_string());
        }
    }

    Ok(files_in_use)
}

#[cfg(target_os = "windows")]
fn prepare_windows_removal(path: &Path) -> Result<()> {
    use walkdir::WalkDir;

    // Remove read-only attributes recursively using winapi
    for entry in WalkDir::new(path) {
        match entry {
            Ok(entry) => {
                if let Err(e) = remove_readonly_attribute(entry.path()) {
                    debug!(
                        "Failed to remove read-only attribute from {}: {}",
                        entry.path().display(),
                        e
                    );
                }
            }
            Err(e) => {
                debug!("Failed to access directory entry: {e}");
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_readonly_attribute(path: &Path) -> std::io::Result<()> {
    let path_wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        // Get current attributes
        let current_attrs = GetFileAttributesW(path_wide.as_ptr());
        if current_attrs == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }

        // Remove READ_ONLY attribute if present
        let new_attrs = current_attrs & !FILE_ATTRIBUTE_READONLY;

        // Only call SetFileAttributesW if the attributes actually changed
        if new_attrs != current_attrs && SetFileAttributesW(path_wide.as_ptr(), new_attrs) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn cleanup_windows(_path: &Path) -> Result<()> {
    // Windows-specific cleanup if needed
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn check_files_in_use_unix(path: &Path) -> Result<Vec<String>> {
    let mut files_in_use = Vec::new();

    // Use lsof to check for open files
    if let Ok(output) = Command::new("lsof")
        .arg("+D")
        .arg(path.display().to_string())
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.trim().is_empty() {
                files_in_use.push(format!("Open files detected: {}", stdout.trim()));
            }
        }
    }

    Ok(files_in_use)
}

#[cfg(not(target_os = "windows"))]
fn prepare_unix_removal(path: &Path) -> Result<()> {
    // Make all files writable
    if let Err(e) = Command::new("chmod")
        .arg("-R")
        .arg("u+w")
        .arg(path.display().to_string())
        .output()
    {
        debug!("Failed to make files writable: {e}");
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn cleanup_unix(path: &Path) -> Result<()> {
    // Clean up any remaining symbolic links that might point to the removed JDK
    if let Some(parent) = path.parent() {
        super::symlink::cleanup_orphaned_symlinks(parent)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_files_in_use_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let files_in_use = check_files_in_use(temp_dir.path()).unwrap();
        assert!(files_in_use.is_empty());
    }

    #[test]
    fn test_prepare_for_removal() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = prepare_for_removal(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_post_removal_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let result = post_removal_cleanup(temp_dir.path());
        assert!(result.is_ok());
    }
}
