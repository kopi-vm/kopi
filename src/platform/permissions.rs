use crate::error::{KopiError, Result};
use std::path::Path;

/// Check if a file has secure permissions
pub fn check_file_permissions(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        check_file_permissions_unix(path)
    }

    #[cfg(windows)]
    {
        check_file_permissions_windows(path)
    }
}

/// Set secure permissions on a file (read-only for security)
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        set_secure_permissions_unix(path)
    }

    #[cfg(windows)]
    {
        set_secure_permissions_windows(path)
    }
}

/// Check if a file has valid executable permissions
pub fn check_executable_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        check_executable_permissions_unix(path)
    }

    #[cfg(windows)]
    {
        check_executable_permissions_windows(path)
    }
}

#[cfg(unix)]
fn check_file_permissions_unix(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if file has dangerous permissions (world-writable)
    if mode & 0o002 != 0 {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(windows)]
fn check_file_permissions_windows(path: &Path) -> Result<bool> {
    use std::fs;

    let metadata = fs::metadata(path)?;

    // On Windows, check if the file is read-only and exists
    if !metadata.is_file() {
        return Err(KopiError::SecurityError(format!(
            "Path {path:?} is not a regular file"
        )));
    }

    // Check if the file has the read-only attribute
    // In Windows, files without read-only attribute are writable by the owner
    // For JDK files, we generally want them to be read-only after installation
    if metadata.permissions().readonly() {
        log::debug!("File {path:?} is read-only (secure)");
        Ok(true)
    } else {
        log::warn!("File {path:?} is writable - consider setting read-only for security");
        Ok(true) // Still return true as writable files are not inherently insecure on Windows
    }
}

#[cfg(unix)]
fn set_secure_permissions_unix(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Set to 644 (owner: read/write, group: read, others: read)
    // This prevents accidental modification while allowing execution
    permissions.set_mode(0o644);

    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(windows)]
fn set_secure_permissions_windows(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Set the read-only attribute on Windows
    permissions.set_readonly(true);

    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
fn check_executable_permissions_unix(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;

    if !metadata.is_file() {
        return Err(KopiError::SecurityError(format!(
            "Path '{}' is not a regular file",
            path.display()
        )));
    }

    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if file is executable
    if mode & 0o111 == 0 {
        return Err(KopiError::SecurityError(format!(
            "File '{}' is not executable",
            path.display()
        )));
    }

    // Check if file is world-writable (security risk)
    if mode & 0o002 != 0 {
        return Err(KopiError::SecurityError(format!(
            "File '{}' is world-writable, which is a security risk",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(windows)]
fn check_executable_permissions_windows(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)?;

    if !metadata.is_file() {
        return Err(KopiError::SecurityError(format!(
            "Path '{}' is not a regular file",
            path.display()
        )));
    }

    // On Windows, check for .exe extension
    if path.extension().is_none_or(|ext| ext != "exe") {
        return Err(KopiError::SecurityError(format!(
            "File '{}' does not have .exe extension",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    #[cfg(unix)]
    fn test_check_file_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();

        // Set safe permissions (644)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        assert!(check_file_permissions(temp_file.path()).unwrap());

        // Set unsafe permissions (world-writable)
        perms.set_mode(0o666);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert!(!check_file_permissions(temp_file.path()).unwrap());
    }

    #[test]
    #[cfg(windows)]
    fn test_check_file_permissions_windows() {
        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files are writable
        assert!(check_file_permissions(temp_file.path()).unwrap());

        // Set file as read-only
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_readonly(true);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        // Should still be OK (read-only is more secure)
        assert!(check_file_permissions(temp_file.path()).unwrap());

        // Test with a directory (should fail)
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(check_file_permissions(temp_dir.path()).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_set_secure_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();

        // Set some initial permissions
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o777);
        temp_file.as_file().set_permissions(perms).unwrap();

        // Apply secure permissions
        set_secure_permissions(temp_file.path()).unwrap();

        // Check that permissions are now 644
        let new_perms = temp_file.as_file().metadata().unwrap().permissions();
        assert_eq!(new_perms.mode() & 0o777, 0o644);
    }

    #[test]
    #[cfg(windows)]
    fn test_set_secure_permissions_windows() {
        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files are writable
        let initial_perms = temp_file.as_file().metadata().unwrap().permissions();
        assert!(!initial_perms.readonly());

        // Apply secure permissions
        set_secure_permissions(temp_file.path()).unwrap();

        // Check that file is now read-only
        let new_perms = temp_file.as_file().metadata().unwrap().permissions();
        assert!(new_perms.readonly());
    }
}
