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
use crate::error::{KopiError, Result};
use crate::version::VersionRequest;
use log::{debug, info, warn};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Result of an installation attempt
#[derive(Debug, PartialEq)]
pub enum InstallationResult {
    /// JDK was successfully installed
    Installed,
    /// User declined installation
    UserDeclined,
    /// Auto-install is disabled
    AutoInstallDisabled,
}

/// Handles automatic JDK installation when a requested version is not found
pub struct AutoInstaller<'a> {
    config: &'a KopiConfig,
}

impl<'a> AutoInstaller<'a> {
    /// Create a new AutoInstaller with the given configuration
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    /// Check if auto-installation is enabled in the configuration
    pub fn should_auto_install(&self) -> bool {
        self.config.auto_install.enabled
    }

    /// Prompt the user for confirmation if configured to do so
    /// Returns true if the user approves or prompting is disabled
    pub fn prompt_user(&self, version_spec: &str) -> Result<bool> {
        if !self.config.auto_install.prompt {
            // Auto-install without prompting
            return Ok(true);
        }

        print!("JDK {version_spec} is not installed. Would you like to install it now? [Y/n] ");
        io::stdout()
            .flush()
            .map_err(|e| KopiError::SystemError(e.to_string()))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| KopiError::SystemError(e.to_string()))?;

        let response = input.trim().to_lowercase();
        Ok(response.is_empty() || response == "y" || response == "yes")
    }

    /// Install a JDK by delegating to the main kopi binary
    pub fn install_jdk(&self, version_request: &VersionRequest) -> Result<()> {
        // Build the version specification for the install command
        // Append +fx if JavaFX is requested
        let mut version_spec = if let Some(dist) = &version_request.distribution {
            format!("{}@{}", dist, version_request.version_pattern)
        } else {
            version_request.version_pattern.clone()
        };

        // Add +fx suffix if JavaFX is requested
        if version_request.javafx_bundled == Some(true) {
            version_spec.push_str("+fx");
            debug!("Adding +fx suffix for JavaFX bundled version");
        }

        info!("Auto-installing JDK: {version_spec}");

        // Find the kopi binary
        let kopi_path = self.find_kopi_binary()?;

        // Build the install command
        let mut cmd = std::process::Command::new(&kopi_path);
        cmd.arg("install").arg(&version_spec);

        // Set timeout if configured
        let timeout_secs = self.config.auto_install.timeout_secs;
        debug!("Auto-install timeout: {timeout_secs} seconds");

        // Execute the installation with timeout
        match self.execute_with_timeout(cmd, Duration::from_secs(timeout_secs)) {
            Ok(status) if status.success() => {
                info!("Successfully auto-installed {version_spec}");
                Ok(())
            }
            Ok(status) => {
                warn!("Auto-install failed with status: {status:?}");
                Err(KopiError::SystemError(format!(
                    "Failed to install {version_spec}: command exited with status {status:?}"
                )))
            }
            Err(e) => {
                warn!("Auto-install error: {e}");
                Err(e)
            }
        }
    }

    /// Prompt user and install JDK if approved
    /// This is a common function used by both global and local commands
    pub fn prompt_and_install(
        &self,
        version_request: &VersionRequest,
    ) -> Result<InstallationResult> {
        if !self.should_auto_install() {
            return Ok(InstallationResult::AutoInstallDisabled);
        }

        // Prompt user for confirmation
        let user_approved = self.prompt_user(&version_request.version_pattern)?;

        if user_approved {
            println!("Installing JDK...");
            self.install_jdk(version_request)?;
            Ok(InstallationResult::Installed)
        } else {
            println!("Skipping installation.");
            println!("You can install this JDK later with:");
            println!("  kopi install {}", version_request.version_pattern);
            Ok(InstallationResult::UserDeclined)
        }
    }

    /// Execute a command with a timeout
    pub(crate) fn execute_with_timeout(
        &self,
        mut cmd: std::process::Command,
        timeout: Duration,
    ) -> Result<std::process::ExitStatus> {
        use std::thread;
        use std::time::Instant;

        let start = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|e| KopiError::SystemError(format!("Failed to spawn command: {e}")))?;

        // Poll the child process until it exits or times out
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {
                    // Still running
                    if start.elapsed() >= timeout {
                        // Timeout exceeded, kill the process
                        let _ = child.kill();
                        return Err(KopiError::SystemError(format!(
                            "Installation timed out after {} seconds",
                            timeout.as_secs()
                        )));
                    }
                    // Sleep briefly before checking again
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(KopiError::SystemError(format!(
                        "Failed to wait for command: {e}"
                    )));
                }
            }
        }
    }

    /// Find the kopi binary in the system
    fn find_kopi_binary(&self) -> Result<PathBuf> {
        let mut searched_paths = Vec::new();
        let kopi_name = crate::platform::kopi_binary_name();

        // Try to find kopi in the same directory as the current executable
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(parent) = current_exe.parent()
        {
            // On Windows, handle shims directory specially
            #[cfg(target_os = "windows")]
            {
                if let Ok(shims_dir) = self.config.shims_dir()
                    && parent == shims_dir
                    && let Ok(bin_dir) = self.config.bin_dir()
                {
                    let kopi_bin_path = bin_dir.join(kopi_name);
                    searched_paths.push(kopi_bin_path.display().to_string());
                    if kopi_bin_path.exists() {
                        return Ok(kopi_bin_path);
                    }
                }
            }

            // Check same directory as current executable
            let kopi_path = parent.join(kopi_name);
            searched_paths.push(kopi_path.display().to_string());
            if kopi_path.exists() {
                return Ok(kopi_path);
            }
        }

        // Fallback to searching in PATH
        searched_paths.push("PATH".to_string());
        if let Ok(kopi_in_path) = which::which(kopi_name) {
            return Ok(kopi_in_path);
        }

        // Return error with searched paths
        Err(KopiError::KopiNotFound {
            searched_paths,
            is_auto_install_context: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> KopiConfig {
        let temp_dir = TempDir::new().unwrap();
        // Clear environment variables that might interfere with config loading
        unsafe {
            std::env::remove_var("KOPI_STORAGE_MIN_DISK_SPACE_MB");
            std::env::remove_var("KOPI_AUTO_INSTALL_TIMEOUT_SECS");
            std::env::remove_var("KOPI_AUTO_INSTALL_ENABLED");
            std::env::remove_var("KOPI_CACHE_TTL_HOURS");
        }
        let mut config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        config.auto_install.enabled = true;
        config.auto_install.prompt = false;
        config.auto_install.timeout_secs = 30;
        config
    }

    #[test]
    fn test_should_auto_install() {
        let config = create_test_config();
        let installer = AutoInstaller::new(&config);
        assert!(installer.should_auto_install());

        // Test with disabled auto-install
        let mut config2 = config.clone();
        config2.auto_install.enabled = false;
        let installer2 = AutoInstaller::new(&config2);
        assert!(!installer2.should_auto_install());
    }

    #[test]
    fn test_prompt_user_no_prompt() {
        let config = create_test_config();
        let installer = AutoInstaller::new(&config);
        // When prompt is false, should return true without prompting
        assert!(installer.prompt_user("temurin@21").unwrap());
    }

    #[test]
    fn test_find_kopi_binary_not_found() {
        let config = create_test_config();
        let installer = AutoInstaller::new(&config);

        // Mock scenario where kopi is not found
        // This test documents expected behavior when kopi binary is not available
        let result = installer.find_kopi_binary();
        if let Err(e) = result {
            match e {
                KopiError::KopiNotFound {
                    searched_paths,
                    is_auto_install_context,
                } => {
                    assert!(!searched_paths.is_empty());
                    assert!(is_auto_install_context);
                }
                _ => panic!("Expected KopiNotFound error"),
            }
        }
    }

    #[test]
    fn test_execute_with_timeout() {
        let config = create_test_config();
        let installer = AutoInstaller::new(&config);

        // Test successful command
        #[cfg(unix)]
        let cmd = std::process::Command::new("true");

        #[cfg(windows)]
        let cmd = {
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "exit 0"]);
            c
        };

        let result = installer.execute_with_timeout(cmd, Duration::from_secs(5));
        assert!(result.is_ok());
        assert!(result.unwrap().success());
    }

    #[test]
    #[cfg(unix)]
    fn test_execute_with_timeout_exceeds() {
        let config = create_test_config();
        let installer = AutoInstaller::new(&config);

        // Test command that exceeds timeout
        let mut cmd = std::process::Command::new("sleep");
        cmd.arg("10");
        let result = installer.execute_with_timeout(cmd, Duration::from_secs(1));
        assert!(result.is_err());
        match result {
            Err(KopiError::SystemError(msg)) => {
                assert!(msg.contains("timed out") || msg.contains("Command execution timed out"));
            }
            _ => panic!("Expected timeout error"),
        }
    }
}
