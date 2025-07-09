//! Platform-specific symlink operations.

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
