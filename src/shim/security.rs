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

use crate::error::KopiError;
use std::fs;
use std::path::{Path, PathBuf};

use super::tools::ToolRegistry;
use crate::config::KopiConfig;

pub struct SecurityValidator {
    kopi_home: PathBuf,
    tool_registry: ToolRegistry,
}

impl SecurityValidator {
    pub fn new(config: &KopiConfig) -> Self {
        Self {
            kopi_home: config.kopi_home().to_path_buf(),
            tool_registry: ToolRegistry::new(),
        }
    }

    pub fn validate_path(&self, path: &Path) -> Result<(), KopiError> {
        let canonical_path = path.canonicalize().map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to canonicalize path '{}': {}",
                path.display(),
                e
            ))
        })?;

        let canonical_kopi_home = self.kopi_home.canonicalize().map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to canonicalize KOPI_HOME '{}': {}",
                self.kopi_home.display(),
                e
            ))
        })?;

        if !canonical_path.starts_with(&canonical_kopi_home) {
            return Err(KopiError::SecurityError(format!(
                "Path '{}' is outside KOPI_HOME directory",
                path.display()
            )));
        }

        if path.components().any(|c| c.as_os_str() == "..") {
            return Err(KopiError::SecurityError(
                "Path contains directory traversal components (..)".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate_version(&self, version: &str) -> Result<(), KopiError> {
        if version.is_empty() {
            return Err(KopiError::ValidationError(
                "Version string cannot be empty".to_string(),
            ));
        }

        if version.len() > 100 {
            return Err(KopiError::ValidationError(
                "Version string is too long (max 100 characters)".to_string(),
            ));
        }

        let valid_chars = |c: char| c.is_alphanumeric() || matches!(c, '@' | '.' | '-' | '_' | '+');

        if !version.chars().all(valid_chars) {
            return Err(KopiError::ValidationError(format!(
                "Version '{version}' contains invalid characters. Only alphanumeric and @.-_+ are \
                 allowed"
            )));
        }

        if version.contains("..") || version.contains("//") {
            return Err(KopiError::SecurityError(
                "Version string contains suspicious patterns".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate_tool(&self, tool: &str) -> Result<(), KopiError> {
        if self.tool_registry.get_tool(tool).is_none() {
            return Err(KopiError::ValidationError(format!(
                "'{tool}' is not a recognized JDK tool"
            )));
        }
        Ok(())
    }

    pub fn check_permissions(&self, path: &Path) -> Result<(), KopiError> {
        crate::platform::file_ops::check_executable_permissions(path)
    }

    pub fn validate_symlink(&self, symlink_path: &Path) -> Result<(), KopiError> {
        if !symlink_path.is_symlink() {
            return Ok(());
        }

        let target = fs::read_link(symlink_path).map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to read symlink target for '{}': {}",
                symlink_path.display(),
                e
            ))
        })?;

        let absolute_target = if target.is_relative() {
            symlink_path
                .parent()
                .ok_or_else(|| {
                    KopiError::SystemError(format!(
                        "Symlink '{}' has no parent directory",
                        symlink_path.display()
                    ))
                })?
                .join(&target)
        } else {
            target
        };

        self.validate_path(&absolute_target)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::install;
    #[cfg(unix)]
    use std::fs::File;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn create_test_validator() -> (SecurityValidator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        // Create necessary directories for config
        std::fs::create_dir_all(temp_dir.path()).unwrap();
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let validator = SecurityValidator::new(&config);
        (validator, temp_dir)
    }

    #[test]
    fn test_validate_path_within_kopi_home() {
        let (validator, temp_dir) = create_test_validator();
        install::ensure_installations_root(temp_dir.path()).unwrap();
        let valid_path = install::installation_directory(temp_dir.path(), "java-11");
        std::fs::create_dir_all(&valid_path).unwrap();

        assert!(validator.validate_path(&valid_path).is_ok());
    }

    #[test]
    fn test_validate_path_outside_kopi_home() {
        let (validator, _temp_dir) = create_test_validator();
        let invalid_path = Path::new("/etc/passwd");

        let result = validator.validate_path(invalid_path);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(KopiError::SecurityError(_)) | Err(KopiError::SystemError(_))
        ));
    }

    #[test]
    fn test_validate_version_valid() {
        let (validator, _) = create_test_validator();

        assert!(validator.validate_version("21").is_ok());
        assert!(validator.validate_version("temurin@21.0.1").is_ok());
        assert!(validator.validate_version("graalvm-ce@22.3.0").is_ok());
        assert!(validator.validate_version("11.0.2_9").is_ok());
        assert!(validator.validate_version("17+35").is_ok());
    }

    #[test]
    fn test_validate_version_invalid() {
        let (validator, _) = create_test_validator();

        assert!(validator.validate_version("").is_err());
        assert!(validator.validate_version("../../../etc/passwd").is_err());
        assert!(validator.validate_version("java; rm -rf /").is_err());
        assert!(validator.validate_version("java\necho hacked").is_err());
        assert!(validator.validate_version(&"x".repeat(101)).is_err());
    }

    #[test]
    fn test_validate_tool_valid() {
        let (validator, _) = create_test_validator();

        assert!(validator.validate_tool("java").is_ok());
        assert!(validator.validate_tool("javac").is_ok());
        assert!(validator.validate_tool("native-image").is_ok());
    }

    #[test]
    fn test_validate_tool_invalid() {
        let (validator, _) = create_test_validator();

        assert!(validator.validate_tool("unknown-tool").is_err());
        assert!(validator.validate_tool("rm").is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_check_permissions_unix() {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;

        let (validator, temp_dir) = create_test_validator();
        let exec_file = temp_dir.path().join("executable");
        File::create(&exec_file).unwrap();

        fs::set_permissions(&exec_file, Permissions::from_mode(0o755)).unwrap();
        assert!(validator.check_permissions(&exec_file).is_ok());

        fs::set_permissions(&exec_file, Permissions::from_mode(0o644)).unwrap();
        assert!(validator.check_permissions(&exec_file).is_err());

        fs::set_permissions(&exec_file, Permissions::from_mode(0o777)).unwrap();
        let result = validator.check_permissions(&exec_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("world-writable"));
    }

    #[test]
    #[cfg(unix)]
    fn test_validate_symlink() {
        let (validator, temp_dir) = create_test_validator();
        let target = temp_dir.path().join("target");
        let link = temp_dir.path().join("link");

        File::create(&target).unwrap();
        symlink(&target, &link).unwrap();

        assert!(validator.validate_symlink(&link).is_ok());

        let bad_link = temp_dir.path().join("bad_link");
        symlink("/etc/passwd", &bad_link).unwrap();
        assert!(validator.validate_symlink(&bad_link).is_err());
    }
}
