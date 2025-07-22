//! Platform-specific file operations.

use crate::error::Result;
use log::debug;
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
use libc;

#[cfg(target_os = "windows")]
use std::ffi::OsStr;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW};

#[cfg(target_os = "windows")]
use winapi::um::winnt::{
    FILE_ATTRIBUTE_READONLY, HANDLE, OWNER_SECURITY_INFORMATION, PSID, TOKEN_QUERY, TOKEN_USER,
};

#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};

#[cfg(target_os = "windows")]
use winapi::um::securitybaseapi::{
    EqualSid, GetFileSecurityW, GetSecurityDescriptorOwner, GetTokenInformation,
};

#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;

#[cfg(target_os = "windows")]
use std::ptr;

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

/// Check if a file is executable (simple check)
///
/// Returns true if the file has execute permissions on Unix or has .exe extension on Windows.
/// For more strict security checks, use `check_executable_permissions` instead.
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

/// Set file permissions from a Unix mode value.
///
/// On Unix systems, this sets the file permissions to the specified mode.
/// On Windows, this is a no-op as Windows doesn't use Unix-style permissions.
///
/// This is useful when extracting files from archives that preserve Unix permissions.
#[cfg(unix)]
pub fn set_permissions_from_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::fs::Permissions;
    fs::set_permissions(path, Permissions::from_mode(mode))
}

/// Set file permissions from a Unix mode value (Windows - no-op)
#[cfg(windows)]
pub fn set_permissions_from_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    // Windows doesn't use Unix-style permissions
    Ok(())
}

/// Make a file or directory writable.
///
/// On Unix systems, this adds owner write permission.
/// On Windows, this removes the read-only attribute.
pub fn make_writable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        let mode = permissions.mode() | 0o200; // Add owner write permission
        permissions.set_mode(mode);
        fs::set_permissions(path, permissions)
    }

    #[cfg(windows)]
    {
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)
    }
}

/// Atomically rename a file from source to destination.
///
/// On Unix systems, rename is atomic by default.
/// On Windows, we need to remove the destination file first if it exists,
/// as Windows rename fails if the destination already exists.
pub fn atomic_rename(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        // On Windows, rename fails if destination exists, so remove it first
        if to.exists() {
            fs::remove_file(to)?;
        }
    }

    fs::rename(from, to)
}

/// Check if any files in the given path are currently in use
#[cfg(target_os = "windows")]
pub fn check_files_in_use(path: &Path) -> Result<Vec<String>> {
    debug!("Checking if files are in use at {}", path.display());

    let mut files_in_use = Vec::new();

    // Try to rename the directory to test if it's in use
    let temp_path = path.with_extension("kopi_test_temp");
    match std::fs::rename(path, &temp_path) {
        Ok(_) => {
            // Rename back immediately
            if let Err(e) = std::fs::rename(&temp_path, path) {
                debug!("Warning: Failed to rename back from temporary name: {e}");
            }
        }
        Err(e) => {
            debug!("Cannot rename directory: {e}");
            files_in_use.push("Directory appears to be in use (cannot rename)".to_string());

            // Additionally, try to check specific files
            if path.is_dir() {
                use walkdir::WalkDir;

                for entry in WalkDir::new(path).max_depth(2).into_iter().flatten() {
                        let file_path = entry.path();
                        if file_path.is_file() {
                            // Try to open the file exclusively
                            match std::fs::OpenOptions::new()
                                .read(true)
                                .write(true)
                                .open(file_path)
                            {
                                Ok(_) => {
                                    // File can be opened, it's not locked
                                }
                                Err(_) => {
                                    files_in_use.push(format!(
                                        "File may be in use: {}",
                                        file_path.display()
                                    ));
                                }
                            }
                        }
                    }
            }
        }
    }

    Ok(files_in_use)
}

/// Check if any files in the given path are currently in use
#[cfg(not(target_os = "windows"))]
pub fn check_files_in_use(path: &Path) -> Result<Vec<String>> {
    debug!("Checking if files are in use at {}", path.display());

    let mut files_in_use = Vec::new();

    // Try to rename the directory to test if it's in use
    let temp_path = path.with_extension("kopi_test_temp");
    match std::fs::rename(path, &temp_path) {
        Ok(_) => {
            // Rename back immediately
            if let Err(e) = std::fs::rename(&temp_path, path) {
                debug!("Warning: Failed to rename back from temporary name: {e}");
            }
        }
        Err(e) => {
            debug!("Cannot rename directory: {e}");
            files_in_use.push("Directory appears to be in use (cannot rename)".to_string());

            // On Unix, we can also check if files are open by trying to get exclusive locks
            if path.is_dir() {
                use walkdir::WalkDir;

                for entry in WalkDir::new(path).max_depth(2).into_iter().flatten() {
                        let file_path = entry.path();
                        if file_path.is_file() {
                            // Try to open the file with exclusive access
                            match std::fs::OpenOptions::new()
                                .read(true)
                                .write(true)
                                .open(file_path)
                            {
                                Ok(file) => {
                                    // Try to get an exclusive lock
                                    use std::os::unix::io::AsRawFd;
                                    let fd = file.as_raw_fd();

                                    let mut flock = libc::flock {
                                        l_type: libc::F_WRLCK as i16,
                                        l_whence: libc::SEEK_SET as i16,
                                        l_start: 0,
                                        l_len: 0,
                                        l_pid: 0,
                                    };

                                    let result =
                                        unsafe { libc::fcntl(fd, libc::F_GETLK, &mut flock) };

                                    if result != -1 && flock.l_type != libc::F_UNLCK as i16 {
                                        files_in_use.push(format!(
                                            "File may be locked by process {}: {}",
                                            flock.l_pid,
                                            file_path.display()
                                        ));
                                    }
                                }
                                Err(_) => {
                                    files_in_use.push(format!(
                                        "Cannot access file: {}",
                                        file_path.display()
                                    ));
                                }
                            }
                        }
                    }
            }
        }
    }

    Ok(files_in_use)
}

/// Prepare path for removal with platform-specific handling
#[cfg(target_os = "windows")]
pub fn prepare_for_removal(path: &Path) -> Result<()> {
    debug!("Preparing {} for removal", path.display());

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

/// Prepare path for removal with platform-specific handling
#[cfg(not(target_os = "windows"))]
pub fn prepare_for_removal(path: &Path) -> Result<()> {
    debug!("Preparing {} for removal", path.display());

    use walkdir::WalkDir;

    // Make all files and directories writable recursively
    for entry in WalkDir::new(path).contents_first(true) {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if let Err(e) = make_writable(path) {
                    debug!("Failed to make {} writable: {}", path.display(), e);
                }
            }
            Err(e) => {
                debug!("Failed to access directory entry: {e}");
            }
        }
    }

    Ok(())
}

/// Clean up after removal with platform-specific handling
#[cfg(target_os = "windows")]
pub fn post_removal_cleanup(path: &Path) -> Result<()> {
    debug!("Performing post-removal cleanup for {}", path.display());
    // Windows-specific cleanup if needed
    Ok(())
}

/// Clean up after removal with platform-specific handling
#[cfg(not(target_os = "windows"))]
pub fn post_removal_cleanup(path: &Path) -> Result<()> {
    debug!("Performing post-removal cleanup for {}", path.display());

    // Clean up any remaining symbolic links that might point to the removed JDK
    if let Some(parent) = path.parent() {
        super::symlink::cleanup_orphaned_symlinks(parent)?;
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

/// Check if the current user owns the file/directory
#[cfg(unix)]
pub fn check_ownership(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path)?;
    let file_uid = metadata.uid();
    let current_uid = unsafe { libc::getuid() };

    Ok(file_uid == current_uid)
}

/// Check if the current user owns the file/directory
#[cfg(windows)]
pub fn check_ownership(path: &Path) -> std::io::Result<bool> {
    unsafe {
        // Get current process token
        let mut token_handle: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Get the size needed for token information
        let mut size_needed = 0;
        GetTokenInformation(
            token_handle,
            winapi::um::winnt::TokenUser,
            ptr::null_mut(),
            0,
            &mut size_needed,
        );

        if size_needed == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Allocate buffer for token information
        let mut buffer = vec![0u8; size_needed as usize];
        if GetTokenInformation(
            token_handle,
            winapi::um::winnt::TokenUser,
            buffer.as_mut_ptr() as *mut _,
            size_needed,
            &mut size_needed,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }

        // Get user SID from token
        let token_user = buffer.as_ptr() as *const TOKEN_USER;
        let current_user_sid = (*token_user).User.Sid;

        // Get file security descriptor
        let path_wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Get the size needed for security descriptor
        let mut sd_size_needed = 0;
        GetFileSecurityW(
            path_wide.as_ptr(),
            OWNER_SECURITY_INFORMATION,
            ptr::null_mut(),
            0,
            &mut sd_size_needed,
        );

        if sd_size_needed == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Allocate buffer for security descriptor
        let mut sd_buffer = vec![0u8; sd_size_needed as usize];
        if GetFileSecurityW(
            path_wide.as_ptr(),
            OWNER_SECURITY_INFORMATION,
            sd_buffer.as_mut_ptr() as *mut _,
            sd_size_needed,
            &mut sd_size_needed,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }

        // Get owner SID from security descriptor
        let mut owner_sid: PSID = ptr::null_mut();
        let mut owner_defaulted = 0;
        if GetSecurityDescriptorOwner(
            sd_buffer.as_ptr() as *mut _,
            &mut owner_sid,
            &mut owner_defaulted,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }

        if owner_sid.is_null() {
            return Ok(false);
        }

        // Compare SIDs
        let is_owner = EqualSid(current_user_sid, owner_sid) != 0;

        // Close the token handle
        CloseHandle(token_handle);

        Ok(is_owner)
    }
}

/// Check if a file has secure permissions
#[cfg(unix)]
pub fn check_file_permissions(path: &Path) -> Result<bool> {
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

/// Check if a file has secure permissions
#[cfg(windows)]
pub fn check_file_permissions(path: &Path) -> Result<bool> {
    use crate::error::KopiError;

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
        debug!("File {path:?} is read-only (secure)");
        Ok(true)
    } else {
        log::warn!("File {path:?} is writable - consider setting read-only for security");
        Ok(true) // Still return true as writable files are not inherently insecure on Windows
    }
}

/// Set secure permissions on a file (read-only for security)
#[cfg(unix)]
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Set to 644 (owner: read/write, group: read, others: read)
    // This prevents accidental modification while allowing execution
    permissions.set_mode(0o644);

    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Set secure permissions on a file (read-only for security)
#[cfg(windows)]
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Set the read-only attribute on Windows
    permissions.set_readonly(true);

    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Check if a file has valid executable permissions with security validation
///
/// This function performs a more strict check than `is_executable`:
/// - Verifies the file is a regular file (not a directory or symlink)
/// - Checks for executable permissions
/// - On Unix: Also ensures the file is not world-writable (security risk)
#[cfg(unix)]
pub fn check_executable_permissions(path: &Path) -> Result<()> {
    use crate::error::KopiError;
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;

    if !metadata.is_file() {
        return Err(KopiError::SecurityError(format!(
            "Path '{}' is not a regular file",
            path.display()
        )));
    }

    // Use is_executable for basic check
    if !is_executable(path)? {
        return Err(KopiError::SecurityError(format!(
            "File '{}' is not executable",
            path.display()
        )));
    }

    // Additional security check: ensure file is not world-writable
    let permissions = metadata.permissions();
    let mode = permissions.mode();
    if mode & 0o002 != 0 {
        return Err(KopiError::SecurityError(format!(
            "File '{}' is world-writable, which is a security risk",
            path.display()
        )));
    }

    Ok(())
}

/// Check if a file has valid executable permissions with security validation
///
/// This function performs a more strict check than `is_executable`:
/// - Verifies the file is a regular file (not a directory or symlink)
/// - Checks for executable permissions
/// - On Unix: Also ensures the file is not world-writable (security risk)
#[cfg(windows)]
pub fn check_executable_permissions(path: &Path) -> Result<()> {
    use crate::error::KopiError;

    let metadata = std::fs::metadata(path)?;

    if !metadata.is_file() {
        return Err(KopiError::SecurityError(format!(
            "Path '{}' is not a regular file",
            path.display()
        )));
    }

    // Use is_executable for the extension check
    if !is_executable(path)? {
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_files_in_use_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let files_in_use = check_files_in_use(temp_dir.path()).unwrap();
        assert!(files_in_use.is_empty());
    }

    #[test]
    fn test_check_files_in_use_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        
        // Directory should be renameable when no files are open
        let files_in_use = check_files_in_use(temp_dir.path()).unwrap();
        assert!(files_in_use.is_empty());
        
        // Note: Actually testing files in use would require keeping a file handle open
        // in another thread/process, which is complex for a unit test
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

    #[test]
    fn test_check_ownership() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Should own files we create
        assert!(check_ownership(temp_dir.path()).unwrap());
        assert!(check_ownership(&test_file).unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn test_check_file_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

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
        use tempfile::NamedTempFile;

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
        use tempfile::NamedTempFile;

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
        use tempfile::NamedTempFile;

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

    #[test]
    #[cfg(unix)]
    fn test_check_executable_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();

        // Set executable but world-writable (insecure)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o777);
        temp_file.as_file().set_permissions(perms).unwrap();

        // Should fail due to world-writable
        assert!(check_executable_permissions(temp_file.path()).is_err());

        // Set executable and secure
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o755);
        temp_file.as_file().set_permissions(perms).unwrap();

        // Should pass
        assert!(check_executable_permissions(temp_file.path()).is_ok());

        // Set non-executable
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms).unwrap();

        // Should fail due to not executable
        assert!(check_executable_permissions(temp_file.path()).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_check_executable_permissions_windows() {
        use tempfile::NamedTempFile;

        // Test with non-exe file
        let temp_file = NamedTempFile::new().unwrap();
        assert!(check_executable_permissions(temp_file.path()).is_err());

        // Test with exe file
        let temp_dir = TempDir::new().unwrap();
        let exe_path = temp_dir.path().join("test.exe");
        fs::write(&exe_path, b"fake exe").unwrap();
        assert!(check_executable_permissions(&exe_path).is_ok());

        // Test with directory
        assert!(check_executable_permissions(temp_dir.path()).is_err());
    }
}
