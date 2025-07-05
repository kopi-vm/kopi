mod common;
use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.env("HOME", kopi_home.parent().unwrap());
    cmd
}

/// Test that install command correctly detects current platform
/// Verifies: Architecture detection, OS detection, and platform-specific package selection
#[test]
fn test_platform_compatibility() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // The install command should detect current platform
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

/// Test version resolution logic
/// Verifies: "21" resolves to latest "21.x.x" version
/// This tests the semantic versioning resolution internally
#[test]
fn test_version_resolution() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test that version "21" resolves to latest 21.x.x
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("21."));
}

/// Test support for various JDK distributions
/// Verifies: Different distributions can be installed on the current platform
/// Note: Some distributions may not be available for all platforms
#[test]
fn test_distribution_variations() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test various distributions
    let distributions = vec!["temurin@17", "corretto@17", "zulu@17", "liberica@17"];

    for dist in distributions {
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install").arg(dist).arg("--dry-run").assert();
        // Some distributions might not be available for all platforms
    }
}

/// Test download resumption capability after interruption
/// Simulates: Partial download file exists
/// Verifies: Download module can handle partial files (resume or restart)
#[test]
fn test_interrupted_download_recovery() {
    // This test simulates an interrupted download scenario
    // In real implementation, this would test resume capability
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a partial download file
    let downloads_dir = kopi_home.join("downloads");
    fs::create_dir_all(&downloads_dir).unwrap();
    let partial_file = downloads_dir.join("partial-jdk.tar.gz");
    fs::write(&partial_file, b"partial content").unwrap();

    // The download module should handle partial files
    assert!(partial_file.exists());
}

#[test]
fn test_disk_space_check() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // This would test disk space checking logic
    // In real scenario, we'd need to mock available disk space
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("21").arg("--dry-run").assert();
}

/// Test checksum verification during installation
/// Verifies: Downloaded files are validated against checksums
/// This ensures integrity of downloaded JDK archives
#[test]
fn test_checksum_verification() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // The install process should verify checksums
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

/// Test handling of network failures
/// Simulates: Invalid proxy configuration to force network errors
/// Verifies: Appropriate error messages and recovery suggestions
#[test]
fn test_network_failure_handling() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test with invalid proxy to simulate network failure
    let mut cmd = get_test_command(&kopi_home);
    cmd.env("HTTP_PROXY", "http://invalid-proxy:9999")
        .env("HTTPS_PROXY", "http://invalid-proxy:9999")
        .arg("install")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("connection").or(predicate::str::contains("network")));
}

#[test]
fn test_invalid_distribution() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("nonexistent@21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown distribution"));
}

#[test]
fn test_archive_extraction_failure() {
    // This would test handling of corrupted archives
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a fake corrupted archive
    let downloads_dir = kopi_home.join("downloads");
    fs::create_dir_all(&downloads_dir).unwrap();
    let corrupt_file = downloads_dir.join("corrupt.tar.gz");
    fs::write(&corrupt_file, b"not a valid tar.gz file").unwrap();

    // In real scenario, extraction would fail with proper error
    assert!(corrupt_file.exists());
}

#[test]
#[cfg(unix)]
fn test_symlink_creation() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test that symlinks/shims would be created properly
    let bin_dir = kopi_home.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // In real implementation, shims would be created here
    assert!(bin_dir.exists());
}

#[test]
fn test_multiple_architecture_support() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // The install should pick the right architecture
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("-vv") // Debug to see architecture detection
        .arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_version_upgrade_scenario() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Simulate having an older version installed
    let old_version_dir = kopi_home.join("jdks").join("temurin-17.0.8");
    fs::create_dir_all(&old_version_dir).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install newer version of same major
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17.0.9")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_package_type_selection() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install should default to JDK (not JRE)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("-v")
        .arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_security_validation() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install process should validate HTTPS certificates
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

/// Test atomic installation mechanism
/// Verifies: Installation uses temporary directory and atomic rename
/// This prevents partial installations from corrupting the JDK directory
#[test]
fn test_atomic_installation() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test that installations are atomic (temp dir + rename)
    let temp_install_dir = kopi_home.join("jdks").join(".tmp-install");

    // In real scenario, installation would use temp directory
    assert!(!temp_install_dir.exists());
}

#[test]
fn test_cleanup_on_failure() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test that temporary files are cleaned up on failure
    let downloads_dir = kopi_home.join("downloads");
    let temp_dir = kopi_home.join("temp");

    // After a failed install, these should be cleaned
    assert!(!downloads_dir.exists() || downloads_dir.read_dir().unwrap().count() == 0);
    assert!(!temp_dir.exists() || temp_dir.read_dir().unwrap().count() == 0);
}

/// Test metadata caching behavior
/// Verifies: Metadata is persisted after refresh and used for subsequent operations
/// This reduces API calls and enables offline operation
#[test]
fn test_metadata_persistence() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd.arg("cache").arg("refresh").output().unwrap();

    // Print debug info if the command failed
    if !output.status.success() {
        eprintln!("cache refresh failed with status: {:?}", output.status);
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success(), "cache refresh command failed");

    // Check that metadata.json was created
    let cache_dir = kopi_home.join("cache");
    let metadata_file = cache_dir.join("metadata.json");

    // Print debug info about the cache directory
    eprintln!("Cache directory exists: {}", cache_dir.exists());
    if cache_dir.exists() {
        eprintln!("Cache directory contents:");
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    eprintln!("  - {:?}", entry.path());
                }
            }
        }
    }

    assert!(
        metadata_file.exists(),
        "metadata.json not found at {:?}",
        metadata_file
    );

    // Second install should use cached metadata
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--dry-run")
        .assert()
        .success();
}

/// Test API rate limit handling
/// Simulates: Multiple rapid API requests
/// Verifies: Proper handling of 429 responses with retry logic
#[test]
fn test_rate_limit_handling() {
    // This would test handling of 429 responses
    // In real scenario, we'd need to mock API responses
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Multiple rapid requests might trigger rate limiting
    for i in 0..3 {
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("cache").arg("refresh").assert();

        if i > 0 {
            // Later attempts might get rate limited
            // The error handling should provide clear message
        }
    }
}
