use std::fmt;
use thiserror::Error;

pub mod shim;

#[derive(Error, Debug)]
pub enum KopiError {
    #[error("JDK version '{0}' is not available")]
    VersionNotAvailable(String),

    #[error("Invalid version format: {0}")]
    InvalidVersionFormat(String),

    #[error("JDK '{0}' is not installed")]
    JdkNotInstalled(String),

    #[error("Failed to download JDK: {0}")]
    Download(String),

    #[error("Failed to extract archive: {0}")]
    Extract(String),

    #[error("Checksum verification failed")]
    ChecksumMismatch,

    #[error("No JDK configured for current project")]
    NoLocalVersion,

    #[error("Configuration file error: {0}")]
    ConfigFile(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Shell '{0}' is not supported")]
    UnsupportedShell(String),

    #[error("Failed to update PATH: {0}")]
    PathUpdate(String),

    #[error("Failed to create shim: {0}")]
    ShimCreation(String),

    #[error("Failed to fetch metadata: {0}")]
    MetadataFetch(String),

    #[error("Invalid metadata format")]
    InvalidMetadata,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("{0}")]
    AlreadyExists(String),

    #[error("Insufficient disk space: {0}")]
    DiskSpaceError(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] attohttpc::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Nul(#[from] std::ffi::NulError),

    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error("Cache not found")]
    CacheNotFound,
}

pub type Result<T> = std::result::Result<T, KopiError>;

pub struct ErrorContext<'a> {
    pub error: &'a KopiError,
    pub suggestion: Option<String>,
    pub details: Option<String>,
}

impl<'a> ErrorContext<'a> {
    pub fn new(error: &'a KopiError) -> Self {
        let (suggestion, details) = match error {
            KopiError::VersionNotAvailable(msg) => {
                let suggestion = Some("Run 'kopi cache search' to see available versions or 'kopi cache refresh' to update the list.".to_string());
                let details = Some(format!("Version lookup failed: {msg}"));
                (suggestion, details)
            }
            KopiError::InvalidVersionFormat(msg) => {
                let suggestion = Some("Version format should be: '<version>' or '<distribution>@<version>' (e.g., '21' or 'corretto@17').".to_string());
                let details = Some(format!("Invalid format: {msg}"));
                (suggestion, details)
            }
            KopiError::JdkNotInstalled(jdk) => {
                let suggestion = Some(format!("Run 'kopi install {jdk}' to install this JDK."));
                let details = None;
                (suggestion, details)
            }
            KopiError::Download(msg) => {
                let suggestion = Some("Check your internet connection and try again. Use --timeout to increase timeout if needed.".to_string());
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
                let suggestion = Some("Try downloading again. If the problem persists, the file may be corrupted at the source.".to_string());
                let details = Some(
                    "The downloaded file's checksum doesn't match the expected value.".to_string(),
                );
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
                let suggestion = Some("Free up disk space and try again. JDK installations typically require 300-500MB.".to_string());
                let details = Some(format!("Disk space issue: {msg}"));
                (suggestion, details)
            }
            KopiError::NetworkError(msg) => {
                let suggestion = Some("Check your internet connection and proxy settings. Try 'kopi cache refresh' to update metadata.".to_string());
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
                    Some("The requested resource was not found. Try 'kopi cache refresh' to update available versions.".to_string())
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

pub fn format_error_chain(error: &KopiError) -> String {
    let context = ErrorContext::new(error);
    context.to_string()
}

pub fn get_exit_code(error: &KopiError) -> i32 {
    match error {
        KopiError::InvalidVersionFormat(_)
        | KopiError::InvalidConfig(_)
        | KopiError::ValidationError(_) => 2,

        KopiError::PermissionDenied(_) => 13,

        KopiError::NetworkError(_) | KopiError::Http(_) | KopiError::MetadataFetch(_) => 20,

        KopiError::DiskSpaceError(_) => 28,

        KopiError::AlreadyExists(_) => 17,

        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_version_not_available() {
        let error = KopiError::VersionNotAvailable("temurin 22".to_string());
        let context = ErrorContext::new(&error);

        assert!(context.suggestion.is_some());
        assert!(context.suggestion.unwrap().contains("kopi cache search"));
        assert!(context.details.is_some());
    }

    #[test]
    fn test_error_context_permission_denied_unix() {
        let error = KopiError::PermissionDenied("/opt/kopi".to_string());
        let context = ErrorContext::new(&error);

        assert!(context.suggestion.is_some());
        let suggestion = context.suggestion.unwrap();
        if cfg!(unix) {
            assert!(suggestion.contains("sudo"));
        } else {
            assert!(suggestion.contains("Administrator"));
        }
    }

    #[test]
    fn test_error_context_network_error() {
        let error = KopiError::NetworkError("Connection timeout".to_string());
        let context = ErrorContext::new(&error);

        assert!(context.suggestion.is_some());
        assert!(context.suggestion.unwrap().contains("internet connection"));
        assert!(context.details.is_some());
    }

    #[test]
    fn test_error_context_with_custom_suggestion() {
        let error = KopiError::Download("Failed".to_string());
        let context =
            ErrorContext::new(&error).with_suggestion("Try using a different mirror.".to_string());

        assert_eq!(
            context.suggestion,
            Some("Try using a different mirror.".to_string())
        );
    }

    #[test]
    fn test_error_context_display() {
        let error = KopiError::ChecksumMismatch;
        let context = ErrorContext::new(&error);
        let output = context.to_string();

        assert!(output.contains("Error:"));
        assert!(output.contains("Details:"));
        assert!(output.contains("Suggestion:"));
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(
            get_exit_code(&KopiError::InvalidVersionFormat("test".to_string())),
            2
        );
        assert_eq!(
            get_exit_code(&KopiError::PermissionDenied("test".to_string())),
            13
        );
        assert_eq!(
            get_exit_code(&KopiError::NetworkError("test".to_string())),
            20
        );
        assert_eq!(
            get_exit_code(&KopiError::DiskSpaceError("test".to_string())),
            28
        );
        assert_eq!(
            get_exit_code(&KopiError::AlreadyExists("test".to_string())),
            17
        );
        assert_eq!(get_exit_code(&KopiError::Download("test".to_string())), 1);
    }

    #[test]
    fn test_http_error_rate_limit() {
        // Since we can't construct specific attohttpc errors directly,
        // we'll test with a NetworkError that simulates rate limiting
        let error = KopiError::NetworkError("429 Too Many Requests".to_string());
        let context = ErrorContext::new(&error);

        assert!(context.suggestion.is_some());
        // Network errors have a generic suggestion
        assert!(context.suggestion.unwrap().contains("internet connection"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test");
        let error = KopiError::Io(io_err);
        let context = ErrorContext::new(&error);

        assert!(context.suggestion.is_some());
    }

    #[test]
    fn test_format_error_chain() {
        let error = KopiError::InvalidVersionFormat("test".to_string());
        let formatted = format_error_chain(&error);

        assert!(formatted.contains("Error:"));
        assert!(formatted.contains("Invalid version format"));
    }
}
