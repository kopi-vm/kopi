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

mod context;
mod exit_codes;
mod format;
#[cfg(test)]
mod tests;

pub use context::ErrorContext;
pub use exit_codes::get_exit_code;
pub use format::{format_error_chain, format_error_with_color};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KopiError {
    #[error("JDK version '{0}' is not available")]
    VersionNotAvailable(String),

    #[error("Invalid version format: {0}")]
    InvalidVersionFormat(String),

    #[error("JDK '{jdk_spec}' is not installed")]
    JdkNotInstalled {
        jdk_spec: String,
        version: Option<String>,
        distribution: Option<String>,
        auto_install_enabled: bool,
        auto_install_failed: Option<String>,
        user_declined: bool,
        install_in_progress: bool,
    },

    #[error("Failed to download JDK: {0}")]
    Download(String),

    #[error("Failed to extract archive: {0}")]
    Extract(String),

    #[error("Checksum verification failed")]
    ChecksumMismatch,

    #[error("No JDK configured for current project")]
    NoLocalVersion { searched_paths: Vec<String> },

    #[error("Configuration file error: {0}")]
    ConfigFile(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Shell '{0}' is not supported")]
    UnsupportedShell(String),

    #[error("Shell detection error: {0}")]
    ShellDetectionError(String),

    #[error("Shell '{0}' not found in PATH")]
    ShellNotFound(String),

    #[error("Failed to update PATH: {0}")]
    PathUpdate(String),

    #[error("Failed to create shim: {0}")]
    ShimCreation(String),

    #[error("Tool '{tool}' not found in JDK at {jdk_path}")]
    ToolNotFound {
        tool: String,
        jdk_path: String,
        available_tools: Vec<String>,
    },

    #[error("Kopi binary not found")]
    KopiNotFound {
        searched_paths: Vec<String>,
        is_auto_install_context: bool,
    },

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

    #[error("Failed to acquire {scope} lock: {details}")]
    LockingAcquire { scope: String, details: String },

    #[error("Failed to acquire {scope} lock within {waited_secs:.2}s (last error: {details})")]
    LockingTimeout {
        scope: String,
        waited_secs: f64,
        details: String,
    },

    #[error("Lock acquisition for {scope} was cancelled after {waited_secs:.2}s")]
    LockingCancelled { scope: String, waited_secs: f64 },

    #[error("Failed to release {scope} lock: {details}")]
    LockingRelease { scope: String, details: String },

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

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Thread panic: {0}")]
    ThreadPanic(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Metadata generation failed: {0}")]
    GenerationFailed(String),
}

pub type Result<T> = std::result::Result<T, KopiError>;
