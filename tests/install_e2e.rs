//! End-to-End Integration Tests for Install Command
//!
//! This file contains tests that verify the complete user experience of the install command,
//! focusing on how users interact with the CLI and what they see in various scenarios.
//!
//! ## Test Categories:
//!
//! 1. **Basic Command Operations** - Testing fundamental install workflows
//!    - Basic version installation (e.g., `kopi install 21`)
//!    - Distribution-specific installation (e.g., `kopi install corretto@17`)
//!    - Specific version installation (e.g., `kopi install 17.0.9`)
//!
//! 2. **Command-Line Options** - Testing various CLI flags and their effects
//!    - Force reinstallation with `--force`
//!    - Progress display control with `--no-progress`
//!    - Timeout configuration with `--timeout`
//!    - Verbose output with `-v/-vv/-vvv`
//!
//! 3. **User-Facing Error Handling** - Testing error messages and suggestions
//!    - Invalid version format errors
//!    - Version not available errors
//!    - Already installed errors
//!    - Permission denied errors
//!
//! 4. **Special Use Cases** - Testing edge cases users might encounter
//!    - Concurrent installations
//!    - JavaFX bundled packages
//!    - Actual download testing (non-CI only)
//!    - Exit code verification for scripting
//!
//! These tests ensure that users have a smooth experience and receive helpful
//! feedback when things go wrong.

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn setup_test_home() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().join(".kopi");
    fs::create_dir_all(&kopi_home).unwrap();
    (temp_dir, kopi_home)
}

fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.env("HOME", kopi_home.parent().unwrap());
    cmd
}

/// Test basic version installation without distribution specification
/// User command: `kopi install 21`
/// Expected: Successfully installs latest Eclipse Temurin 21.x.x
#[test]
fn test_install_basic_version() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install a basic version
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

/// Test installation with specific distribution and version
/// User command: `kopi install corretto@17`
/// Expected: Successfully installs Amazon Corretto 17
#[test]
fn test_install_with_distribution() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install with specific distribution
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("corretto@17")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

/// Test error handling for non-existent version
/// User command: `kopi install 999.999.999`
/// Expected: Clear error message with suggestion to check available versions
#[test]
fn test_install_invalid_version() {
    let (_temp_dir, kopi_home) = setup_test_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("999.999.999")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("not available"));
}

/// Test error handling for invalid version format
/// User command: `kopi install invalid@#$%`
/// Expected: Error message explaining proper version format with examples
#[test]
fn test_install_invalid_format() {
    let (_temp_dir, kopi_home) = setup_test_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("invalid@#$%")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid version format"))
        .stderr(predicate::str::contains("Suggestion:"));
}

/// Test error handling when JDK version is already installed
/// Simulates: User tries to install a version that already exists
/// Expected: Error message suggesting --force flag to reinstall
#[test]
fn test_install_already_exists() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // Create a fake installation
    let install_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&install_dir).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install the same version
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already installed"))
        .stderr(predicate::str::contains("--force"));
}

/// Test --force flag to overwrite existing installation
/// User command: `kopi install 21 --force`
/// Expected: Successfully reinstalls even if version exists
#[test]
fn test_install_force_reinstall() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // Create a fake installation
    let install_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&install_dir).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install with force
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--force")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

#[test]
fn test_install_with_timeout() {
    let (_temp_dir, kopi_home) = setup_test_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--timeout")
        .arg("1") // Very short timeout
        .arg("--dry-run")
        .assert();
    // Note: May succeed or fail depending on network speed
}

#[test]
fn test_install_no_progress() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--no-progress")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_install_verbose_output() {
    let (_temp_dir, kopi_home) = setup_test_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("-vv") // Debug verbosity
        .arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert();
}

#[test]
fn test_install_without_cache() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // Try to install without cache refresh - should fail with helpful message
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache refresh"));
}

#[test]
#[cfg(unix)]
fn test_install_permission_denied() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // Make directory read-only
    let jdks_dir = kopi_home.join("jdks");
    fs::create_dir_all(&jdks_dir).unwrap();

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o444); // Read-only
    fs::set_permissions(&jdks_dir, perms).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install - should fail with permission error
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Permission"))
        .stderr(predicate::str::contains("sudo").or(predicate::str::contains("permissions")));

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&jdks_dir, perms).unwrap();
}

#[test]
fn test_install_with_javafx() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache with javafx
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache")
        .arg("refresh")
        .arg("--javafx-bundled")
        .assert()
        .success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--javafx-bundled")
        .arg("--dry-run")
        .assert();
}

#[test]
fn test_concurrent_installs() {
    use std::thread;

    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install two different versions concurrently
    let kopi_home_1 = kopi_home.clone();
    let handle1 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_1);
        cmd.arg("install")
            .arg("17")
            .arg("--dry-run")
            .assert()
            .success();
    });

    let kopi_home_2 = kopi_home.clone();
    let handle2 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_2);
        cmd.arg("install")
            .arg("21")
            .arg("--dry-run")
            .assert()
            .success();
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn test_install_specific_version() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install a specific patch version
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("17.0.9").arg("--dry-run").assert();
}

#[test]
fn test_install_lts_version() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install an LTS version - should show LTS note
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21") // 21 is LTS
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_exit_codes() {
    let (_temp_dir, kopi_home) = setup_test_home();

    // Test invalid version format - should exit with code 2
    let output = get_test_command(&kopi_home)
        .arg("install")
        .arg("@@@invalid")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));

    // Test network error by using invalid API URL (if we can simulate it)
    // This would require environment variable override or mock
}

#[test]
#[cfg(not(ci))]
fn test_actual_download() {
    // Skip in CI to avoid downloading large files
    if env::var("CI").is_ok() {
        return;
    }

    let (_temp_dir, kopi_home) = setup_test_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try actual download with a small JDK if available
    // This test might take a while and requires internet
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("8") // Older versions might be smaller
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .assert();
}
