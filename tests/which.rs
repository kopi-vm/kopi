#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command as AssertCommand;
use common::{TestHomeGuard, fixtures};
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_which_command_basic() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // First install a JDK using the real command
    let mut install_cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    install_cmd
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["install", "temurin@21", "--dry-run"])
        .output()
        .expect("Failed to run install command");

    // Create a fake installed JDK for testing
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Test basic which
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "temurin@21"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bin").and(predicate::str::contains("java")));
}

#[test]
fn test_which_current_project() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "17.0.11+9");

    // Create a project version file
    std::fs::write(_guard.path().join(".kopi-version"), "temurin@17.0.11+9").unwrap();

    // Which without version should find project version
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .current_dir(_guard.path())
        .args(["which"])
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin-17"));
}

#[test]
fn test_which_tools() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Test various tools
    for tool in &["java", "javac", "jar", "jshell"] {
        AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
            .env("KOPI_HOME", _guard.kopi_home())
            .args(["which", "--tool", tool, "temurin@21"])
            .assert()
            .success()
            .stdout(predicate::str::contains(*tool));
    }
}

#[test]
fn test_which_home_option() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Home option should not include /bin/java
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "--home", "temurin@21"])
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin-21").and(predicate::str::contains("bin").not()));
}

#[test]
fn test_which_json_format() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    let output = AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "--json", "temurin@21"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["distribution"], "temurin");
    assert_eq!(json["tool"], "java");
    assert!(json["tool_path"].as_str().unwrap().contains("java"));
}

#[test]
fn test_which_not_installed() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "temurin@22"])
        .assert()
        .failure()
        .code(4) // JdkNotInstalled
        .stderr(predicate::str::contains("not installed"));
}

#[test]
fn test_which_tool_not_found() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "--tool", "nonexistent", "temurin@21"])
        .assert()
        .failure()
        .code(5) // ToolNotFound
        .stderr(predicate::str::contains("Tool 'nonexistent' not found"));
}

#[test]
fn test_which_help() {
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["which", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show installation path for a JDK version",
        ));
}

#[test]
fn test_which_w_alias() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Test 'w' alias works
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["w", "temurin@21"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bin").and(predicate::str::contains("java")));
}

#[test]
fn test_which_environment_variable() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "corretto", "11.0.23.9.1");

    // Set environment variable
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .env("KOPI_JAVA_VERSION", "corretto@11.0.23.9.1")
        .args(["which"])
        .assert()
        .success()
        .stdout(predicate::str::contains("corretto-11"));
}

#[test]
fn test_which_global_default() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "zulu", "8.78.0.19");

    // Set global default
    std::fs::write(_guard.kopi_home().join("version"), "zulu@8.78.0.19").unwrap();

    // Which without version should find global default
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .env_remove("KOPI_JAVA_VERSION")
        .current_dir("/") // Ensure we're not in a project directory
        .args(["which"])
        .assert()
        .success()
        .stdout(predicate::str::contains("zulu-8"));
}

#[test]
fn test_which_ambiguous_version() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create multiple JDKs with same major version
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "corretto", "21.0.5.11.1");

    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "21"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Multiple JDKs match").and(
            predicate::str::contains("temurin@21").and(predicate::str::contains("corretto@21")),
        ));
}

#[test]
fn test_which_no_version_configured() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // No version configured anywhere
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .env_remove("KOPI_JAVA_VERSION")
        .current_dir("/") // Ensure we're not in a project directory
        .args(["which"])
        .assert()
        .failure()
        .code(3) // NoLocalVersion
        .stderr(predicate::str::contains("No JDK configured"));
}
