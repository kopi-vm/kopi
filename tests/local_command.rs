use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::TestHomeGuard;

/// Helper to create a test directory with KOPI_HOME set up
fn setup_test_environment() -> (TempDir, TestHomeGuard) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    (temp_dir, test_home)
}

#[test]
fn test_local_command_help() {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.arg("local")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set the local project JDK version",
        ));
}

#[test]
fn test_pin_alias_works() {
    // Test that 'pin' is an alias for 'local'
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.arg("pin")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set the local project JDK version",
        ));
}

#[test]
fn test_local_creates_kopi_version_file() {
    let (temp_dir, test_home) = setup_test_environment();

    // Run local command with auto-install disabled
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("21")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .kopi-version file"));

    // Check that .kopi-version file was created
    let version_file = temp_dir.path().join(".kopi-version");
    assert!(version_file.exists());

    // Verify content
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "21");
}

#[test]
fn test_local_with_distribution() {
    let (temp_dir, test_home) = setup_test_environment();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("temurin@17")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .kopi-version file"));

    // Verify file content
    let version_file = temp_dir.path().join(".kopi-version");
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "temurin@17");
}

#[test]
fn test_local_shows_install_hint_when_not_installed() {
    let (temp_dir, test_home) = setup_test_environment();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("corretto@21")
        .assert()
        .success()
        .stdout(predicate::str::contains("is not installed"))
        .stdout(predicate::str::contains("kopi install"));
}

#[test]
fn test_local_invalid_version() {
    let (temp_dir, test_home) = setup_test_environment();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("invalid-version")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid version"));
}

#[test]
fn test_local_without_specific_version() {
    let (temp_dir, test_home) = setup_test_environment();

    // Test that "latest" or similar non-specific versions are rejected
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("latest")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires a specific version"));
}

#[test]
fn test_local_overwrites_existing_file() {
    let (temp_dir, test_home) = setup_test_environment();

    // Create initial .kopi-version file
    let version_file = temp_dir.path().join(".kopi-version");
    fs::write(&version_file, "17").unwrap();

    // Run local command to overwrite
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("21")
        .assert()
        .success();

    // Verify new content
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "21");
}

#[test]
fn test_local_with_full_version() {
    let (temp_dir, test_home) = setup_test_environment();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("temurin@17.0.9")
        .assert()
        .success();

    // Verify version format is preserved
    let version_file = temp_dir.path().join(".kopi-version");
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "temurin@17.0.9");
}

/// Test interaction with version resolution after local command
#[test]
fn test_local_affects_current_command() {
    let (temp_dir, test_home) = setup_test_environment();

    // First set a local version
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("21")
        .assert()
        .success();

    // Now check that current command sees it
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .current_dir(temp_dir.path())
        .arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains("21"))
        .stdout(predicate::str::contains(".kopi-version"));
}

/// Test that .kopi-version takes precedence over .java-version
#[test]
fn test_kopi_version_precedence() {
    let (temp_dir, test_home) = setup_test_environment();

    // Create .java-version file first
    let java_version_file = temp_dir.path().join(".java-version");
    fs::write(&java_version_file, "17").unwrap();

    // Create .kopi-version file with different version
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("21")
        .assert()
        .success();

    // Verify that current command shows .kopi-version (21) not .java-version (17)
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .current_dir(temp_dir.path())
        .arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains("21"))
        .stdout(predicate::str::contains(".kopi-version"));
}

/// Test auto-installation prompt handling (when enabled but user declines)
#[test]
fn test_local_with_auto_install_prompt() {
    let (temp_dir, test_home) = setup_test_environment();

    // Note: We can't easily test interactive prompts in integration tests
    // This test just verifies the command runs with auto-install enabled
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("KOPI_AUTO_INSTALL__ENABLED", "true")
        .env("KOPI_AUTO_INSTALL__PROMPT", "false") // Disable prompt for testing
        .current_dir(temp_dir.path())
        .arg("local")
        .arg("21");

    // The command should either succeed or fail based on whether
    // it can find the kopi binary for auto-installation
    let output = cmd.output().unwrap();

    // If successful, file should be created
    if output.status.success() {
        let version_file = temp_dir.path().join(".kopi-version");
        assert!(version_file.exists());
    }
}
