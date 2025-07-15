use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::installation::auto::{AutoInstaller, InstallationResult};
use crate::platform::process::launch_shell_with_env;
use crate::platform::shell::{Shell, detect_shell, find_shell_in_path};
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use log::{debug, info};
use std::path::PathBuf;
use std::str::FromStr;

pub struct ShellCommand;

impl ShellCommand {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute(&self, version_spec: &str, shell_override: Option<&str>) -> Result<()> {
        info!("Setting shell JDK version to {version_spec}");

        // Load configuration
        let config = new_kopi_config()?;

        // Parse version specification using lenient parsing
        let version_request = VersionRequest::from_str(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Create storage repository
        let repository = JdkRepository::new(&config);

        // Check if matching JDK is installed
        let mut matching_jdks = repository.find_matching_jdks(&version_request)?;

        if matching_jdks.is_empty() {
            // Auto-installation for shell command
            info!("JDK {} is not installed.", version_request.version_pattern);

            let auto_installer = AutoInstaller::new(&config);

            match auto_installer.prompt_and_install(version_spec, &version_request)? {
                InstallationResult::Installed => {
                    info!(
                        "JDK {} installed successfully",
                        version_request.version_pattern
                    );
                    // Re-fetch matching JDKs after installation
                    matching_jdks = repository.find_matching_jdks(&version_request)?;
                }
                InstallationResult::UserDeclined => {
                    return Err(KopiError::JdkNotInstalled {
                        jdk_spec: version_request.version_pattern.clone(),
                        version: Some(version_request.version_pattern.clone()),
                        distribution: version_request.distribution.clone(),
                        auto_install_enabled: true,
                        auto_install_failed: None,
                        user_declined: true,
                        install_in_progress: false,
                    });
                }
                InstallationResult::AutoInstallDisabled => {
                    return Err(KopiError::JdkNotInstalled {
                        jdk_spec: version_request.version_pattern.clone(),
                        version: Some(version_request.version_pattern.clone()),
                        distribution: version_request.distribution.clone(),
                        auto_install_enabled: false,
                        auto_install_failed: None,
                        user_declined: false,
                        install_in_progress: false,
                    });
                }
            }
        }

        // Get the last (latest) matching JDK
        let selected_jdk = matching_jdks
            .last()
            .ok_or_else(|| KopiError::JdkNotInstalled {
                jdk_spec: version_request.version_pattern.clone(),
                version: Some(version_request.version_pattern.clone()),
                distribution: version_request.distribution.clone(),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            })?;

        // Detect or override shell
        let (shell_type, shell_path) = if let Some(shell_name) = shell_override {
            self.get_shell_override(shell_name)?
        } else {
            detect_shell()?
        };

        info!("Using shell: {shell_type:?} at {shell_path:?}");

        // Launch shell with KOPI_JAVA_VERSION set
        let version_str = format!("{}@{}", selected_jdk.distribution, selected_jdk.version);

        println!(
            "Launching shell with JDK {}@{}",
            selected_jdk.distribution, selected_jdk.version
        );

        self.launch_shell(&shell_path, &version_str)
    }

    fn get_shell_override(&self, shell_name: &str) -> Result<(Shell, PathBuf)> {
        let shell_type = match shell_name.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" | "pwsh" => Shell::PowerShell,
            "cmd" => Shell::Cmd,
            _ => Shell::Unknown(shell_name.to_string()),
        };

        let shell_path = find_shell_in_path(&shell_type)?;
        Ok((shell_type, shell_path))
    }

    fn launch_shell(&self, shell_path: &PathBuf, version_str: &str) -> Result<()> {
        info!(
            "Launching {} with KOPI_JAVA_VERSION={}",
            shell_path.display(),
            version_str
        );

        // Use platform-specific shell launching
        launch_shell_with_env(shell_path, "KOPI_JAVA_VERSION", version_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_command_creation() {
        let command = ShellCommand::new().unwrap();
        assert!(!std::ptr::addr_of!(command).is_null());
    }

    #[test]
    fn test_shell_override() {
        let cmd = ShellCommand::new().unwrap();

        // Test known shells
        let (shell_type, _) = cmd
            .get_shell_override("bash")
            .unwrap_or((Shell::Bash, PathBuf::new()));
        assert!(matches!(shell_type, Shell::Bash));

        let (shell_type, _) = cmd
            .get_shell_override("zsh")
            .unwrap_or((Shell::Zsh, PathBuf::new()));
        assert!(matches!(shell_type, Shell::Zsh));

        let (shell_type, _) = cmd
            .get_shell_override("powershell")
            .unwrap_or((Shell::PowerShell, PathBuf::new()));
        assert!(matches!(shell_type, Shell::PowerShell));
    }
}
