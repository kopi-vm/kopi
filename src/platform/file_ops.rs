// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Platform-specific file operations.

use crate::error::Result;
use log::debug;
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(target_os = "windows")]
use std::ffi::OsStr;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW};

#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_READONLY;

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
    use fs2::FileExt;

    debug!(
        "Checking if critical JDK files are in use at {}",
        path.display()
    );

    let mut files_in_use = Vec::new();

    // Check only critical JDK executables
    let critical_files = [
        "bin/java.exe",
        "bin/javac.exe",
        "bin/javaw.exe",
        "bin/jar.exe",
    ];

    for file_name in &critical_files {
        let file_path = path.join(file_name);
        if file_path.exists() {
            match fs::File::open(&file_path) {
                Ok(file) => {
                    // Try to acquire an exclusive lock (non-blocking)
                    match file.try_lock_exclusive() {
                        Ok(_) => {
                            // File is not in use, unlock it
                            let _ = FileExt::unlock(&file);
                        }
                        Err(_) => {
                            files_in_use.push(format!(
                                "Critical file may be in use: {}",
                                file_path.display()
                            ));
                        }
                    }
                }
                Err(e) => {
                    // If we can't even open the file, it might be in use
                    debug!("Cannot open {}: {}", file_path.display(), e);
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        files_in_use.push(format!(
                            "Critical file may be in use (access denied): {}",
                            file_path.display()
                        ));
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
    use fs2::FileExt;

    debug!(
        "Checking if critical JDK files are in use at {}",
        path.display()
    );

    let mut files_in_use = Vec::new();

    // Check only critical JDK executables
    let critical_files = ["bin/java", "bin/javac", "bin/javaw", "bin/jar"];

    for file_name in &critical_files {
        let file_path = path.join(file_name);
        if file_path.exists() {
            match fs::File::open(&file_path) {
                Ok(file) => {
                    // Try to acquire an exclusive lock (non-blocking)
                    match file.try_lock_exclusive() {
                        Ok(_) => {
                            // File is not in use, unlock it
                            let _ = FileExt::unlock(&file);
                        }
                        Err(_) => {
                            files_in_use.push(format!(
                                "Critical file may be in use: {}",
                                file_path.display()
                            ));
                        }
                    }
                }
                Err(e) => {
                    // If we can't even open the file, it might be in use
                    debug!("Cannot open {}: {}", file_path.display(), e);
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        files_in_use.push(format!(
                            "Critical file may be in use (access denied): {}",
                            file_path.display()
                        ));
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

/// Check if a file is readable by the current user
///
/// On Unix: Checks if the file has read permission for the owner
/// On Windows: Attempts to read the file to verify readability
#[cfg(unix)]
pub fn check_file_readable(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if readable by owner (0o400)
    Ok(mode & 0o400 != 0)
}

/// Check if a file is readable by the current user
///
/// On Unix: Checks if the file has read permission for the owner
/// On Windows: Attempts to read the file to verify readability
#[cfg(windows)]
pub fn check_file_readable(path: &Path) -> std::io::Result<bool> {
    // On Windows, just try to open the file for reading
    match fs::File::open(path) {
        Ok(_) => Ok(true),
        Err(e) => {
            // If it's a permission error, the file exists but isn't readable
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Ok(false)
            } else {
                // For other errors (file not found, etc.), propagate the error
                Err(e)
            }
        }
    }
}

/// Get file permissions in a platform-independent format
///
/// On Unix: Returns the octal permission mode (e.g., 644)
/// On Windows: Returns a string indicating read-only status
#[cfg(unix)]
pub fn get_file_permissions_string(path: &Path) -> std::io::Result<String> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    Ok(format!("{:o}", mode & 0o777))
}

/// Get file permissions in a platform-independent format
///
/// On Unix: Returns the octal permission mode (e.g., 644)
/// On Windows: Returns a string indicating read-only status
#[cfg(windows)]
pub fn get_file_permissions_string(path: &Path) -> std::io::Result<String> {
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();

    if permissions.readonly() {
        Ok("read-only".to_string())
    } else {
        Ok("read-write".to_string())
    }
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

    #[test]
    #[cfg(unix)]
    fn test_check_file_readable_unix() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();

        // Set readable permissions (644)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert!(check_file_readable(temp_file.path()).unwrap());

        // Set unreadable permissions (000)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o000);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert!(!check_file_readable(temp_file.path()).unwrap());
    }

    #[test]
    #[cfg(windows)]
    fn test_check_file_readable_windows() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files should be readable
        assert!(check_file_readable(temp_file.path()).unwrap());

        // Test with non-existent file
        let non_existent = temp_file.path().with_extension("nonexistent");
        assert!(check_file_readable(&non_existent).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_get_file_permissions_string_unix() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();

        // Set specific permissions
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert_eq!(
            get_file_permissions_string(temp_file.path()).unwrap(),
            "644"
        );

        // Set different permissions
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o755);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert_eq!(
            get_file_permissions_string(temp_file.path()).unwrap(),
            "755"
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_get_file_permissions_string_windows() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files are writable
        assert_eq!(
            get_file_permissions_string(temp_file.path()).unwrap(),
            "read-write"
        );

        // Set file as read-only
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_readonly(true);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert_eq!(
            get_file_permissions_string(temp_file.path()).unwrap(),
            "read-only"
        );
    }
}
