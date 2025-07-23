mod common;

use common::TestHomeGuard;
use kopi::config::KopiConfig;
use kopi::doctor::{CheckCategory, CheckStatus, DiagnosticEngine};
use std::fs;
use std::path::Path;

/// Create a mock JDK installation with basic structure
fn create_mock_jdk(jdks_dir: &Path, name: &str, create_executables: bool) {
    let jdk_path = jdks_dir.join(name);
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    if create_executables {
        // Create mock executables
        for exe in &["java", "javac", "jar", "javadoc"] {
            let exe_name = if cfg!(windows) {
                format!("{}.exe", exe)
            } else {
                exe.to_string()
            };
            let exe_path = bin_dir.join(exe_name);
            fs::write(&exe_path, "#!/bin/sh\necho mock executable").unwrap();

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&exe_path).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&exe_path, perms).unwrap();
            }
        }
    }
}

#[test]
fn test_jdk_checks_no_jdks() {
    let _guard = TestHomeGuard::new("test_jdk_checks_no_jdks");
    let config = KopiConfig::test_default();

    // Ensure jdks directory exists but is empty
    let jdks_dir = config.jdks_dir().unwrap();
    fs::create_dir_all(&jdks_dir).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    // Should have 4 checks: installation, integrity, disk space, version consistency
    assert_eq!(results.len(), 4);

    // Check installation enumeration
    let install_check = &results[0];
    assert_eq!(install_check.name, "JDK Installation Enumeration");
    assert_eq!(install_check.status, CheckStatus::Warning);
    assert!(install_check.message.contains("No JDKs installed"));
    assert!(install_check.suggestion.is_some());

    // Other checks should skip when no JDKs are installed
    for i in 1..4 {
        assert_eq!(results[i].status, CheckStatus::Skip);
    }
}

#[test]
fn test_jdk_checks_with_valid_jdks() {
    let _guard = TestHomeGuard::new("test_jdk_checks_with_valid_jdks");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();
    create_mock_jdk(&jdks_dir, "temurin-21.0.1", true);
    create_mock_jdk(&jdks_dir, "corretto-17.0.9", true);

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    assert_eq!(results.len(), 4);

    // Check installation enumeration
    let install_check = &results[0];
    assert_eq!(install_check.status, CheckStatus::Pass);
    assert!(install_check.message.contains("2 JDKs installed"));
    assert!(install_check.details.is_some());
    let details = install_check.details.as_ref().unwrap();
    assert!(details.contains("temurin-21.0.1"));
    assert!(details.contains("corretto-17.0.9"));

    // Check integrity
    let integrity_check = &results[1];
    assert_eq!(integrity_check.status, CheckStatus::Pass);
    assert!(
        integrity_check
            .message
            .contains("All 2 JDK installations are intact")
    );

    // Check disk space (should pass or warn, not fail)
    let disk_check = &results[2];
    assert!(disk_check.status == CheckStatus::Pass || disk_check.status == CheckStatus::Warning);
    assert!(disk_check.message.contains("JDKs using"));
    assert!(disk_check.details.is_some());
}

#[test]
fn test_jdk_integrity_check_corrupted() {
    let _guard = TestHomeGuard::new("test_jdk_integrity_check_corrupted");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();
    // Create one valid JDK
    create_mock_jdk(&jdks_dir, "temurin-21.0.1", true);

    // Create corrupted JDKs
    // Missing bin directory
    fs::create_dir_all(jdks_dir.join("corretto-17.0.9")).unwrap();

    // Missing executables
    create_mock_jdk(&jdks_dir, "zulu-11.0.21", false);

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    // Find integrity check result
    let integrity_check = results
        .iter()
        .find(|r| r.name == "JDK Installation Integrity")
        .unwrap();

    assert_eq!(integrity_check.status, CheckStatus::Fail);
    assert!(
        integrity_check
            .message
            .contains("2 of 3 JDK installations have issues")
    );
    assert!(integrity_check.details.is_some());
    assert!(integrity_check.suggestion.is_some());

    let details = integrity_check.details.as_ref().unwrap();
    assert!(details.contains("corretto-17.0.9"));
    assert!(details.contains("Missing bin directory"));
    assert!(details.contains("zulu-11.0.21"));
    assert!(details.contains("Missing required executable"));
}

#[test]
fn test_jdk_disk_space_analysis() {
    let _guard = TestHomeGuard::new("test_jdk_disk_space_analysis");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();
    create_mock_jdk(&jdks_dir, "temurin-21.0.1", true);

    // Create some larger files to simulate JDK size
    let lib_dir = jdks_dir.join("temurin-21.0.1").join("lib");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("rt.jar"), vec![0u8; 1024 * 1024]).unwrap(); // 1MB file

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    let disk_check = results
        .iter()
        .find(|r| r.name == "JDK Disk Space Analysis")
        .unwrap();

    assert!(disk_check.status == CheckStatus::Pass || disk_check.status == CheckStatus::Warning);
    assert!(disk_check.message.contains("JDKs using"));
    assert!(disk_check.message.contains("available"));
    assert!(disk_check.details.is_some());

    let details = disk_check.details.as_ref().unwrap();
    assert!(details.contains("temurin-21.0.1"));
}

#[test]
fn test_jdk_checks_performance() {
    let _guard = TestHomeGuard::new("test_jdk_checks_performance");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();
    // Create multiple JDKs to test performance
    for i in 1..=5 {
        create_mock_jdk(&jdks_dir, &format!("temurin-21.0.{}", i), true);
    }

    let start = std::time::Instant::now();
    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));
    let elapsed = start.elapsed();

    // All checks should complete quickly
    assert!(
        elapsed.as_secs() < 5,
        "JDK checks took too long: {:?}",
        elapsed
    );

    // Each individual check should be fast
    for result in &results {
        assert!(
            result.duration.as_millis() < 1000,
            "Check '{}' took too long: {:?}",
            result.name,
            result.duration
        );
    }
}

#[test]
fn test_jdk_version_consistency() {
    let _guard = TestHomeGuard::new("test_jdk_version_consistency");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();
    let jdk_path = jdks_dir.join("temurin-21.0.1");
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create a mock java executable that outputs version info
    let java_exe = if cfg!(windows) { "java.exe" } else { "java" };
    let java_path = bin_dir.join(java_exe);

    #[cfg(unix)]
    {
        let java_script = r#"#!/bin/sh
if [ "$1" = "-version" ]; then
    echo 'openjdk version "21.0.1" 2023-10-17 LTS' >&2
    echo 'OpenJDK Runtime Environment Temurin-21.0.1+12 (build 21.0.1+12-LTS)' >&2
    echo 'OpenJDK 64-Bit Server VM Temurin-21.0.1+12 (build 21.0.1+12-LTS, mixed mode, sharing)' >&2
    exit 0
fi
"#;
        fs::write(&java_path, java_script).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms).unwrap();
    }

    #[cfg(windows)]
    {
        // On Windows, we can't easily create a mock executable that outputs version info
        // So we just create a dummy file and the check will handle it gracefully
        fs::write(&java_path, "mock java.exe").unwrap();
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    let version_check = results
        .iter()
        .find(|r| r.name == "JDK Version Consistency")
        .unwrap();

    // On Unix, it should pass if our mock script works
    // On Windows, it will likely show a warning or handle gracefully
    assert!(
        version_check.status == CheckStatus::Pass
            || version_check.status == CheckStatus::Warning
            || version_check.status == CheckStatus::Skip
    );
}

#[test]
fn test_jdk_checks_with_non_standard_names() {
    let _guard = TestHomeGuard::new("test_jdk_checks_non_standard");
    let config = KopiConfig::test_default();

    let jdks_dir = config.jdks_dir().unwrap();

    // Create JDKs with various naming patterns
    create_mock_jdk(&jdks_dir, "graalvm-ce-21.0.1", true);
    create_mock_jdk(&jdks_dir, "liberica-21.0.1-13", true);
    create_mock_jdk(&jdks_dir, "temurin-22-ea", true);

    // Create invalid directories that should be ignored
    fs::create_dir_all(jdks_dir.join(".tmp")).unwrap();
    fs::create_dir_all(jdks_dir.join("invalid-name")).unwrap();
    fs::write(jdks_dir.join("not-a-directory.txt"), "file").unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Jdks]));

    let install_check = &results[0];
    assert_eq!(install_check.status, CheckStatus::Pass);
    assert!(install_check.message.contains("3 JDKs installed"));

    // Verify the details contain the valid JDKs but not invalid entries
    let details = install_check.details.as_ref().unwrap();
    assert!(details.contains("graalvm-ce-21.0.1"));
    assert!(details.contains("liberica-21.0.1-13"));
    assert!(details.contains("temurin-22-ea"));
    assert!(!details.contains("invalid-name"));
    assert!(!details.contains(".tmp"));
}
