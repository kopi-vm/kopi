//! Platform-specific symlink operations.

use crate::error::Result;
use log::{debug, warn};
use std::fs;
use std::path::Path;

/// Create a symlink (Unix)
#[cfg(unix)]
pub fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    use std::os::unix::fs as unix_fs;
    unix_fs::symlink(target, link)
}

/// Create a symlink (Windows - copies the file instead)
#[cfg(windows)]
pub fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    // Copy the file and verify the copy succeeded
    let bytes_copied = fs::copy(target, link)?;

    // Verify the file sizes match
    let source_size = fs::metadata(target)?.len();
    if bytes_copied != source_size {
        return Err(std::io::Error::other(format!(
            "Copy size mismatch: expected {source_size} bytes, copied {bytes_copied} bytes"
        )));
    }

    Ok(())
}

/// Verify a symlink points to the expected target
#[cfg(unix)]
pub fn verify_symlink(link: &Path, expected_target: &Path) -> std::io::Result<bool> {
    if !link.exists() {
        return Ok(false);
    }

    let metadata = fs::symlink_metadata(link)?;
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }

    let target = fs::read_link(link)?;
    Ok(target == expected_target)
}

/// Verify a symlink (Windows - checks if file exists)
#[cfg(windows)]
pub fn verify_symlink(link: &Path, _expected_target: &Path) -> std::io::Result<bool> {
    // On Windows, shims are copies, not symlinks
    Ok(link.exists() && link.is_file())
}

/// Check if a path is a symlink
#[cfg(unix)]
pub fn is_symlink(path: &Path) -> std::io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(metadata.file_type().is_symlink())
}

#[cfg(windows)]
pub fn is_symlink(_path: &Path) -> std::io::Result<bool> {
    // Windows shims are copies, not symlinks
    Ok(false)
}

/// Clean up orphaned symlinks in the given directory
#[cfg(not(target_os = "windows"))]
pub fn cleanup_orphaned_symlinks(dir: &Path) -> Result<()> {
    debug!("Cleaning up orphaned symlinks in {}", dir.display());

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                if metadata.file_type().is_symlink() {
                    // Check if symlink target exists
                    if fs::metadata(&path).is_err() {
                        debug!("Removing orphaned symlink: {}", path.display());
                        if let Err(e) = fs::remove_file(&path) {
                            warn!(
                                "Failed to remove orphaned symlink {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Clean up orphaned symlinks (Windows - no-op)
#[cfg(target_os = "windows")]
pub fn cleanup_orphaned_symlinks(_dir: &Path) -> Result<()> {
    // Windows doesn't use symlinks for shims
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_cleanup_orphaned_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let symlink_path = temp_dir.path().join("test_link");
        let target_path = temp_dir.path().join("nonexistent_target");

        // Create a symlink to a non-existent target
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &symlink_path).unwrap();
            // Use symlink_metadata to check if symlink exists (not the target)
            assert!(std::fs::symlink_metadata(&symlink_path).is_ok()); // symlink exists
            assert!(!target_path.exists()); // but target doesn't

            cleanup_orphaned_symlinks(temp_dir.path()).unwrap();

            // Orphaned symlink should be removed
            assert!(std::fs::symlink_metadata(&symlink_path).is_err()); // symlink is gone
        }
    }
}
