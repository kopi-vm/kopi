// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::process::Command;
use tempfile::TempDir;

fn run_kopi(args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

fn run_kopi_with_test_home(args: &[&str]) -> (String, String, bool, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);
    cmd.env("KOPI_HOME", temp_dir.path());

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success(), temp_dir)
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
    assert!(stderr.contains("requires a specific version"));
}

#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_install_dry_run() {
    // Skip if explicitly disabled
    if std::env::var("SKIP_NETWORK_TESTS").is_ok() {
        println!("Skipping network test due to SKIP_NETWORK_TESTS env var");
        return;
    }

    let (_, _, _, temp_dir) = run_kopi_with_test_home(&["cache", "refresh"]);
    let temp_path = temp_dir.path().to_path_buf();

    // Run install with dry-run using the same temp home
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["install", "21", "--dry-run"]);
    cmd.env("KOPI_HOME", &temp_path);

    let output = cmd.output().expect("Failed to execute kopi");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    if !success {
        eprintln!("test_install_dry_run failed: stdout={stdout}, stderr={stderr}");
    }
    assert!(success);
    assert!(stdout.contains("Would install"));
}

#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_install_version_not_found() {
    // Skip if explicitly disabled
    if std::env::var("SKIP_NETWORK_TESTS").is_ok() {
        println!("Skipping network test due to SKIP_NETWORK_TESTS env var");
        return;
    }

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
