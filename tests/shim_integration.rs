#[cfg(test)]
mod shim_integration_tests {
    use kopi::error::{ErrorContext, KopiError, format_error_with_color};
    use kopi::version::resolver::VersionResolver;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_version_file_search_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root_dir = temp_dir.path();

        // Create nested directory structure
        let project_dir = root_dir.join("workspace").join("project");
        fs::create_dir_all(&project_dir).unwrap();

        // Place version file in workspace directory
        let version_file = root_dir.join("workspace").join(".kopi-version");
        fs::write(&version_file, "temurin@21").unwrap();

        // Resolver should find the version file when starting from project directory
        let resolver = VersionResolver::with_dir(project_dir.clone());
        let result = resolver.resolve_version();

        assert!(result.is_ok());
        let (version_request, _source) = result.unwrap();
        assert_eq!(version_request.version.to_string(), "21");
        assert_eq!(version_request.distribution, Some("temurin".to_string()));
    }

    #[test]
    fn test_no_version_error_paths() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("deep").join("nested").join("project");
        fs::create_dir_all(&project_dir).unwrap();

        let resolver = VersionResolver::with_dir(project_dir.clone());
        let result = resolver.resolve_version();

        // Check that it's a NoLocalVersion error with searched paths
        match result {
            Err(KopiError::NoLocalVersion { searched_paths }) => {
                // Should include the project directory and its parents
                assert!(!searched_paths.is_empty());
                assert!(
                    searched_paths
                        .iter()
                        .any(|p| p.contains(&project_dir.display().to_string()))
                );
            }
            _ => panic!("Expected NoLocalVersion error"),
        }
    }

    #[test]
    fn test_error_formatting() {
        // Test various error scenarios
        let errors = vec![
            KopiError::JdkNotInstalled {
                jdk_spec: "temurin@21".to_string(),
                version: Some("21".to_string()),
                distribution: Some("temurin".to_string()),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            },
            KopiError::JdkNotInstalled {
                jdk_spec: "temurin@21".to_string(),
                version: Some("21".to_string()),
                distribution: Some("temurin".to_string()),
                auto_install_enabled: true,
                auto_install_failed: Some("Network error".to_string()),
                user_declined: false,
                install_in_progress: false,
            },
            KopiError::ToolNotFound {
                tool: "javap".to_string(),
                jdk_path: "/home/user/.kopi/jdks/temurin-21".to_string(),
                available_tools: vec!["java".to_string(), "javac".to_string()],
            },
            KopiError::NoLocalVersion {
                searched_paths: vec![
                    "/home/user/project".to_string(),
                    "/home/user".to_string(),
                    "/home".to_string(),
                ],
            },
            KopiError::KopiNotFound {
                searched_paths: vec!["/home/user/.kopi/bin".to_string(), "PATH".to_string()],
                is_auto_install_context: true,
            },
        ];

        for error in errors {
            // Test ErrorContext
            let context = ErrorContext::new(&error);
            assert!(context.suggestion.is_some(), "Error should have suggestion");

            // Test both colored and non-colored output
            let colored = format_error_with_color(&error, true);
            let plain = format_error_with_color(&error, false);

            assert!(colored.contains("Error:"));
            assert!(plain.contains("Error:"));
            assert!(!plain.contains("\x1b[")); // No color codes in plain output

            if colored.contains("Suggestions:") {
                assert!(colored.contains("\x1b[")); // Should have color codes
            }
        }
    }
}
