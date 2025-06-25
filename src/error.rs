use thiserror::Error;

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
    ConfigFile(#[source] std::io::Error),

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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] attohttpc::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, KopiError>;
