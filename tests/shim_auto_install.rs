use kopi::config::KopiConfig;
use kopi::models::version::VersionRequest;
use std::fs;
use tempfile::TempDir;

mod common;
use common::TestHomeGuard;

/// Create a test configuration with auto-install settings
fn create_test_config_with_auto_install(enabled: bool, prompt: bool) -> KopiConfig {
    let test_home = TestHomeGuard::new();
    let mut config = KopiConfig::new(test_home.path().to_path_buf()).unwrap();

    config.auto_install.enabled = enabled;
    config.auto_install.prompt = prompt;
    config.auto_install.timeout_secs = 5; // Short timeout for tests

    config
}

#[test]
fn test_auto_install_configuration() {
    // Test with auto-install enabled
    let config = create_test_config_with_auto_install(true, false);
    assert!(config.auto_install.enabled);
    assert!(!config.auto_install.prompt);
    assert_eq!(config.auto_install.timeout_secs, 5);

    // Test with auto-install disabled
    let config = create_test_config_with_auto_install(false, true);
    assert!(!config.auto_install.enabled);
    assert!(config.auto_install.prompt);
}

#[test]
fn test_shims_config_defaults() {
    // Clear any environment variables that might affect shims config
    unsafe {
        std::env::remove_var("KOPI_SHIMS__AUTO_CREATE_SHIMS");
        std::env::remove_var("KOPI_SHIMS__AUTO_INSTALL");
        std::env::remove_var("KOPI_SHIMS__INSTALL_TIMEOUT");
    }

    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

    // Check ShimsConfig defaults
    assert!(config.shims.auto_create_shims);
    assert!(config.shims.additional_tools.is_empty());
    assert!(config.shims.exclude_tools.is_empty());
    assert!(!config.shims.auto_install); // Should default to false
    assert!(config.shims.auto_install_prompt);
    assert_eq!(config.shims.install_timeout, 600);
}

#[test]
fn test_shims_config_from_toml() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();
    let config_path = kopi_home.join("config.toml");

    // Write a config file with custom shims settings
    let config_content = r#"
[shims]
auto_create_shims = false
additional_tools = ["custom-tool"]
exclude_tools = ["javah"]
auto_install = true
auto_install_prompt = false
install_timeout = 300
"#;

    fs::write(&config_path, config_content).unwrap();

    // Load config
    let config = KopiConfig::new(kopi_home).unwrap();

    // Verify settings were loaded
    assert!(!config.shims.auto_create_shims);
    assert_eq!(config.shims.additional_tools, vec!["custom-tool"]);
    assert_eq!(config.shims.exclude_tools, vec!["javah"]);
    assert!(config.shims.auto_install);
    assert!(!config.shims.auto_install_prompt);
    assert_eq!(config.shims.install_timeout, 300);
}

#[test]
fn test_version_request_formatting() {
    // Test version request with distribution
    let version_request =
        VersionRequest::new("21".to_string()).with_distribution("temurin".to_string());

    // Verify the version request is created correctly
    assert_eq!(version_request.version_pattern, "21");
    assert_eq!(version_request.distribution, Some("temurin".to_string()));
}

#[test]
fn test_shims_config_with_custom_tools() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();
    let config_path = kopi_home.join("config.toml");

    // Write a config with custom tools configuration
    let config_content = r#"
[shims]
additional_tools = ["custom-javac", "my-tool"]
exclude_tools = ["javah", "jhat"]
"#;

    fs::write(&config_path, config_content).unwrap();
    let config = KopiConfig::new(kopi_home).unwrap();

    // Verify custom tools configuration
    assert_eq!(config.shims.additional_tools.len(), 2);
    assert!(
        config
            .shims
            .additional_tools
            .contains(&"custom-javac".to_string())
    );
    assert!(
        config
            .shims
            .additional_tools
            .contains(&"my-tool".to_string())
    );

    assert_eq!(config.shims.exclude_tools.len(), 2);
    assert!(config.shims.exclude_tools.contains(&"javah".to_string()));
    assert!(config.shims.exclude_tools.contains(&"jhat".to_string()));
}

#[test]
fn test_auto_install_timeout_configuration() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();
    let config_path = kopi_home.join("config.toml");

    // Test different timeout configurations
    let config_content = r#"
[auto_install]
timeout_secs = 60

[shims]
install_timeout = 120
"#;

    fs::write(&config_path, config_content).unwrap();
    let config = KopiConfig::new(kopi_home).unwrap();

    // Auto-install timeout should be 60
    assert_eq!(config.auto_install.timeout_secs, 60);

    // Shims install timeout should be 120
    assert_eq!(config.shims.install_timeout, 120);
}

#[test]
fn test_environment_variable_overrides() {
    // Set environment variables
    unsafe {
        std::env::set_var("KOPI_SHIMS__AUTO_CREATE_SHIMS", "false");
        std::env::set_var("KOPI_SHIMS__AUTO_INSTALL", "true");
        std::env::set_var("KOPI_SHIMS__INSTALL_TIMEOUT", "900");
    }

    let test_home = TestHomeGuard::new();
    let config = KopiConfig::new(test_home.path().to_path_buf()).unwrap();

    // Verify environment variables override defaults
    assert!(!config.shims.auto_create_shims);
    assert!(config.shims.auto_install);
    assert_eq!(config.shims.install_timeout, 900);

    // Clean up
    unsafe {
        std::env::remove_var("KOPI_SHIMS__AUTO_CREATE_SHIMS");
        std::env::remove_var("KOPI_SHIMS__AUTO_INSTALL");
        std::env::remove_var("KOPI_SHIMS__INSTALL_TIMEOUT");
    }
}

#[test]
fn test_combined_config_settings() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();
    let config_path = kopi_home.join("config.toml");

    // Test a comprehensive configuration
    let config_content = r#"
[auto_install]
enabled = true
prompt = false
timeout_secs = 120

[shims]
auto_create_shims = true
additional_tools = ["graalvm-tool"]
exclude_tools = ["old-tool"]
auto_install = true
auto_install_prompt = false
install_timeout = 300
"#;

    fs::write(&config_path, config_content).unwrap();
    let config = KopiConfig::new(kopi_home).unwrap();

    // Verify all settings
    assert!(config.auto_install.enabled);
    assert!(!config.auto_install.prompt);
    assert_eq!(config.auto_install.timeout_secs, 120);

    assert!(config.shims.auto_create_shims);
    assert_eq!(config.shims.additional_tools, vec!["graalvm-tool"]);
    assert_eq!(config.shims.exclude_tools, vec!["old-tool"]);
    assert!(config.shims.auto_install);
    assert!(!config.shims.auto_install_prompt);
    assert_eq!(config.shims.install_timeout, 300);
}

#[test]
fn test_config_partial_settings() {
    // Clear any environment variables that might affect shims config
    unsafe {
        std::env::remove_var("KOPI_SHIMS__AUTO_CREATE_SHIMS");
        std::env::remove_var("KOPI_SHIMS__AUTO_INSTALL");
        std::env::remove_var("KOPI_SHIMS__INSTALL_TIMEOUT");
    }

    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();
    let config_path = kopi_home.join("config.toml");

    // Test with only some settings specified
    let config_content = r#"
[shims]
additional_tools = ["my-tool"]
install_timeout = 180
"#;

    fs::write(&config_path, config_content).unwrap();
    let config = KopiConfig::new(kopi_home).unwrap();

    // Verify defaults are used for unspecified settings
    assert!(config.shims.auto_create_shims); // Should use default (true)
    assert_eq!(config.shims.additional_tools, vec!["my-tool"]);
    assert!(config.shims.exclude_tools.is_empty()); // Should use default (empty)
    assert!(!config.shims.auto_install); // Should use default (false)
    assert!(config.shims.auto_install_prompt); // Should use default (true)
    assert_eq!(config.shims.install_timeout, 180);
}
