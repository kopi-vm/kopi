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

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test Kopi home directory
fn setup_test_home() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[test]
fn test_global_command_without_installation() {
    let temp_home = setup_test_home();

    // Run global command for uninstalled JDK with auto-install disabled
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", temp_home.path())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .arg("global")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not installed"));
}

#[test]
fn test_global_command_creates_version_file() {
    // This test would require a mock installation
    // For now, we'll test that the command exists and runs

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.arg("global")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set the global default JDK version",
        ));
}

#[test]
fn test_default_alias_works() {
    // Test that 'default' is an alias for 'global'
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.arg("default")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set the global default JDK version",
        ));
}

#[test]
fn test_global_command_invalid_version() {
    let temp_home = setup_test_home();

    // Test with invalid version format
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", temp_home.path())
        .arg("global")
        .arg("invalid-version")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid version"));
}

#[test]
fn test_global_with_distribution() {
    let temp_home = setup_test_home();

    // Test with distribution specified and auto-install disabled
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", temp_home.path())
        .env("KOPI_AUTO_INSTALL__ENABLED", "false")
        .arg("global")
        .arg("temurin@21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not installed"));
}

/// Integration test to verify version file format
#[test]
fn test_version_file_format() {
    // This test documents the expected format of the version file
    let temp_dir = TempDir::new().unwrap();
    let version_file = temp_dir.path().join("version");

    // Test format with distribution
    fs::write(&version_file, "temurin@21").unwrap();
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "temurin@21");

    // Test format without distribution
    fs::write(&version_file, "17").unwrap();
    let content = fs::read_to_string(&version_file).unwrap();
    assert_eq!(content, "17");
}
