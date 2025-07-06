#[cfg(test)]
mod auto_install_integration_tests {
    use kopi::config::KopiConfig;
    use kopi::error::KopiError;
    use kopi::models::jdk::VersionRequest;
    use kopi::shim::auto_install::AutoInstaller;
    use kopi::shim::errors::{AutoInstallStatus, ShimError, ShimErrorBuilder};
    use kopi::shim::version_resolver::VersionResolver;
    use serial_test::serial;
    use std::env;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Test helper to set up a clean test environment
    fn setup_test_env() -> (TempDir, KopiConfig) {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Clear any auto-install environment variables
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        (temp_dir, config)
    }

    #[test]
    fn test_missing_jdk_with_auto_install_disabled() {
        let (_temp_dir, mut config) = setup_test_env();

        // Explicitly disable auto-install
        config.auto_install.enabled = false;

        let installer = AutoInstaller::new(&config);
        assert!(!installer.is_enabled());

        let version_request =
            VersionRequest::new("99.0.0".to_string()).with_distribution("nonexistent".to_string());

        let result = installer.auto_install(&version_request);
        assert!(matches!(result, Err(KopiError::JdkNotInstalled(_))));
    }

    #[test]
    fn test_auto_install_enabled_via_config() {
        let (_temp_dir, mut config) = setup_test_env();

        // Enable auto-install in config
        config.auto_install.enabled = true;
        config.save().unwrap();

        let installer = AutoInstaller::new(&config);
        assert!(installer.is_enabled());
    }

    #[test]
    #[serial]
    fn test_auto_install_environment_overrides_config() {
        // Clear environment variables first
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();

        // Set environment variable before creating config
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__ENABLED", "false");
        }

        // Create config which should pick up the environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(!config.auto_install.enabled);

        let installer = AutoInstaller::new(&config);
        assert!(!installer.is_enabled());

        // Cleanup
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }
    }

    #[test]
    fn test_concurrent_install_lock_coordination() {
        let (_temp_dir, config) = setup_test_env();
        let config = Arc::new(config);
        let results = Arc::new(Mutex::new(Vec::new()));

        let mut handles = vec![];

        // Spawn multiple threads trying to acquire the same lock
        for i in 0..3 {
            let config_clone = config.clone();
            let results_clone = results.clone();

            let handle = thread::spawn(move || {
                let installer = AutoInstaller::new(&*config_clone);
                let version_spec = "test-jdk@1.0.0";

                let lock_result = installer.acquire_install_lock(version_spec);
                let mut results = results_clone.lock().unwrap();
                results.push((i, lock_result.is_ok()));

                // Hold the lock briefly
                thread::sleep(Duration::from_millis(50));

                // Release if acquired
                if lock_result.is_ok() {
                    let _ = installer.release_install_lock(version_spec);
                }
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Check that only one thread got the lock
        let results = results.lock().unwrap();
        let successful_locks = results.iter().filter(|(_, success)| *success).count();
        assert_eq!(
            successful_locks, 1,
            "Exactly one thread should acquire the lock"
        );
    }

    #[test]
    fn test_lock_file_cleanup() {
        let (_temp_dir, config) = setup_test_env();
        let installer = AutoInstaller::new(&config);
        let version_spec = "cleanup-test@1.0.0";

        // Acquire lock
        installer.acquire_install_lock(version_spec).unwrap();

        // Verify lock file exists
        let cache_dir = config.cache_dir().unwrap();
        let lock_files: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".kopi-install.lock")
            })
            .collect();
        assert_eq!(lock_files.len(), 1, "Lock file should exist");

        // Release lock
        installer.release_install_lock(version_spec).unwrap();

        // Verify lock file is cleaned up
        let lock_files_after: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".kopi-install.lock")
            })
            .collect();
        assert_eq!(lock_files_after.len(), 0, "Lock file should be cleaned up");
    }

    #[test]
    #[serial]
    fn test_timeout_configuration() {
        // Clear environment variables first
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();

        // Environment variable should override config
        unsafe {
            env::set_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS", "30");
        }

        // Create new config to pick up environment variable
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(config.auto_install.timeout_secs, 30);

        let _installer = AutoInstaller::new(&config);

        // Cleanup
        unsafe {
            env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }
    }

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

    #[test]
    fn test_auto_install_prompt_configuration() {
        let (_temp_dir, mut config) = setup_test_env();

        // Enable auto-install but disable prompts
        config.auto_install.enabled = true;
        config.auto_install.prompt = false;

        let installer = AutoInstaller::new(&config);
        assert!(installer.is_enabled());
        // Note: We can't easily test prompt behavior without mocking stdin
    }

    // This test would require mocking or a test harness for the install command
    #[test]
    #[ignore = "Requires network access and actual JDK installation"]
    fn test_actual_auto_install_flow() {
        // This would test the full auto-install flow with a real JDK
        // Ignored by default as it requires network access
    }
}
