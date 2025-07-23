mod common;

use common::TestHomeGuard;
use kopi::commands::doctor::DoctorCommand;
use kopi::config::KopiConfig;
use kopi::doctor::checks::{
    ConfigFileCheck, DirectoryPermissionsCheck, InstallationDirectoryCheck, KopiBinaryCheck,
    OwnershipCheck, ShimsInPathCheck, VersionCheck,
};
use kopi::doctor::{CheckCategory, CheckStatus, DiagnosticCheck};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn test_installation_directory_check_with_structure() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();

    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }
    let config = kopi::config::new_kopi_config().unwrap();

    let check = InstallationDirectoryCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Installation);

    assert_eq!(result.status, CheckStatus::Pass);
    assert!(
        result
            .message
            .contains("Installation directory structure is valid")
    );

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
fn test_installation_directory_check_missing_subdirs() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();

    unsafe {
        env::set_var("KOPI_HOME", &kopi_home);
    }
    let config = kopi::config::new_kopi_config().unwrap();

    let check = InstallationDirectoryCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Installation);

    // Should warn about missing subdirectories
    assert_eq!(result.status, CheckStatus::Warning);
    assert!(result.message.contains("Missing subdirectories"));
    assert!(result.suggestion.is_some());

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
fn test_config_file_check_valid() {
    let test_home = TestHomeGuard::new();
    let kopi_home = test_home.kopi_home();
    fs::create_dir_all(&kopi_home).unwrap();

    // Create a valid config file
    let config_content = r#"
[network]
timeout = 30

[cache]
max_age_days = 30
"#;
    fs::write(kopi_home.join("config.toml"), config_content).unwrap();

    unsafe {
        env::set_var("KOPI_HOME", &kopi_home);
    }
    let config = kopi::config::new_kopi_config().unwrap();

    let check = ConfigFileCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Installation);

    assert_eq!(result.status, CheckStatus::Pass);
    assert!(result.message.contains("Config file is valid"));

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
fn test_shims_in_path_check_integration() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();

    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }
    let config = kopi::config::new_kopi_config().unwrap();

    // Get shims directory
    let shims_dir = config.shims_dir().unwrap();

    // Save original PATH
    let original_path = env::var("PATH").unwrap_or_default();

    // Test without shims in PATH
    unsafe {
        env::set_var("PATH", "/usr/bin:/bin");
    }
    let check = ShimsInPathCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Installation);
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("not found in PATH"));

    // Test with shims in PATH
    unsafe {
        env::set_var("PATH", format!("{}:/usr/bin", shims_dir.display()));
    }
    let check = ShimsInPathCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Installation);
    assert_eq!(result.status, CheckStatus::Pass);

    // Restore environment
    unsafe {
        env::set_var("PATH", original_path);
    }
    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
fn test_directory_permissions_check() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();

    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }
    let config = kopi::config::new_kopi_config().unwrap();

    let check = DirectoryPermissionsCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Permissions);

    // Should pass since we just created the directories
    assert_eq!(result.status, CheckStatus::Pass);
    assert!(result.message.contains("proper write permissions"));

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
#[cfg(unix)]
fn test_ownership_check_integration() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();

    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }
    let config = kopi::config::new_kopi_config().unwrap();

    let check = OwnershipCheck::new(&config);
    let start = Instant::now();
    let result = check.run(start, CheckCategory::Permissions);

    // Should pass since we own the directories we just created
    assert_eq!(result.status, CheckStatus::Pass);
    assert!(result.message.contains("ownership is consistent"));

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}

#[test]
fn test_doctor_command_full_execution() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();

    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }
    let config = kopi::config::new_kopi_config().unwrap();

    // This test would normally call execute() but it calls process::exit
    // So we test the individual components instead
    let doctor = DoctorCommand::new(&config).unwrap();

    // Test category filtering
    assert!(doctor.execute(false, false, Some("invalid")).is_err());

    unsafe {
        env::remove_var("KOPI_HOME");
    }
}
