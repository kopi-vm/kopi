use crate::error::{KopiError, Result};
use crate::platform::{self, shim_binary_name};
use std::fs;
use std::path::{Path, PathBuf};

/// Installs, removes, and verifies shims for JDK tools
pub struct ShimInstaller {
    shims_dir: PathBuf,
    kopi_bin_path: PathBuf,
}

impl ShimInstaller {
    /// Create a new ShimInstaller with the specified shims directory
    pub fn new(kopi_home: &Path) -> Self {
        Self {
            shims_dir: kopi_home.join("shims"),
            kopi_bin_path: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("kopi")),
        }
    }

    /// Get the shims directory path
    pub fn shims_dir(&self) -> &Path {
        &self.shims_dir
    }

    /// Initialize the shims directory
    pub fn init_shims_directory(&self) -> Result<()> {
        if !self.shims_dir.exists() {
            fs::create_dir_all(&self.shims_dir)?;
            log::info!("Created shims directory: {:?}", self.shims_dir);
        }
        Ok(())
    }

    /// Create a shim for the specified tool
    pub fn create_shim(&self, tool_name: &str) -> Result<()> {
        self.init_shims_directory()?;

        let shim_path = self.get_shim_path(tool_name);

        // Check if shim already exists
        if shim_path.exists() {
            return Err(KopiError::SystemError(format!(
                "Shim for '{tool_name}' already exists at {shim_path:?}"
            )));
        }

        self.create_shim_internal(tool_name, &shim_path)?;

        log::info!("Created shim for '{tool_name}' at {shim_path:?}");
        Ok(())
    }

    /// Remove a shim for the specified tool
    pub fn remove_shim(&self, tool_name: &str) -> Result<()> {
        let shim_path = self.get_shim_path(tool_name);

        if !shim_path.exists() {
            return Err(KopiError::SystemError(format!(
                "Shim for '{tool_name}' does not exist"
            )));
        }

        fs::remove_file(&shim_path)?;
        log::info!("Removed shim for '{tool_name}' from {shim_path:?}");
        Ok(())
    }

    /// List all installed shims
    pub fn list_shims(&self) -> Result<Vec<String>> {
        if !self.shims_dir.exists() {
            return Ok(Vec::new());
        }

        let mut shims = Vec::new();

        for entry in fs::read_dir(&self.shims_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        shims.push(name_str.to_string());
                    }
                }
            }
        }

        shims.sort();
        Ok(shims)
    }

    /// Verify all shims and return list of broken ones
    pub fn verify_shims(&self) -> Result<Vec<(String, String)>> {
        let mut broken_shims = Vec::new();

        if !self.shims_dir.exists() {
            return Ok(broken_shims);
        }

        for entry in fs::read_dir(&self.shims_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        if let Err(e) = platform::shim::verify_shim(&path) {
                            broken_shims.push((name_str.to_string(), e.to_string()));
                        }
                    }
                }
            }
        }

        Ok(broken_shims)
    }

    /// Repair a broken shim
    pub fn repair_shim(&self, tool_name: &str) -> Result<()> {
        let shim_path = self.get_shim_path(tool_name);

        // Remove the broken shim
        if shim_path.exists() {
            fs::remove_file(&shim_path)?;
        }

        // Recreate it
        self.create_shim_internal(tool_name, &shim_path)?;

        log::info!("Repaired shim for '{tool_name}'");
        Ok(())
    }

    /// Create shims for any tools that don't already have them
    pub fn create_missing_shims(&self, tools: &[String]) -> Result<Vec<String>> {
        self.init_shims_directory()?;

        let mut created_shims = Vec::new();

        for tool in tools {
            let shim_path = self.get_shim_path(tool);

            // Skip if shim already exists
            if shim_path.exists() {
                log::debug!("Shim for '{tool}' already exists");
                continue;
            }

            // Create the shim
            self.create_shim_internal(tool, &shim_path)?;

            log::info!("Created shim for '{tool}' at {shim_path:?}");
            created_shims.push(tool.clone());
        }

        Ok(created_shims)
    }

    /// Get the path for a shim
    fn get_shim_path(&self, tool_name: &str) -> PathBuf {
        let shim_name = if platform::executable_extension().is_empty() {
            tool_name.to_string()
        } else {
            format!("{}{}", tool_name, platform::executable_extension())
        };
        self.shims_dir.join(shim_name)
    }

    /// Internal method to create a shim with platform-specific implementation
    fn create_shim_internal(&self, _tool_name: &str, shim_path: &Path) -> Result<()> {
        // Find the kopi-shim binary
        let kopi_shim_path = self.find_kopi_shim_binary()?;

        #[cfg(unix)]
        {
            // Create symlink to kopi-shim
            platform::symlink::create_symlink(&kopi_shim_path, shim_path)?;
            // Ensure symlink is executable (symlinks inherit permissions from target)
        }

        #[cfg(windows)]
        {
            // On Windows, we need to copy the kopi-shim.exe for each tool
            // Copy the file
            platform::symlink::create_symlink(&kopi_shim_path, shim_path)?;
        }

        Ok(())
    }

    fn find_kopi_shim_binary(&self) -> Result<PathBuf> {
        // Look for kopi-shim in the same directory as the kopi binary
        let kopi_dir = self.kopi_bin_path.parent().ok_or_else(|| {
            KopiError::SystemError("Cannot determine kopi binary directory".to_string())
        })?;

        let shim_path = kopi_dir.join(shim_binary_name());

        if !shim_path.exists() {
            return Err(KopiError::SystemError(format!(
                "kopi-shim binary not found at {shim_path:?}. Please run 'kopi setup' first."
            )));
        }

        Ok(shim_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_shim_installer_new() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ShimInstaller::new(temp_dir.path());

        assert_eq!(installer.shims_dir(), temp_dir.path().join("shims"));
    }

    #[test]
    fn test_init_shims_directory() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ShimInstaller::new(temp_dir.path());

        assert!(!installer.shims_dir().exists());

        installer.init_shims_directory().unwrap();

        assert!(installer.shims_dir().exists());
        assert!(installer.shims_dir().is_dir());
    }

    #[test]
    fn test_get_shim_path() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ShimInstaller::new(temp_dir.path());

        let java_shim = installer.get_shim_path("java");

        let expected_name = if platform::executable_extension().is_empty() {
            "java"
        } else {
            "java.exe"
        };
        assert_eq!(java_shim, temp_dir.path().join("shims").join(expected_name));
    }

    #[test]
    fn test_list_shims_empty() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ShimInstaller::new(temp_dir.path());

        let shims = installer.list_shims().unwrap();
        assert!(shims.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_shim() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ShimInstaller::new(temp_dir.path());

        let result = installer.remove_shim("java");
        assert!(result.is_err());
        assert!(matches!(result, Err(KopiError::SystemError(_))));
    }

    // Note: More comprehensive tests for create_shim, verify_shims, etc.
    // would require mocking the kopi-shim binary existence and filesystem
    // operations, which will be done in the integration tests
}
