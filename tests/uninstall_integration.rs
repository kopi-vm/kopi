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
