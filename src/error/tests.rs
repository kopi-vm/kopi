use crate::error::format::format_error_with_color;
use crate::error::*;

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

    // The output should end with a reset code
    assert!(formatted.ends_with("\x1b[0m"));

    // Check that the output contains expected elements
    assert!(formatted.contains("Error:"));
    assert!(formatted.contains("Suggestions:"));
    assert!(formatted.contains("kopi install temurin@21"));
}

#[test]
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
