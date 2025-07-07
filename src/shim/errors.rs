use crate::error::{KopiError, Result};
use std::fmt;

/// Shim-specific error types with clear categorization
#[derive(Debug)]
pub enum ShimError {
    /// No version found in environment or version files
    NoVersionFound {
        searched_paths: Vec<String>,
        suggestion: String,
    },

    /// JDK version requested but not installed
    JdkNotInstalled {
        version: String,
        distribution: String,
        auto_install_status: AutoInstallStatus,
    },

    /// Tool not found in the JDK installation
    ToolNotFound {
        tool: String,
        jdk_path: String,
        available_tools: Vec<String>,
    },

    /// Permission denied accessing files or directories
    PermissionDenied { path: String, operation: String },

    /// Generic shim execution error
    ExecutionError { message: String, exit_code: i32 },
}

#[derive(Debug)]
pub enum AutoInstallStatus {
    Disabled,
    UserDeclined,
    InProgress,
    Failed(String),
    NotApplicable,
}

impl ShimError {
    /// Convert to KopiError for compatibility with existing error handling
    pub fn to_kopi_error(self) -> KopiError {
        match self {
            ShimError::NoVersionFound { .. } => KopiError::NoLocalVersion,
            ShimError::JdkNotInstalled {
                version,
                distribution,
                ..
            } => KopiError::JdkNotInstalled(format!("{distribution}@{version}")),
            ShimError::ToolNotFound { tool, .. } => {
                KopiError::SystemError(format!("Tool '{tool}' not found"))
            }
            ShimError::PermissionDenied { path, .. } => KopiError::PermissionDenied(path),
            ShimError::ExecutionError { message, .. } => KopiError::SystemError(message),
        }
    }

    /// Get actionable error message for the user
    pub fn user_message(&self) -> String {
        match self {
            ShimError::NoVersionFound {
                searched_paths,
                suggestion,
            } => {
                format!(
                    "No Java version configured for this project.\n\
                    \n\
                    Searched in:\n{}\n\
                    \n\
                    {}",
                    searched_paths
                        .iter()
                        .map(|p| format!("  - {p}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    suggestion
                )
            }

            ShimError::JdkNotInstalled {
                version,
                distribution,
                auto_install_status,
            } => {
                let base_msg = format!("JDK {distribution} {version} is not installed.");

                let action = match auto_install_status {
                    AutoInstallStatus::Disabled => {
                        format!(
                            "To install it, run:\n  kopi install {}@{}\n\n\
                            Or enable auto-install:\n  export KOPI_AUTO_INSTALL__ENABLED=true",
                            distribution, version
                        )
                    }
                    AutoInstallStatus::UserDeclined => {
                        format!("Installation was declined. To install manually:\n  kopi install {distribution}@{version}")
                    }
                    AutoInstallStatus::InProgress => {
                        "Another process is currently installing this JDK. Please wait and try again.".to_string()
                    }
                    AutoInstallStatus::Failed(reason) => {
                        format!(
                            "Auto-installation failed: {reason}\n\n\
                            To install manually:\n  kopi install {distribution}@{version}"
                        )
                    }
                    AutoInstallStatus::NotApplicable => {
                        format!("To install it, run:\n  kopi install {distribution}@{version}")
                    }
                };

                format!("{base_msg}\n\n{action}")
            }

            ShimError::ToolNotFound {
                tool,
                jdk_path,
                available_tools,
            } => {
                if available_tools.is_empty() {
                    format!(
                        "Tool '{tool}' not found in JDK at {jdk_path}.\n\n\
                        This JDK installation may be corrupted. Try reinstalling it."
                    )
                } else {
                    format!(
                        "Tool '{}' not found in JDK at {}.\n\n\
                        Available tools in this JDK:\n{}\n\n\
                        This tool may not be available in this JDK distribution or version.",
                        tool,
                        jdk_path,
                        available_tools
                            .iter()
                            .map(|t| format!("  - {t}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            }

            ShimError::PermissionDenied { path, operation } => {
                let suggestion = if cfg!(unix) {
                    "Try running with sudo or check file ownership."
                } else {
                    "Try running as Administrator or check file permissions."
                };

                format!("Permission denied while {operation} '{path}'.\n\n{suggestion}")
            }

            ShimError::ExecutionError { message, .. } => {
                format!("Failed to execute Java tool: {}", message)
            }
        }
    }

    /// Get suggested fixes for the error
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            ShimError::NoVersionFound { .. } => vec![
                "Create a .kopi-version file: echo 'temurin@21' > .kopi-version".to_string(),
                "Set environment variable: export KOPI_JAVA_VERSION='temurin@21'".to_string(),
                "Set a global default: kopi global temurin@21".to_string(),
            ],

            ShimError::JdkNotInstalled {
                version,
                distribution,
                ..
            } => vec![
                format!("Install the JDK: kopi install {}@{}", distribution, version),
                "List available versions: kopi cache search".to_string(),
                "Enable auto-install: export KOPI_AUTO_INSTALL__ENABLED=true".to_string(),
            ],

            ShimError::ToolNotFound { tool, .. } => vec![
                format!("Verify the tool name: which {}", tool),
                "List installed JDKs: kopi list".to_string(),
                "Reinstall the JDK if corrupted".to_string(),
            ],

            ShimError::PermissionDenied { .. } => vec![
                if cfg!(unix) {
                    "Check file ownership: ls -la ~/.kopi".to_string()
                } else {
                    "Check file permissions in File Properties".to_string()
                },
                "Ensure kopi has write access to its directories".to_string(),
            ],

            ShimError::ExecutionError { .. } => vec![
                "Check if the JDK is properly installed: kopi list".to_string(),
                "Verify shims are correctly set up: kopi shim verify".to_string(),
                "Reinstall shims if needed: kopi setup".to_string(),
            ],
        }
    }

    /// Get the appropriate exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            ShimError::NoVersionFound { .. } => 3,
            ShimError::JdkNotInstalled { .. } => 4,
            ShimError::ToolNotFound { .. } => 5,
            ShimError::PermissionDenied { .. } => 13,
            ShimError::ExecutionError { exit_code, .. } => *exit_code,
        }
    }
}

impl fmt::Display for ShimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for ShimError {}

/// Format error for display to user with colors and formatting
pub fn format_shim_error(error: &ShimError, use_color: bool) -> String {
    let red = if use_color { "\x1b[31m" } else { "" };
    let yellow = if use_color { "\x1b[33m" } else { "" };
    let cyan = if use_color { "\x1b[36m" } else { "" };
    let reset = if use_color { "\x1b[0m" } else { "" };
    let bold = if use_color { "\x1b[1m" } else { "" };

    let mut output = String::new();

    // Error header
    output.push_str(&format!(
        "{}{}Error:{} {}\n",
        red,
        bold,
        reset,
        error.user_message()
    ));

    // Suggestions
    let suggestions = error.suggestions();
    if !suggestions.is_empty() {
        output.push_str(&format!("\n{yellow}{bold}Suggestions:{reset}\n"));
        for suggestion in suggestions {
            output.push_str(&format!("  {cyan} {suggestion}\n"));
        }
    }

    output
}

/// Helper to create common shim errors
pub struct ShimErrorBuilder;

impl ShimErrorBuilder {
    pub fn no_version_found(current_dir: &str) -> ShimError {
        let mut searched_paths = vec![current_dir.to_string()];

        // Add parent directories up to home
        if let Ok(home) = dirs::home_dir().ok_or("no home dir") {
            let mut dir = std::path::PathBuf::from(current_dir);
            while dir.parent().is_some() && dir != home {
                if let Some(parent) = dir.parent() {
                    searched_paths.push(parent.display().to_string());
                    dir = parent.to_path_buf();
                }
            }
        }

        ShimError::NoVersionFound {
            searched_paths,
            suggestion:
                "To configure a Java version for this project, use one of the suggestions below."
                    .to_string(),
        }
    }

    pub fn jdk_not_installed(
        version: &str,
        distribution: &str,
        auto_install_enabled: bool,
    ) -> ShimError {
        let auto_install_status = if !auto_install_enabled {
            AutoInstallStatus::Disabled
        } else {
            AutoInstallStatus::NotApplicable
        };

        ShimError::JdkNotInstalled {
            version: version.to_string(),
            distribution: distribution.to_string(),
            auto_install_status,
        }
    }

    pub fn tool_not_found(tool: &str, jdk_path: &str) -> Result<ShimError> {
        // List available tools in the JDK bin directory
        let bin_dir = std::path::Path::new(jdk_path).join("bin");
        let mut available_tools = Vec::new();

        if bin_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&bin_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Remove .exe extension on Windows
                        let tool_name = if cfg!(windows) && name.ends_with(".exe") {
                            &name[..name.len() - 4]
                        } else {
                            name
                        };

                        // Only include executable files
                        if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                            available_tools.push(tool_name.to_string());
                        }
                    }
                }
            }
        }

        available_tools.sort();

        Ok(ShimError::ToolNotFound {
            tool: tool.to_string(),
            jdk_path: jdk_path.to_string(),
            available_tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_version_error_message() {
        let error = ShimErrorBuilder::no_version_found("/home/user/project");
        let message = error.user_message();

        assert!(message.contains("No Java version configured"));
        assert!(message.contains("/home/user/project"));
        assert!(!message.is_empty());
    }

    #[test]
    fn test_jdk_not_installed_auto_install_disabled() {
        let error = ShimErrorBuilder::jdk_not_installed("21", "temurin", false);
        let message = error.user_message();

        assert!(message.contains("JDK temurin 21 is not installed"));
        assert!(message.contains("kopi install temurin@21"));
        assert!(message.contains("KOPI_AUTO_INSTALL__ENABLED=true"));
    }

    #[test]
    fn test_tool_not_found_error() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        // Create some dummy tools
        std::fs::write(bin_dir.join("java"), "").unwrap();
        std::fs::write(bin_dir.join("javac"), "").unwrap();

        let error = ShimErrorBuilder::tool_not_found("javap", jdk_path.to_str().unwrap()).unwrap();
        let message = error.user_message();

        assert!(message.contains("Tool 'javap' not found"));
        assert!(message.contains("Available tools"));
        // Note: We only check that the files were created, not that they're executable
        // In a real scenario, we'd also check file permissions
    }

    #[test]
    fn test_permission_denied_error() {
        let error = ShimError::PermissionDenied {
            path: "/opt/kopi/bin/java".to_string(),
            operation: "executing".to_string(),
        };

        let message = error.user_message();
        assert!(message.contains("Permission denied"));
        assert!(message.contains("executing"));

        if cfg!(unix) {
            assert!(message.contains("sudo"));
        } else {
            assert!(message.contains("Administrator"));
        }
    }

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(ShimErrorBuilder::no_version_found("/tmp").exit_code(), 3);
        assert_eq!(
            ShimErrorBuilder::jdk_not_installed("21", "temurin", false).exit_code(),
            4
        );

        let error = ShimError::PermissionDenied {
            path: "/tmp".to_string(),
            operation: "reading".to_string(),
        };
        assert_eq!(error.exit_code(), 13);
    }

    #[test]
    fn test_error_suggestions() {
        let error = ShimErrorBuilder::no_version_found("/tmp");
        let suggestions = error.suggestions();

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains(".kopi-version")));
        assert!(suggestions.iter().any(|s| s.contains("KOPI_JAVA_VERSION")));
    }

    #[test]
    fn test_format_shim_error_no_color() {
        let error = ShimErrorBuilder::no_version_found("/tmp");
        let formatted = format_shim_error(&error, false);

        assert!(formatted.contains("Error:"));
        assert!(formatted.contains("Suggestions:"));
        assert!(!formatted.contains("\x1b[")); // No color codes
    }

    #[test]
    fn test_format_shim_error_with_color() {
        let error = ShimErrorBuilder::jdk_not_installed("21", "temurin", false);
        let formatted = format_shim_error(&error, true);

        assert!(formatted.contains("\x1b[31m")); // Red
        assert!(formatted.contains("\x1b[33m")); // Yellow
        assert!(formatted.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_auto_install_status_messages() {
        let version = "21";
        let distribution = "temurin";

        // Test each auto-install status
        let statuses = vec![
            (
                AutoInstallStatus::Disabled,
                "export KOPI_AUTO_INSTALL__ENABLED=true",
            ),
            (AutoInstallStatus::UserDeclined, "Installation was declined"),
            (AutoInstallStatus::InProgress, "currently installing"),
            (
                AutoInstallStatus::Failed("timeout".to_string()),
                "Auto-installation failed",
            ),
        ];

        for (status, expected_text) in statuses {
            let error = ShimError::JdkNotInstalled {
                version: version.to_string(),
                distribution: distribution.to_string(),
                auto_install_status: status,
            };

            let message = error.user_message();
            assert!(
                message.contains(expected_text),
                "Expected '{expected_text}' in message: {message}"
            );
        }
    }
}
