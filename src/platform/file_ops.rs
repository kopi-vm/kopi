//! Platform-specific file operations.

use std::path::Path;

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Make a file executable (Unix only)
#[cfg(unix)]
pub fn make_executable(path: &Path) -> std::io::Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Add execute permission for owner, group, and others (755)
    let mode = permissions.mode() | 0o755;
    permissions.set_mode(mode);

    fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Make a file executable (Windows - no-op)
#[cfg(windows)]
pub fn make_executable(_path: &Path) -> std::io::Result<()> {
    // Windows determines executability by file extension
    Ok(())
}

/// Check if a file is executable
#[cfg(unix)]
pub fn is_executable(path: &Path) -> std::io::Result<bool> {
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    Ok(permissions.mode() & 0o111 != 0)
}

#[cfg(windows)]
pub fn is_executable(path: &Path) -> std::io::Result<bool> {
    // On Windows, check for .exe extension
    Ok(path.extension().map(|ext| ext == "exe").unwrap_or(false))
}
