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
use std::fmt;

pub struct ErrorContext<'a> {
    pub error: &'a KopiError,
    pub suggestion: Option<String>,
    pub details: Option<String>,
}

impl<'a> ErrorContext<'a> {
    pub fn new(error: &'a KopiError) -> Self {
        let (suggestion, details) = match error {
            KopiError::VersionNotAvailable(msg) => {
                let suggestion = Some(
                    "Run 'kopi cache search' to see available versions or 'kopi cache refresh' to \
                     update the list."
                        .to_string(),
                );
                let details = Some(format!("Version lookup failed: {msg}"));
                (suggestion, details)
            }
            KopiError::InvalidVersionFormat(msg) => {
                let suggestion = Some(
                    "Version format should be: '<version>' or '<distribution>@<version>' (e.g., \
                     '21' or 'corretto@17')."
                        .to_string(),
                );
                let details = Some(format!("Invalid format: {msg}"));
                (suggestion, details)
            }
            KopiError::JdkNotInstalled {
                jdk_spec,
                auto_install_enabled,
                auto_install_failed,
                user_declined,
                install_in_progress,
                ..
            } => {
                let suggestion = if *install_in_progress {
                    Some(
                        "Another process is currently installing this JDK. Please wait and try \
                         again."
                            .to_string(),
                    )
                } else if *user_declined {
                    Some(format!(
                        "Installation was declined. To install manually: kopi install {jdk_spec}"
                    ))
                } else if let Some(reason) = auto_install_failed {
                    Some(format!(
                        "Auto-installation failed: {reason}\n\nTo install manually: kopi install \
                         {jdk_spec}"
                    ))
                } else if *auto_install_enabled {
                    Some(format!(
                        "Run 'kopi install {jdk_spec}' to install this JDK."
                    ))
                } else {
                    let enable_cmd = if cfg!(windows) {
                        "set KOPI_AUTO_INSTALL__ENABLED=true"
                    } else {
                        "export KOPI_AUTO_INSTALL__ENABLED=true"
                    };
                    Some(format!(
                        "Run 'kopi install {jdk_spec}' to install this JDK.\n\nOr enable \
                         auto-install: {enable_cmd}"
                    ))
                };
                let details = None;
                (suggestion, details)
            }
            KopiError::Download(msg) => {
                let suggestion = Some(
                    "Check your internet connection and try again. Use --timeout to increase \
                     timeout if needed."
                        .to_string(),
                );
                let details = Some(format!("Download failed: {msg}"));
                (suggestion, details)
            }
            KopiError::Extract(msg) => {
                let suggestion =
                    Some("Ensure you have enough disk space and try again.".to_string());
                let details = Some(format!("Extraction failed: {msg}"));
                (suggestion, details)
            }
            KopiError::ChecksumMismatch => {
                let suggestion = Some(
                    "Try downloading again. If the problem persists, the file may be corrupted at \
                     the source."
                        .to_string(),
                );
                let details = Some(
                    "The downloaded file's checksum doesn't match the expected value.".to_string(),
                );
                (suggestion, details)
            }
            KopiError::NoLocalVersion { searched_paths } => {
                let suggestion = Some(
                    "To configure a Java version for this project:\n  - Create a .kopi-version \
                     file: echo 'temurin@21' > .kopi-version\n  - Set for this directory: kopi local temurin@21\n  - Set a global default: kopi global temurin@21"
                        .to_string(),
                );
                let details = if searched_paths.is_empty() {
                    None
                } else {
                    Some(format!(
                        "Searched in:\n{}",
                        searched_paths
                            .iter()
                            .map(|p| format!("  - {p}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                };
                (suggestion, details)
            }
            KopiError::PermissionDenied(path) => {
                let suggestion = if cfg!(unix) {
                    Some(format!(
                        "Try running with sudo or ensure you have write permissions to: {path}"
                    ))
                } else {
                    Some(format!(
                        "Run as Administrator or ensure you have write permissions to: {path}"
                    ))
                };
                let details = None;
                (suggestion, details)
            }
            KopiError::DiskSpaceError(msg) => {
                let suggestion = Some(
                    "Free up disk space and try again. JDK installations typically require \
                     300-500MB."
                        .to_string(),
                );
                let details = Some(format!("Disk space issue: {msg}"));
                (suggestion, details)
            }
            KopiError::NetworkError(msg) => {
                let suggestion = Some(
                    "Check your internet connection and proxy settings. Try 'kopi cache refresh' \
                     to update metadata."
                        .to_string(),
                );
                let details = Some(format!("Network issue: {msg}"));
                (suggestion, details)
            }
            KopiError::Http(http_err) => {
                let error_string = http_err.to_string();
                let suggestion = if error_string.contains("timeout")
                    || error_string.contains("Timeout")
                {
                    Some(
                        "Try increasing the timeout with --timeout option (e.g., --timeout 300)."
                            .to_string(),
                    )
                } else if error_string.contains("429") {
                    Some(
                        "API rate limit exceeded. Please wait a few minutes and try again."
                            .to_string(),
                    )
                } else if error_string.contains("404") {
                    Some(
                        "The requested resource was not found. Try 'kopi cache refresh' to update \
                         available versions."
                            .to_string(),
                    )
                } else if error_string.contains("redirect") || error_string.contains("Redirect") {
                    Some("The download URL has too many redirects. Try again later.".to_string())
                } else {
                    Some("Check your internet connection and try again.".to_string())
                };
                let details = Some(format!("HTTP error: {http_err}"));
                (suggestion, details)
            }
            KopiError::AlreadyExists(msg) => {
                let suggestion =
                    Some("Use --force to overwrite the existing installation.".to_string());
                let details = Some(msg.clone());
                (suggestion, details)
            }
            KopiError::DirectoryNotFound(dir) => {
                let suggestion = Some(format!("Ensure the directory exists: {dir}"));
                let details = None;
                (suggestion, details)
            }
            KopiError::CacheNotFound => {
                let suggestion =
                    Some("Run 'kopi cache refresh' to fetch the latest JDK metadata.".to_string());
                let details = Some("No cached metadata found.".to_string());
                (suggestion, details)
            }
            KopiError::Io(io_err) => {
                let suggestion = match io_err.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        if cfg!(unix) {
                            Some("Try running with sudo or check file permissions.".to_string())
                        } else {
                            Some("Run as Administrator or check file permissions.".to_string())
                        }
                    }
                    std::io::ErrorKind::NotFound => Some(
                        "Ensure the file or directory exists and the path is correct.".to_string(),
                    ),
                    std::io::ErrorKind::AlreadyExists => Some(
                        "The file already exists. Use --force to overwrite if applicable."
                            .to_string(),
                    ),
                    _ => None,
                };
                let details = Some(format!("I/O error: {io_err}"));
                (suggestion, details)
            }
            KopiError::ToolNotFound {
                tool: _,
                jdk_path,
                available_tools,
            } => {
                let suggestion = if available_tools.is_empty() {
                    Some(format!(
                        "This JDK installation at {jdk_path} may be corrupted. Try reinstalling \
                         it."
                    ))
                } else {
                    Some(format!(
                        "Available tools in this JDK:\n{}\n\nThis tool may not be available in \
                         this JDK distribution or version.",
                        available_tools
                            .iter()
                            .map(|t| format!("  - {t}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                };
                let details = None;
                (suggestion, details)
            }
            KopiError::KopiNotFound {
                searched_paths,
                is_auto_install_context,
            } => {
                let suggestion = if cfg!(windows) {
                    Some(
                        "Verify kopi is installed correctly and add it to your PATH environment \
                         variable."
                            .to_string(),
                    )
                } else {
                    Some(
                        "Verify kopi is installed correctly. Add kopi to your PATH: export \
                         PATH=\"$HOME/.kopi/bin:$PATH\""
                            .to_string(),
                    )
                };
                let details = if !searched_paths.is_empty() {
                    Some(format!(
                        "{}Searched in:\n{}",
                        if *is_auto_install_context {
                            "Cannot auto-install JDK. "
                        } else {
                            ""
                        },
                        searched_paths
                            .iter()
                            .map(|p| format!("  - {p}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                } else {
                    None
                };
                (suggestion, details)
            }
            KopiError::ShellDetectionError(msg) => {
                let suggestion = Some(
                    "Specify the shell type explicitly with --shell option (e.g., --shell bash, \
                     --shell powershell)."
                        .to_string(),
                );
                let details = Some(msg.clone());
                (suggestion, details)
            }
            KopiError::ShellNotFound(shell) => {
                let suggestion = Some(format!(
                    "Ensure '{shell}' is installed and available in your PATH."
                ));
                let details = None;
                (suggestion, details)
            }
            KopiError::UnsupportedShell(shell) => {
                let suggestion =
                    Some("Supported shells: bash, zsh, fish, powershell, cmd.".to_string());
                let details = Some(format!("Shell '{shell}' is not supported."));
                (suggestion, details)
            }
            _ => (None, None),
        };

        ErrorContext {
            error,
            suggestion,
            details,
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

impl<'a> fmt::Display for ErrorContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.error)?;

        if let Some(details) = &self.details {
            write!(f, "\n\nDetails: {details}")?;
        }

        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n\nSuggestion: {suggestion}")?;
        }

        Ok(())
    }
}
