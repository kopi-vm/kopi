use std::env;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

mod common;
use common::TestHomeGuard;

fn run_kopi_with_env(args: &[&str], env_vars: &[(&str, &str)]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

fn run_kopi(args: &[&str]) -> (String, String, bool) {
    run_kopi_with_env(args, &[])
}

#[test]
fn test_current_help() {
    let (stdout, _, success) = run_kopi(&["current", "--help"]);
    assert!(success);
    assert!(stdout.contains("Show currently active JDK version"));
    assert!(stdout.contains("--quiet"));
    assert!(stdout.contains("--json"));
}

#[test]
fn test_current_no_version_configured() {
    // Create a temporary directory to ensure clean environment
    let temp_dir = TempDir::new().unwrap();
    let temp_home = TestHomeGuard::new();
    temp_home.setup_kopi_structure();

    // Run in temp directory with clean KOPI_HOME
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env("KOPI_HOME", temp_home.kopi_home());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(!output.status.success());
    assert!(stderr.contains("No JDK version configured"));
    assert!(stderr.contains("kopi local"));
    assert!(stderr.contains("kopi global"));
}

#[test]
fn test_current_with_environment_variable() {
    let (stdout, _, success) = run_kopi_with_env(&["current"], &[("KOPI_JAVA_VERSION", "17")]);
    assert!(success);
    assert!(stdout.contains("17"));
    assert!(stdout.contains("KOPI_JAVA_VERSION"));
}

#[test]
fn test_current_with_environment_variable_distribution() {
    let (stdout, _, success) =
        run_kopi_with_env(&["current"], &[("KOPI_JAVA_VERSION", "temurin@21")]);
    assert!(success);
    assert!(stdout.contains("21") || stdout.contains("temurin@21"));
    assert!(stdout.contains("KOPI_JAVA_VERSION"));
}

#[test]
fn test_current_quiet_mode() {
    let (stdout, _, success) =
        run_kopi_with_env(&["current", "-q"], &[("KOPI_JAVA_VERSION", "17.0.9")]);
    assert!(success);
    assert_eq!(stdout.trim(), "17.0.9");
}

#[test]
fn test_current_json_output() {
    let (stdout, _, success) =
        run_kopi_with_env(&["current", "--json"], &[("KOPI_JAVA_VERSION", "21")]);
    assert!(success);

    // Parse JSON to verify structure
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert_eq!(json["version"], "21");
    assert_eq!(json["source"], "KOPI_JAVA_VERSION");
    assert!(json["source_path"].is_string());
    assert!(json["installed"].is_boolean());
}

#[test]
fn test_current_json_error_output() {
    let temp_dir = TempDir::new().unwrap();
    let temp_home = TestHomeGuard::new();
    temp_home.setup_kopi_structure();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current", "--json"]);
    cmd.current_dir(temp_dir.path());
    cmd.env("KOPI_HOME", temp_home.kopi_home());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(!output.status.success());

    // Parse JSON error output
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert_eq!(json["error"], "no_version_configured");
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("No JDK version configured")
    );
    assert!(json["hints"].is_array());
}

#[test]
fn test_current_with_project_file() {
    let temp_dir = TempDir::new().unwrap();
    let version_file = temp_dir.path().join(".kopi-version");
    fs::write(&version_file, "corretto@17.0.8").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(output.status.success());
    assert!(stdout.contains("17.0.8") || stdout.contains("corretto@17.0.8"));
    assert!(stdout.contains(".kopi-version"));
}

#[test]
fn test_current_with_java_version_file() {
    let temp_dir = TempDir::new().unwrap();
    let version_file = temp_dir.path().join(".java-version");
    fs::write(&version_file, "11.0.2").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(output.status.success());
    assert!(stdout.contains("11.0.2"));
    assert!(stdout.contains(".java-version"));
}

#[test]
fn test_current_kopi_version_takes_precedence() {
    let temp_dir = TempDir::new().unwrap();

    // Create both version files
    let kopi_version = temp_dir.path().join(".kopi-version");
    fs::write(&kopi_version, "temurin@21").unwrap();

    let java_version = temp_dir.path().join(".java-version");
    fs::write(&java_version, "17").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(output.status.success());
    assert!(stdout.contains("21") || stdout.contains("temurin@21"));
    assert!(stdout.contains(".kopi-version"));
    assert!(!stdout.contains(".java-version"));
}

#[test]
fn test_current_searches_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let parent_dir = temp_dir.path();
    let child_dir = parent_dir.join("child");
    fs::create_dir_all(&child_dir).unwrap();

    // Place version file in parent
    let version_file = parent_dir.join(".kopi-version");
    fs::write(&version_file, "zulu@8").unwrap();

    // Run command from child directory
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(child_dir);
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(output.status.success());
    assert!(stdout.contains("8") || stdout.contains("zulu@8"));
}

#[test]
fn test_current_environment_takes_priority() {
    let temp_dir = TempDir::new().unwrap();

    // Create project file
    let version_file = temp_dir.path().join(".kopi-version");
    fs::write(&version_file, "corretto@17").unwrap();

    // Set environment variable
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env("KOPI_JAVA_VERSION", "temurin@21");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(output.status.success());
    assert!(stdout.contains("21") || stdout.contains("temurin@21"));
    assert!(stdout.contains("KOPI_JAVA_VERSION"));
    assert!(!stdout.contains(".kopi-version"));
}

#[test]
#[ignore = "Global default currently uses dirs::home_dir() instead of KOPI_HOME, making it difficult to test in isolation"]
fn test_current_with_global_default() {
    let temp_home = TestHomeGuard::new();
    temp_home.setup_kopi_structure();

    // Create global default version file
    let global_version_file = temp_home.kopi_home().join("default-version");
    fs::write(&global_version_file, "11").unwrap();

    // Create a temp directory for running the command
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"]);
    cmd.current_dir(temp_dir.path());
    cmd.env("KOPI_HOME", temp_home.kopi_home());
    cmd.env_remove("KOPI_JAVA_VERSION");

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Print debug info if test fails
    if !output.status.success() {
        eprintln!("Command failed!");
        eprintln!("Stdout: {stdout}");
        eprintln!("Stderr: {stderr}");
    }

    assert!(output.status.success());
    assert!(stdout.contains("11"));
    assert!(stdout.contains("global default"));
}

#[test]
fn test_current_shows_not_installed_warning() {
    // This test shows that uninstalled versions are properly marked
    let (stdout, stderr, success) =
        run_kopi_with_env(&["current"], &[("KOPI_JAVA_VERSION", "99.99.99")]);
    assert!(success);
    assert!(stdout.contains("[NOT INSTALLED]"));
    assert!(stderr.contains("Warning"));
    assert!(stderr.contains("kopi install"));
}
