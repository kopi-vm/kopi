//! Platform-specific uninstall operations.

use crate::error::Result;
use log::debug;
use std::path::Path;

#[cfg(target_os = "windows")]
use crate::error::KopiError;
#[cfg(target_os = "windows")]
use log::warn;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::time::Duration;

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

/// Handle antivirus interference on Windows
#[cfg(target_os = "windows")]
fn handle_antivirus_interference(path: &Path) -> Result<()> {
    debug!("Checking for antivirus interference at {}", path.display());

    // Wait a bit for antivirus to release files
    std::thread::sleep(Duration::from_millis(500));

    // Check if path still exists after delay
    if path.exists() {
        warn!("Files may be held by antivirus software");
        // Try a few more times with increasing delays
        for attempt in 1..=3 {
            std::thread::sleep(Duration::from_millis(attempt * 1000));
            if !path.exists() {
                debug!("Antivirus released files after {} attempts", attempt);
                return Ok(());
            }
        }

        return Err(KopiError::SystemError(
            "Files may be held by antivirus software. Try temporarily disabling real-time protection.".to_string()
        ));
    }

    Ok(())
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
    // Remove read-only attributes recursively
    if let Err(e) = Command::new("attrib")
        .arg("-R")
        .arg("/S")
        .arg(path.display().to_string())
        .output()
    {
        debug!("Failed to remove read-only attributes: {}", e);
    }

    // Handle potential antivirus interference
    handle_antivirus_interference(path)?;

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
