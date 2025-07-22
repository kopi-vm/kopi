use crate::error::{KopiError, Result};
use log::{debug, warn};
use std::fs;
use std::path::Path;

/// Perform safety checks before uninstalling a JDK
pub fn perform_safety_checks(distribution: &str, version: &str) -> Result<()> {
    debug!("Performing safety checks for {distribution}@{version}");

    // Check if JDK is currently active (global)
    if is_active_global_jdk(distribution, version)? {
        return Err(KopiError::ValidationError(format!(
            "Cannot uninstall {distribution}@{version} - it is currently active globally. Use \
             --force to override this check"
        )));
    }

    // Check if JDK is currently active (local)
    if is_active_local_jdk(distribution, version)? {
        return Err(KopiError::ValidationError(format!(
            "Cannot uninstall {distribution}@{version} - it is currently active in this project. \
             Use --force to override this check"
        )));
    }

    // Check for running Java processes (future enhancement)
    check_running_processes(distribution, version)?;

    Ok(())
}

/// Check if the JDK is currently set as the global default
/// This is a stub function that always returns false for Phase 1
pub fn is_active_global_jdk(distribution: &str, version: &str) -> Result<bool> {
    debug!("Checking if {distribution}@{version} is active global JDK");
    // TODO: Implement actual global JDK detection when global command is implemented
    Ok(false)
}

/// Check if the JDK is currently active in the local project
/// This is a stub function that always returns false for Phase 1
pub fn is_active_local_jdk(distribution: &str, version: &str) -> Result<bool> {
    debug!("Checking if {distribution}@{version} is active local JDK");
    // TODO: Implement actual local JDK detection when local command is implemented
    Ok(false)
}

/// Check for running Java processes using this JDK
fn check_running_processes(_distribution: &str, _version: &str) -> Result<()> {
    // TODO: Future enhancement - check for running processes
    Ok(())
}

/// Verify user has permission to remove the directory
pub fn verify_removal_permission(path: &Path) -> Result<()> {
    debug!("Verifying removal permission for {}", path.display());

    // Check if path exists
    if !path.exists() {
        return Err(KopiError::DirectoryNotFound(path.display().to_string()));
    }

    // Try to check if we can write to parent directory (proxy for removal permission)
    if let Some(parent) = path.parent() {
        match fs::metadata(parent) {
            Ok(metadata) => {
                if metadata.permissions().readonly() {
                    return Err(KopiError::PermissionDenied(format!(
                        "Parent directory is read-only: {}",
                        parent.display()
                    )));
                }
            }
            Err(e) => {
                return Err(KopiError::PermissionDenied(format!(
                    "Cannot access parent directory: {e}"
                )));
            }
        }
    }

    Ok(())
}

/// Check if other tools depend on this JDK
pub fn check_tool_dependencies(path: &Path) -> Result<()> {
    debug!("Checking tool dependencies for {}", path.display());

    // TODO: Future enhancement - check if other tools have hardcoded paths to this JDK
    // For now, just warn about potential issues
    if path.join("bin/java").exists() {
        warn!("Note: Other tools may have references to this JDK installation");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_active_jdk_stubs() {
        // Test that stub functions always return false
        assert!(!is_active_global_jdk("temurin", "21.0.5+11").unwrap());
        assert!(!is_active_local_jdk("temurin", "21.0.5+11").unwrap());
        assert!(!is_active_global_jdk("corretto", "17.0.9").unwrap());
        assert!(!is_active_local_jdk("corretto", "17.0.9").unwrap());
    }

    #[test]
    fn test_safety_checks_pass_with_stubs() {
        // With stubs returning false, safety checks should pass
        assert!(perform_safety_checks("temurin", "21.0.5+11").is_ok());
        assert!(perform_safety_checks("corretto", "17.0.9").is_ok());
    }

    #[test]
    fn test_verify_removal_permission() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_jdk");
        fs::create_dir(&test_path).unwrap();

        // Should succeed for existing directory
        assert!(verify_removal_permission(&test_path).is_ok());

        // Should fail for non-existent directory
        let non_existent = temp_dir.path().join("non_existent");
        assert!(verify_removal_permission(&non_existent).is_err());
    }

    #[test]
    fn test_check_tool_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("jdk");

        // No warnings for empty directory
        fs::create_dir(&jdk_path).unwrap();
        assert!(check_tool_dependencies(&jdk_path).is_ok());

        // Should succeed even with java binary (just warns)
        fs::create_dir(jdk_path.join("bin")).unwrap();
        fs::write(jdk_path.join("bin/java"), "mock").unwrap();
        assert!(check_tool_dependencies(&jdk_path).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_permission_check_readonly_parent() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().join("readonly_parent");
        fs::create_dir(&parent).unwrap();

        let test_path = parent.join("jdk");
        fs::create_dir(&test_path).unwrap();

        // Make parent read-only
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&parent, perms).unwrap();

        // Should detect read-only parent
        let result = verify_removal_permission(&test_path);

        // Restore permissions before asserting (so cleanup works)
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&parent, perms).unwrap();

        assert!(result.is_err());
    }
}
