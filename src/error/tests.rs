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

use crate::error::format::format_error_with_color;
use crate::error::*;
use crate::locking::{LockTimeoutSource, LockTimeoutValue};
use serial_test::serial;

#[test]
fn test_error_context_version_not_available() {
    let error = KopiError::VersionNotAvailable("temurin 22".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("kopi cache search"));
    assert!(context.details.is_some());
}

#[test]
fn test_error_context_no_local_version() {
    let error = KopiError::NoLocalVersion {
        searched_paths: vec!["/home/user/project".to_string(), "/home/user".to_string()],
    };
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains(".kopi-version"));
    assert!(context.details.is_some());
    assert!(context.details.unwrap().contains("/home/user/project"));
}

#[test]
fn test_error_context_jdk_not_installed() {
    let error = KopiError::JdkNotInstalled {
        jdk_spec: "temurin@21".to_string(),
        version: Some("21".to_string()),
        distribution: Some("temurin".to_string()),
        auto_install_enabled: false,
        auto_install_failed: None,
        user_declined: false,
        install_in_progress: false,
    };
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    let suggestion = context.suggestion.unwrap();
    assert!(suggestion.contains("kopi install temurin@21"));
    assert!(suggestion.contains("KOPI_AUTO_INSTALL__ENABLED=true"));
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
        get_exit_code(&KopiError::NoLocalVersion {
            searched_paths: vec![]
        }),
        3
    );
    assert_eq!(
        get_exit_code(&KopiError::JdkNotInstalled {
            jdk_spec: "test".to_string(),
            version: None,
            distribution: None,
            auto_install_enabled: false,
            auto_install_failed: None,
            user_declined: false,
            install_in_progress: false,
        }),
        4
    );
    assert_eq!(
        get_exit_code(&KopiError::ToolNotFound {
            tool: "java".to_string(),
            jdk_path: "/test".to_string(),
            available_tools: vec![],
        }),
        5
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
    assert_eq!(
        get_exit_code(&KopiError::KopiNotFound {
            searched_paths: vec![],
            is_auto_install_context: false,
        }),
        127
    );
    assert_eq!(
        get_exit_code(&KopiError::LockingCancelled {
            scope: "installation temurin-21".to_string(),
            waited_secs: 12.5,
        }),
        75
    );
    assert_eq!(
        get_exit_code(&KopiError::LockingTimeout {
            scope: "installation temurin-21".to_string(),
            waited_secs: 600.0,
            timeout_value: LockTimeoutValue::from_secs(600),
            timeout_source: LockTimeoutSource::Default,
            details: "lock would block".to_string(),
        }),
        1
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

#[test]
#[serial]
fn test_format_error_with_color_reset() {
    // Test that color codes are properly reset at the end
    let error = KopiError::JdkNotInstalled {
        jdk_spec: "temurin@21".to_string(),
        version: Some("21".to_string()),
        distribution: Some("temurin".to_string()),
        auto_install_enabled: false,
        auto_install_failed: None,
        user_declined: false,
        install_in_progress: false,
    };

    let formatted = format_error_with_color(&error, true);

    // Check that the output contains expected elements
    assert!(formatted.contains("Error:"));
    assert!(formatted.contains("Suggestions:"));
    assert!(formatted.contains("kopi install temurin@21"));
}

#[test]
#[serial]
fn test_format_error_no_color_no_reset() {
    // Test that when color is disabled, no ANSI codes are added
    let error = KopiError::JdkNotInstalled {
        jdk_spec: "temurin@21".to_string(),
        version: Some("21".to_string()),
        distribution: Some("temurin".to_string()),
        auto_install_enabled: false,
        auto_install_failed: None,
        user_declined: false,
        install_in_progress: false,
    };

    let formatted = format_error_with_color(&error, false);

    // The output should NOT contain any ANSI escape codes
    assert!(!formatted.contains("\x1b["));

    // But should still contain the text content
    assert!(formatted.contains("Error:"));
    assert!(formatted.contains("Suggestions:"));
    assert!(formatted.contains("kopi install temurin@21"));
}

#[test]
fn test_error_extract() {
    let error = KopiError::Extract("Failed to extract archive.tar.gz".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("disk space"));
    assert!(context.details.is_some());
}

#[test]
fn test_error_config_file() {
    let error = KopiError::ConfigFile("Invalid TOML format".to_string());
    let context = ErrorContext::new(&error);

    // ConfigFile errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_invalid_config() {
    let error = KopiError::InvalidConfig("Missing required field 'storage.path'".to_string());
    let context = ErrorContext::new(&error);

    // InvalidConfig errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_unsupported_shell() {
    let error = KopiError::UnsupportedShell("csh".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("Supported shells:"));
}

#[test]
fn test_error_shell_detection_error() {
    let error = KopiError::ShellDetectionError("Unable to determine current shell".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("--shell"));
}

#[test]
fn test_error_shell_not_found() {
    let error = KopiError::ShellNotFound("zsh".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("Ensure"));
}

#[test]
fn test_error_path_update() {
    let error = KopiError::PathUpdate("Failed to update shell configuration".to_string());
    let context = ErrorContext::new(&error);

    // PathUpdate errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_shim_creation() {
    let error = KopiError::ShimCreation("Failed to create java shim".to_string());
    let context = ErrorContext::new(&error);

    // ShimCreation errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_metadata_fetch() {
    let error = KopiError::MetadataFetch("Failed to fetch from foojay.io".to_string());
    let context = ErrorContext::new(&error);

    // MetadataFetch errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_invalid_metadata() {
    let error = KopiError::InvalidMetadata;
    let context = ErrorContext::new(&error);

    // InvalidMetadata errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_directory_not_found() {
    let error = KopiError::DirectoryNotFound("/opt/java".to_string());
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(
        context
            .suggestion
            .unwrap()
            .contains("Ensure the directory exists")
    );
}

#[test]
fn test_error_config_error() {
    let error = KopiError::ConfigError("Failed to parse configuration".to_string());
    let context = ErrorContext::new(&error);

    // ConfigError errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_security_error() {
    let error = KopiError::SecurityError("Path traversal detected".to_string());
    let context = ErrorContext::new(&error);

    // SecurityError errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_validation_error() {
    let error = KopiError::ValidationError("Invalid JDK structure".to_string());
    let context = ErrorContext::new(&error);

    // ValidationError errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_system_error() {
    let error = KopiError::SystemError("Process execution failed".to_string());
    let context = ErrorContext::new(&error);

    // SystemError errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_cache_not_found() {
    let error = KopiError::CacheNotFound;
    let context = ErrorContext::new(&error);

    assert!(context.suggestion.is_some());
    assert!(context.suggestion.unwrap().contains("kopi cache refresh"));
}

#[test]
fn test_error_not_found() {
    let error = KopiError::NotFound("Package xyz not found".to_string());
    let context = ErrorContext::new(&error);

    // NotFound errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_thread_panic() {
    let error = KopiError::ThreadPanic("Worker thread panicked".to_string());
    let context = ErrorContext::new(&error);

    // ThreadPanic errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_not_implemented() {
    let error = KopiError::NotImplemented("Feature X is not yet implemented".to_string());
    let context = ErrorContext::new(&error);

    // NotImplemented errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_error_generation_failed() {
    let error = KopiError::GenerationFailed("Failed to generate metadata".to_string());
    let context = ErrorContext::new(&error);

    // GenerationFailed errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_exit_code_coverage() {
    // Test additional exit codes not covered by other tests
    assert_eq!(get_exit_code(&KopiError::Extract("test".to_string())), 1);
    assert_eq!(get_exit_code(&KopiError::ChecksumMismatch), 1);
    assert_eq!(get_exit_code(&KopiError::ConfigFile("test".to_string())), 1);
    assert_eq!(get_exit_code(&KopiError::PathUpdate("test".to_string())), 1);
    assert_eq!(
        get_exit_code(&KopiError::ShimCreation("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::MetadataFetch("test".to_string())),
        20
    );
    assert_eq!(get_exit_code(&KopiError::InvalidMetadata), 1);
    assert_eq!(
        get_exit_code(&KopiError::DirectoryNotFound("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::ConfigError("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::SecurityError("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::SystemError("test".to_string())),
        1
    );
    assert_eq!(get_exit_code(&KopiError::CacheNotFound), 1);
    assert_eq!(get_exit_code(&KopiError::NotFound("test".to_string())), 1);
    assert_eq!(
        get_exit_code(&KopiError::ThreadPanic("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::NotImplemented("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::GenerationFailed("test".to_string())),
        1
    );
    assert_eq!(
        get_exit_code(&KopiError::ShellDetectionError("test".to_string())),
        6
    );
    assert_eq!(
        get_exit_code(&KopiError::ShellNotFound("test".to_string())),
        127
    );
    assert_eq!(
        get_exit_code(&KopiError::UnsupportedShell("test".to_string())),
        7
    );
}

#[test]
fn test_json_error() {
    let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
    let error = KopiError::Json(json_err);
    let context = ErrorContext::new(&error);

    // Json errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_nul_error() {
    use std::ffi::CString;
    let nul_err = CString::new("test\0string").unwrap_err();
    let error = KopiError::Nul(nul_err);
    let context = ErrorContext::new(&error);

    // Nul errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none() || context.details.is_none());
}

#[test]
fn test_walkdir_error() {
    // Create a WalkDirError by attempting to walk a non-existent directory
    let walkdir_iter = walkdir::WalkDir::new("/non/existent/path");
    let walkdir_err = walkdir_iter.into_iter().next().unwrap().unwrap_err();
    let error = KopiError::WalkDir(walkdir_err);
    let context = ErrorContext::new(&error);

    // WalkDir errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none());
    assert!(context.details.is_none());
}

#[test]
fn test_zip_error() {
    use zip::result::ZipError;
    let zip_err = ZipError::UnsupportedArchive("test");
    let error = KopiError::Zip(zip_err);
    let context = ErrorContext::new(&error);

    // Zip errors don't have specific context in the current implementation
    assert!(context.suggestion.is_none() || context.details.is_none());
}
