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

use crate::config::KopiConfig;
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use crate::platform::file_ops::check_executable_permissions;
use crate::platform::{executable_extension, kopi_binary_name, shim_binary_name};
use std::fs;
use std::path::Path;
use std::time::Instant;
use which::which;

/// Check write permissions on kopi directories
pub struct DirectoryPermissionsCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> DirectoryPermissionsCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl DiagnosticCheck for DirectoryPermissionsCheck<'_> {
    fn name(&self) -> &str {
        "Directory Write Permissions"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        // Get kopi home directory
        let kopi_home = self.config.kopi_home();

        if !kopi_home.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cannot check permissions - kopi home does not exist",
                start.elapsed(),
            );
        }

        let mut permission_issues = Vec::new();

        // Check main kopi directory
        if let Err(e) = check_directory_writable(kopi_home) {
            permission_issues.push(format!("{}: {}", kopi_home.display(), e));
        }

        // Check subdirectories if they exist
        let subdirs = [
            ("jdks", self.config.jdks_dir()),
            ("shims", self.config.shims_dir()),
            ("cache", self.config.cache_dir()),
        ];

        for (name, dir_result) in subdirs {
            if let Ok(dir) = dir_result
                && dir.exists()
                && let Err(e) = check_directory_writable(&dir)
            {
                permission_issues.push(format!("{} ({}): {}", name, dir.display(), e));
            }
        }

        if permission_issues.is_empty() {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "All kopi directories have proper write permissions",
                start.elapsed(),
            )
        } else {
            let details = permission_issues.join("\n");

            #[cfg(unix)]
            let suggestion = format!(
                "Fix permissions with:\nsudo chown -R {}:{} {}",
                std::env::var("USER").unwrap_or_else(|_| "$(whoami)".to_string()),
                get_user_group(),
                kopi_home.display()
            );

            #[cfg(windows)]
            let suggestion =
                "Check Windows file permissions in Properties > Security tab".to_string();

            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "Some directories have permission issues",
                start.elapsed(),
            )
            .with_details(details)
            .with_suggestion(suggestion)
        }
    }
}

/// Check execute permissions on kopi binaries
pub struct BinaryPermissionsCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> BinaryPermissionsCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }
}

impl DiagnosticCheck for BinaryPermissionsCheck<'_> {
    fn name(&self) -> &str {
        "Binary Execute Permissions"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let mut permission_issues = Vec::new();

        // Check kopi binary
        let kopi_name = kopi_binary_name();
        if let Ok(kopi_path) = which(kopi_name)
            && let Err(e) = check_executable_permissions(&kopi_path)
        {
            permission_issues.push(format!("{}: {}", kopi_path.display(), e));
        }

        // Check shim binaries if shims directory exists
        if let Ok(shims_dir) = self.config.shims_dir()
            && shims_dir.exists()
        {
            // Check kopi-shim binary
            let shim_path = shims_dir.join(shim_binary_name());
            if shim_path.exists()
                && let Err(e) = check_executable_permissions(&shim_path)
            {
                permission_issues.push(format!("{}: {}", shim_path.display(), e));
            }

            // Check Java shims
            let java_shims = ["java", "javac", "jar", "javap", "jshell"];
            for shim_name in &java_shims {
                let shim_path = shims_dir
                    .join(shim_name)
                    .with_extension(executable_extension());
                if shim_path.exists()
                    && let Err(e) = check_executable_permissions(&shim_path)
                {
                    permission_issues.push(format!("{}: {}", shim_path.display(), e));
                }
            }
        }

        if permission_issues.is_empty() {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "All kopi binaries have proper execute permissions",
                start.elapsed(),
            )
        } else {
            let details = permission_issues.join("\n");

            #[cfg(unix)]
            let suggestion = "Fix permissions with: chmod +x <binary_path>";

            #[cfg(windows)]
            let suggestion = "Ensure files are not blocked. Right-click > Properties > Unblock";

            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                "Some binaries lack execute permissions",
                start.elapsed(),
            )
            .with_details(details)
            .with_suggestion(suggestion)
        }
    }
}

// Helper functions

fn check_directory_writable(path: &Path) -> Result<(), String> {
    // Try to create a temporary file to test write permissions
    let test_file = path.join(".kopi_permission_test");

    match fs::write(&test_file, b"test") {
        Ok(_) => {
            // Clean up test file
            let _ = fs::remove_file(&test_file);
            Ok(())
        }
        Err(e) => Err(format!("Not writable: {e}")),
    }
}

#[cfg(unix)]
fn get_user_group() -> String {
    use std::process::Command;

    Command::new("id")
        .arg("-gn")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "$(id -gn)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::{cache as cache_paths, home};
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_directory_permissions_check() {
        let temp_dir = TempDir::new().unwrap();

        // Create the kopi home directory structure
        let kopi_home = temp_dir.path().join(".kopi");
        fs::create_dir(&kopi_home).unwrap();

        // Create subdirectories
        home::ensure_jdks_dir(&kopi_home).unwrap();
        home::ensure_shims_dir(&kopi_home).unwrap();
        cache_paths::ensure_cache_root(&kopi_home).unwrap();

        unsafe {
            env::set_var("KOPI_HOME", &kopi_home);
        }
        let config = crate::config::new_kopi_config().unwrap();

        let check = DirectoryPermissionsCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Permissions);

        // Should pass since we just created the directories
        assert_eq!(result.status, CheckStatus::Pass);

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_binary_permissions_check_no_binaries() {
        let temp_dir = TempDir::new().unwrap();

        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        let config = crate::config::new_kopi_config().unwrap();

        let check = BinaryPermissionsCheck::new(&config);
        let start = Instant::now();
        let result = check.run(start, CheckCategory::Permissions);

        // Should pass when no binaries exist
        assert_eq!(result.status, CheckStatus::Pass);

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_check_directory_writable() {
        let temp_dir = TempDir::new().unwrap();

        // Should be writable
        assert!(check_directory_writable(temp_dir.path()).is_ok());

        // Non-existent directory
        let non_existent = temp_dir.path().join("does_not_exist");
        assert!(check_directory_writable(&non_existent).is_err());
    }
}
