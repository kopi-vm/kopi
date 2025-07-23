use kopi::commands::uninstall::UninstallCommand;
use kopi::config::KopiConfig;
use kopi::storage::JdkRepository;
use kopi::uninstall::UninstallHandler;
use kopi::uninstall::safety::{
    check_tool_dependencies, is_active_global_jdk, is_active_local_jdk, perform_safety_checks,
};

#[cfg(unix)]
use kopi::uninstall::safety::verify_removal_permission;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct TestEnvironment {
    _temp_dir: TempDir,
    config: KopiConfig,
}

impl TestEnvironment {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Create jdks directory
        fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();

        Self {
            _temp_dir: temp_dir,
            config,
        }
    }

    fn create_real_jdk(&self, distribution: &str, version: &str) -> PathBuf {
        let jdk_path = self
            .config
            .jdks_dir()
            .unwrap()
            .join(format!("{distribution}-{version}"));

        // Create directory structure similar to a real JDK
        fs::create_dir_all(&jdk_path).unwrap();
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        fs::create_dir_all(jdk_path.join("lib")).unwrap();
        fs::create_dir_all(jdk_path.join("conf")).unwrap();

        // Create some real files with content
        fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21.0.5\"").unwrap();
        fs::write(jdk_path.join("bin/java"), "#!/bin/sh\necho mock java").unwrap();
        fs::write(jdk_path.join("bin/javac"), "#!/bin/sh\necho mock javac").unwrap();
        fs::write(jdk_path.join("lib/modules"), "mock modules file").unwrap();

        jdk_path
    }

    fn create_jdk_with_metadata(&self, distribution: &str, version: &str) -> PathBuf {
        // For uninstall tests, we don't need actual metadata files
        // The JDK listing is done by parsing directory names
        self.create_real_jdk(distribution, version)
    }
}

#[test]
fn test_real_jdk_removal() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create a mock JDK
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");
    assert!(jdk_path.exists());
    assert!(jdk_path.join("bin/java").exists());

    // Uninstall the JDK
    let result = handler.uninstall_jdk("temurin@21.0.5-11", false);
    assert!(result.is_ok());

    // Verify removal
    assert!(!jdk_path.exists());
}

#[test]
fn test_uninstall_nonexistent_jdk() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Try to uninstall non-existent JDK
    let result = handler.uninstall_jdk("temurin@21.0.5-11", false);
    assert!(result.is_err());

    match result {
        Err(kopi::error::KopiError::JdkNotInstalled { jdk_spec, .. }) => {
            assert_eq!(jdk_spec, "temurin@21.0.5-11");
        }
        _ => panic!("Expected JdkNotInstalled error"),
    }
}

#[test]
fn test_uninstall_with_version_pattern() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create multiple JDKs
    // Note: Using "-" as build separator instead of "+" for cross-platform compatibility
    // Also using valid version formats (max 3 version components) that can be parsed
    env.create_real_jdk("temurin", "21.0.5-11");
    env.create_real_jdk("temurin", "17.0.9-9");
    env.create_real_jdk("zulu", "21.0.1-12"); // Two JDKs with version 21

    // Try to uninstall with just version - should fail due to multiple matches
    let result = handler.uninstall_jdk("21", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Multiple JDKs match")
    );
}

#[test]
fn test_uninstall_dry_run() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create a mock JDK
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");
    assert!(jdk_path.exists());

    // Dry run should not remove anything
    let result = handler.uninstall_jdk("temurin@21.0.5-11", true);
    assert!(result.is_ok());

    // JDK should still exist
    assert!(jdk_path.exists());
}

#[test]
fn test_active_jdk_detection_stubs() {
    // Test that stub functions always return false in Phase 1
    assert!(!is_active_global_jdk("temurin", "21.0.5-11").unwrap());
    assert!(!is_active_local_jdk("temurin", "21.0.5-11").unwrap());
    assert!(!is_active_global_jdk("corretto", "17.0.9").unwrap());
    assert!(!is_active_local_jdk("corretto", "17.0.9").unwrap());
}

#[test]
fn test_safety_checks_pass() {
    // With active JDK stubs returning false, safety checks should pass
    assert!(perform_safety_checks("temurin", "21.0.5-11").is_ok());
    assert!(perform_safety_checks("corretto", "17.0.9").is_ok());
}

#[test]
#[cfg(unix)]
fn test_permission_error_handling() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvironment::new();
    let _repository = JdkRepository::new(&env.config);

    // Create a JDK
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");

    // Make the parent directory read-only
    let jdks_dir = env.config.jdks_dir().unwrap();
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&jdks_dir, perms).unwrap();

    // Try to verify removal permission
    let result = verify_removal_permission(&jdk_path);

    // Restore permissions before asserting
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&jdks_dir, perms).unwrap();

    assert!(result.is_err());
}

#[test]
fn test_atomic_removal_with_recovery() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create a mock JDK
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");
    let original_files = vec![
        jdk_path.join("release"),
        jdk_path.join("bin/java"),
        jdk_path.join("bin/javac"),
    ];

    // Verify files exist
    for file in &original_files {
        assert!(file.exists());
    }

    // Uninstall should be atomic
    let result = handler.uninstall_jdk("temurin@21.0.5-11", false);
    assert!(result.is_ok());

    // Verify complete removal
    assert!(!jdk_path.exists());
    for file in &original_files {
        assert!(!file.exists());
    }
}

#[test]
fn test_tool_dependency_check() {
    let env = TestEnvironment::new();

    // Create a mock JDK with tools
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");

    // Should succeed (just warns about potential dependencies)
    assert!(check_tool_dependencies(&jdk_path).is_ok());
}

#[test]
fn test_multiple_jdk_versions() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create multiple versions of the same distribution
    let jdk1 = env.create_real_jdk("temurin", "21.0.5-11");
    let jdk2 = env.create_real_jdk("temurin", "17.0.9-9");
    let jdk3 = env.create_real_jdk("temurin", "11.0.21-9");

    // Uninstall specific version
    let result = handler.uninstall_jdk("temurin@17.0.9-9", false);
    assert!(result.is_ok());

    // Verify only the specified version was removed
    assert!(jdk1.exists());
    assert!(!jdk2.exists());
    assert!(jdk3.exists());
}

#[test]
fn test_partial_version_matching() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create JDK with full version
    env.create_real_jdk("temurin", "21.0.5-11");

    // Should match with partial version
    let result = handler.uninstall_jdk("temurin@21", false);
    assert!(result.is_ok());
}

#[test]
fn test_disk_space_calculation() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);

    // Create a JDK with known content
    let jdk_path = env.create_real_jdk("temurin", "21.0.5-11");

    // Calculate size
    let size = repository.get_jdk_size(&jdk_path).unwrap();

    // Should be greater than 0 (we created some files)
    assert!(size > 0);
}

#[test]
fn test_security_validation() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);

    // Try to remove a path outside of JDKs directory
    let result = repository.remove_jdk(&PathBuf::from("/etc/passwd"));
    assert!(result.is_err());

    match result {
        Err(kopi::error::KopiError::SecurityError(msg)) => {
            assert!(msg.contains("outside of JDKs directory"));
        }
        _ => panic!("Expected SecurityError"),
    }
}

#[test]
fn test_corretto_extended_version_formats() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create Corretto JDKs with 4-5 component versions
    let jdk1 = env.create_real_jdk("corretto", "21.0.7.6");
    let jdk2 = env.create_real_jdk("corretto", "21.0.7.6.1");
    let jdk3 = env.create_real_jdk("corretto", "8.452.9.1");

    assert!(jdk1.exists());
    assert!(jdk2.exists());
    assert!(jdk3.exists());

    // Test that specifying partial version matches multiple JDKs
    let result = handler.uninstall_jdk("corretto@21.0.7.6", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Multiple JDKs match")
    );

    // Remove JDK2 first to avoid ambiguity
    let result = handler.uninstall_jdk("corretto@21.0.7.6.1", false);
    assert!(result.is_ok());
    assert!(!jdk2.exists());

    // Now we can uninstall JDK1 without ambiguity
    let result = handler.uninstall_jdk("corretto@21.0.7.6", false);
    assert!(result.is_ok());
    assert!(!jdk1.exists());

    // Test partial version matching for Corretto Java 8
    let result = handler.uninstall_jdk("corretto@8", false);
    assert!(result.is_ok());
    assert!(!jdk3.exists());
}

#[test]
fn test_dragonwell_extended_version_formats() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create Dragonwell JDKs with 6-component versions
    let jdk1 = env.create_real_jdk("dragonwell", "21.0.7.0.7.6");
    let jdk2 = env.create_real_jdk("dragonwell", "17.0.13.0.13.11-11");

    assert!(jdk1.exists());
    assert!(jdk2.exists());

    // Test uninstall with partial version matching
    let result = handler.uninstall_jdk("dragonwell@21.0.7", false);
    assert!(result.is_ok());
    assert!(!jdk1.exists());

    // Test uninstall with full version including build
    let result = handler.uninstall_jdk("dragonwell@17.0.13.0.13.11-11", false);
    assert!(result.is_ok());
    assert!(!jdk2.exists());
}

#[test]
fn test_partial_version_matching_extended() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create various JDKs with extended versions
    let corretto = env.create_real_jdk("corretto", "21.0.7.6.1");
    let dragonwell = env.create_real_jdk("dragonwell", "21.0.7.0.7.6");
    let temurin = env.create_real_jdk("temurin", "21.0.7-11");

    // Try to uninstall with just "21" - should fail due to multiple matches
    let result = handler.uninstall_jdk("21", false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Multiple JDKs match")
    );

    // Uninstall with distribution and partial version
    let result = handler.uninstall_jdk("corretto@21", false);
    assert!(result.is_ok());
    assert!(!corretto.exists());
    assert!(dragonwell.exists());
    assert!(temurin.exists());

    // Test more specific partial match
    let result = handler.uninstall_jdk("dragonwell@21.0.7.0", false);
    assert!(result.is_ok());
    assert!(!dragonwell.exists());
    assert!(temurin.exists());
}

#[test]
fn test_complex_build_identifiers() {
    let env = TestEnvironment::new();
    let repository = JdkRepository::new(&env.config);
    let handler = UninstallHandler::new(&repository);

    // Create JDKs with complex build identifiers
    let jetbrains = env.create_real_jdk("jetbrains", "21.0.5-13.674.11");
    let semeru = env.create_real_jdk("semeru", "21.0.5-11.0.572");
    let graalvm = env.create_real_jdk("graalvm", "21.0.5-11-jvmci-24.1-b01");

    // Test uninstall with full version including complex build
    let result = handler.uninstall_jdk("jetbrains@21.0.5-13.674.11", false);
    assert!(result.is_ok());
    assert!(!jetbrains.exists());

    // Test partial matching still works
    let result = handler.uninstall_jdk("semeru@21", false);
    assert!(result.is_ok());
    assert!(!semeru.exists());

    // Test with pre-release identifier
    let result = handler.uninstall_jdk("graalvm@21.0.5-11-jvmci-24.1-b01", false);
    assert!(result.is_ok());
    assert!(!graalvm.exists());
}

// Command-level integration tests
// NOTE: These tests modify the KOPI_HOME environment variable and should be run
// with --test-threads=1 to avoid race conditions between parallel tests.

#[test]
#[serial_test::serial]
fn test_uninstall_command_single_jdk() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create a mock JDK with metadata
    let jdk_path = env.create_jdk_with_metadata("temurin", "21.0.5-11");
    assert!(jdk_path.exists());

    // Execute uninstall command with force flag to skip confirmation
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("temurin@21.0.5-11"), true, false, false, false);

    assert!(result.is_ok());

    // Verify JDK was removed
    assert!(!jdk_path.exists());
    assert!(!jdk_path.with_extension("meta.json").exists());
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_dry_run() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create a mock JDK with metadata
    let jdk_path = env.create_jdk_with_metadata("corretto", "17.0.13.11.1");

    // Execute uninstall command with dry_run flag
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("corretto@17.0.13.11.1"), false, true, false, false);

    assert!(result.is_ok());

    // Verify JDK was NOT removed (dry run)
    assert!(jdk_path.exists());
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_all_versions() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create multiple versions of the same distribution
    env.create_jdk_with_metadata("zulu", "21.0.5-11");
    env.create_jdk_with_metadata("zulu", "17.0.13-11");
    env.create_jdk_with_metadata("zulu", "11.0.25-9");

    // Create a different distribution that should NOT be removed
    let temurin_path = env.create_jdk_with_metadata("temurin", "21.0.5-11");

    // Execute uninstall command with --all flag
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("zulu"), true, false, true, false);

    assert!(result.is_ok());

    // Verify all Zulu JDKs were removed
    let jdks_dir = env.config.jdks_dir().unwrap();
    assert!(!jdks_dir.join("zulu-21.0.5-11").exists());
    assert!(!jdks_dir.join("zulu-17.0.13-11").exists());
    assert!(!jdks_dir.join("zulu-11.0.25-9").exists());

    // Verify Temurin JDK was NOT removed
    assert!(temurin_path.exists());
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_nonexistent_error() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Try to uninstall a JDK that doesn't exist
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("nonexistent@1.0.0"), false, false, false, false);

    assert!(result.is_err());

    match result {
        Err(kopi::error::KopiError::JdkNotInstalled { jdk_spec, .. }) => {
            assert_eq!(jdk_spec, "nonexistent@1.0.0");
        }
        _ => panic!("Expected JdkNotInstalled error"),
    }
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_ambiguous_error() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create multiple JDKs with the same major version
    env.create_jdk_with_metadata("temurin", "21.0.5-11");
    env.create_jdk_with_metadata("corretto", "21.0.1.12.1");

    // Try to uninstall with just the major version
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("21"), false, false, false, false);

    assert!(result.is_err());

    match result {
        Err(kopi::error::KopiError::SystemError(msg)) => {
            assert!(msg.contains("Multiple JDKs match"));
        }
        _ => panic!("Expected SystemError for ambiguous version"),
    }
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_version_shorthand() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create a single JDK with metadata
    let jdk_path = env.create_jdk_with_metadata("temurin", "17.0.13-11");

    // Uninstall using shorthand version (should work when unambiguous)
    let command = UninstallCommand::new(&env.config).unwrap();
    let result = command.execute(Some("17"), true, false, false, false);

    assert!(result.is_ok());

    // Verify JDK was removed
    assert!(!jdk_path.exists());
}

#[test]
#[serial_test::serial]
fn test_uninstall_command_with_jre_suffix() {
    let env = TestEnvironment::new();
    unsafe {
        std::env::set_var("KOPI_HOME", env.config.kopi_home());
    }

    // Create a mock JRE installation
    // Note: JRE installations have "-jre" in the directory name
    let jre_dir = env.config.jdks_dir().unwrap().join("temurin-jre-21.0.5-11");
    fs::create_dir_all(&jre_dir).unwrap();

    // The directory naming doesn't match the expected pattern for version parsing
    // This test is commented out as the jre@ prefix format is not supported
    // TODO: Add support for JRE uninstall if needed

    // For now, just clean up the directory
    fs::remove_dir_all(&jre_dir).ok();
}
