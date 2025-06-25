use std::env;
use std::process::Command;

fn run_kopi(args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

#[test]
fn test_install_help() {
    let (stdout, _, success) = run_kopi(&["install", "--help"]);
    assert!(success);
    assert!(stdout.contains("Install a JDK version"));
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--no-progress"));
    assert!(stdout.contains("--timeout"));
}

#[test]
fn test_install_invalid_version() {
    let (_, stderr, success) = run_kopi(&["install", "invalid"]);
    assert!(!success);
    assert!(stderr.contains("InvalidVersionFormat") || stderr.contains("Invalid version format"));
}

#[test]
fn test_install_unknown_distribution() {
    let (_, stderr, success) = run_kopi(&["install", "unknown@21"]);
    assert!(!success);
    assert!(stderr.contains("Unknown distribution"));
}

#[test]
fn test_install_distribution_without_version() {
    let (_, stderr, success) = run_kopi(&["install", "temurin"]);
    assert!(!success);
    assert!(stderr.contains("without version"));
}

#[test]
fn test_install_dry_run() {
    let (stdout, _, success) = run_kopi(&["install", "21", "--dry-run"]);
    assert!(success);
    assert!(stdout.contains("Would install"));
}

#[test]
#[ignore] // This test requires network access
fn test_install_version_not_found() {
    // This test requires network access to check against real API
    let (_, stderr, success) = run_kopi(&["install", "99.99.99"]);
    assert!(!success);
    assert!(stderr.contains("not found") || stderr.contains("not available"));
}

#[test]
fn test_cli_version() {
    let (stdout, _, success) = run_kopi(&["--version"]);
    assert!(success);
    assert!(stdout.contains("kopi"));
}

#[test]
fn test_cli_help() {
    let (stdout, _, success) = run_kopi(&["--help"]);
    assert!(success);
    assert!(stdout.contains("JDK version management tool"));
    assert!(stdout.contains("install"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("use"));
    assert!(stdout.contains("current"));
}
