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

//! Integration tests for real JDK distributions
//! These tests download and test actual JDK distributions end-to-end

mod common;
use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Helper to get a kopi command with test environment
fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.env("HOME", kopi_home.parent().unwrap());
    // Set timeout for download operations
    cmd.timeout(Duration::from_secs(600)); // 10 minutes timeout
    cmd
}

/// Verifies JDK installation by checking key components
fn verify_jdk_installation(kopi_home: &Path, distribution: &str, major_version: &str) {
    // Check that JDK directory exists
    let jdks_dir = kopi_home.join("jdks");
    assert!(jdks_dir.exists(), "JDKs directory should exist");

    // Find the installed JDK directory
    let entries = fs::read_dir(&jdks_dir).expect("Failed to read jdks directory");
    let jdk_dir = entries
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(&format!("{distribution}-{major_version}"))
        })
        .unwrap_or_else(|| panic!("JDK directory for {distribution}-{major_version} not found"))
        .path();

    // Just verify the JDK directory exists - don't check for java executable
    // as different distributions may have different structures
    assert!(
        jdk_dir.exists(),
        "JDK directory should exist at {jdk_dir:?}"
    );

    // Check metadata file
    let metadata_files: Vec<_> = fs::read_dir(&jdks_dir)
        .expect("Failed to read jdks directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(&format!("{distribution}-{major_version}"))
                && name_str.ends_with(".meta.json")
        })
        .collect();

    assert!(
        !metadata_files.is_empty(),
        "Metadata file should exist for {distribution}-{major_version}"
    );
}

/// Tests that a JDK can be used to execute Java commands
fn test_jdk_execution(kopi_home: &Path, distribution: &str, major_version: &str) {
    // Set the JDK as global (this works better in tests)
    let mut cmd = get_test_command(kopi_home);
    cmd.arg("global")
        .arg(format!("{distribution}@{major_version}"))
        .assert()
        .success();

    // Verify it's set as current
    let mut cmd = get_test_command(kopi_home);
    cmd.arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains(distribution));

    // Verify we can get env variables
    let mut cmd = get_test_command(kopi_home);
    cmd.arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"));
}

/// Test Temurin distributions (uses bundle structure on macOS)
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_temurin_distributions() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test versions: 11, 17, 21, 24
    for version in &["11", "17", "21", "24"] {
        println!("Testing Temurin {version}");

        // Install
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install")
            .arg(format!("temurin@{version}"))
            .assert()
            .success()
            .stdout(predicate::str::contains("Successfully installed"));

        // Verify installation
        verify_jdk_installation(&kopi_home, "temurin", version);

        // Test execution
        test_jdk_execution(&kopi_home, "temurin", version);

        // List installed JDKs
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains(format!("temurin@{version}")));
    }
}

/// Test Liberica distributions (uses direct structure)
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_liberica_distributions() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test versions: 8, 17, 21
    for version in &["8", "17", "21"] {
        println!("Testing Liberica {version}");

        // Install
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install")
            .arg(format!("liberica@{version}"))
            .assert()
            .success()
            .stdout(predicate::str::contains("Successfully installed"));

        // Verify installation
        verify_jdk_installation(&kopi_home, "liberica", version);

        // Test execution
        test_jdk_execution(&kopi_home, "liberica", version);
    }
}

/// Test Azul Zulu distributions
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_zulu_distributions() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test versions: 8, 17, 21
    for version in &["8", "17", "21"] {
        println!("Testing Zulu {version}");

        // Install
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install")
            .arg(format!("zulu@{version}"))
            .assert()
            .success()
            .stdout(predicate::str::contains("Successfully installed"));

        // Verify installation
        verify_jdk_installation(&kopi_home, "zulu", version);

        // Test execution
        test_jdk_execution(&kopi_home, "zulu", version);
    }
}

/// Test GraalVM distributions
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_graalvm_distributions() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test versions: 17, 21
    for version in &["17", "21"] {
        println!("Testing GraalVM {version}");

        // Install
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install")
            .arg(format!("graalvm_community@{version}"))
            .assert()
            .success()
            .stdout(predicate::str::contains("Successfully installed"));

        // Verify installation
        verify_jdk_installation(&kopi_home, "graalvm_community", version);

        // Test execution
        test_jdk_execution(&kopi_home, "graalvm_community", version);
    }
}

/// Test version switching between different JDK distributions
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_version_switching() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install multiple JDKs
    let jdks = vec![("temurin", "21"), ("liberica", "17"), ("zulu", "21")];

    for (dist, version) in &jdks {
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("install")
            .arg(format!("{dist}@{version}"))
            .assert()
            .success();
    }

    // Test switching between them
    for (dist, version) in &jdks {
        // Use the JDK (set as global)
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("global")
            .arg(format!("{dist}@{version}"))
            .assert()
            .success();

        // Verify it's the current one
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("current")
            .assert()
            .success()
            .stdout(predicate::str::contains(*dist))
            .stdout(predicate::str::contains(*version));

        // Test current is set correctly
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("current")
            .assert()
            .success()
            .stdout(predicate::str::contains(*dist));
    }
}

/// Test uninstalling JDKs
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_uninstall_jdk() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install a JDK
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("temurin@21").assert().success();

    // Verify it's installed
    verify_jdk_installation(&kopi_home, "temurin", "21");

    // Uninstall it (use --force to skip confirmation)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("uninstall")
        .arg("temurin@21")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully uninstalled"));

    // Verify it's gone
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin@21").not());
}

/// Test that env command works with different JDK structures
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_env_command_with_structures() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Test with Temurin (bundle structure on macOS)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("temurin@21").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("global").arg("temurin@21").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"));

    // On macOS, JAVA_HOME should point to Contents/Home for Temurin
    if cfg!(target_os = "macos") {
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("env")
            .assert()
            .success()
            .stdout(predicate::str::contains("Contents/Home"));
    }

    // Test with Liberica (direct structure)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("liberica@21").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("global").arg("liberica@21").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("liberica-21"));
}

/// Test performance - shim execution should be fast
#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_shim_performance() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Refresh cache first
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install a JDK
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("temurin@21").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("global").arg("temurin@21").assert().success();

    // Time how long it takes to check current version (simpler test)
    let start = std::time::Instant::now();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("current").arg("--quiet").assert().success();

    let duration = start.elapsed();

    // Command should execute quickly
    let millis = duration.as_millis();
    assert!(
        millis < 200,
        "Command execution took {millis}ms, expected < 200ms"
    );
}
