#[cfg(test)]
mod shim_integration_tests {
    use kopi::error::KopiError;
    use kopi::shim::errors::{AutoInstallStatus, ShimError, ShimErrorBuilder};
    use kopi::shim::version_resolver::VersionResolver;
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
        let version_request = result.unwrap();
        assert_eq!(version_request.version_pattern, "21");
        assert_eq!(version_request.distribution, Some("temurin".to_string()));
    }

    #[test]
    fn test_no_version_error_paths() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("deep").join("nested").join("project");
        fs::create_dir_all(&project_dir).unwrap();

        let resolver = VersionResolver::with_dir(project_dir.clone());
        let result = resolver.resolve_version();

        assert!(matches!(result, Err(KopiError::NoLocalVersion)));

        // Test error formatting
        let error = ShimErrorBuilder::no_version_found(&project_dir.display().to_string());
        let message = error.user_message();

        // Should include searched paths
        assert!(message.contains("Searched in:"));
        assert!(message.contains(&project_dir.display().to_string()));
    }

    #[test]
    fn test_shim_error_formatting() {
        // Test various error scenarios
        let errors = vec![
            ShimError::JdkNotInstalled {
                version: "21".to_string(),
                distribution: "temurin".to_string(),
                auto_install_status: AutoInstallStatus::Disabled,
            },
            ShimError::JdkNotInstalled {
                version: "21".to_string(),
                distribution: "temurin".to_string(),
                auto_install_status: AutoInstallStatus::Failed("Network error".to_string()),
            },
            ShimError::ToolNotFound {
                tool: "javap".to_string(),
                jdk_path: "/home/user/.kopi/jdks/temurin-21".to_string(),
                available_tools: vec!["java".to_string(), "javac".to_string()],
            },
        ];

        for error in errors {
            let message = error.user_message();
            assert!(!message.is_empty());

            let suggestions = error.suggestions();
            assert!(!suggestions.is_empty());

            // Test both colored and non-colored output
            let colored = kopi::shim::errors::format_shim_error(&error, true);
            let plain = kopi::shim::errors::format_shim_error(&error, false);

            assert!(colored.contains("Error:"));
            assert!(plain.contains("Error:"));
            assert!(!plain.contains("\x1b[")); // No color codes in plain output
        }
    }

}
