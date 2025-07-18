#[path = "common/mod.rs"]
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

/// Test basic env command with bash shell (default)
#[test]
fn test_env_basic_bash() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version (using the correct filename that VersionResolver expects)
    let global_version_file = kopi_home.join("default-version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with quiet flag
#[test]
fn test_env_quiet_flag() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version (using the correct filename that VersionResolver expects)
    let global_version_file = kopi_home.join("default-version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with quiet flag
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--quiet");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )))
        .stderr(predicate::str::is_empty());
}

/// Test env command with JDK not installed
#[test]
fn test_env_jdk_not_installed() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Set a global version without installing the JDK
    let global_version_file = kopi_home.join("default-version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("is not installed"));
}