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

//! End-to-end simulation tests for JDK bundle structure handling.
//! These tests simulate real JDK distributions without requiring actual downloads.

use kopi::config::KopiConfig;
use kopi::storage::JdkLister;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

mod common;
use common::TestHomeGuard;

/// Simulates a real JDK installation structure based on known patterns
fn simulate_real_jdk_structure(jdk_path: &Path, vendor: &str, version: &str) {
    match vendor {
        "temurin" => {
            // Temurin uses bundle structure on macOS
            #[cfg(target_os = "macos")]
            {
                let home_dir = jdk_path.join("Contents").join("Home");
                let bin_dir = home_dir.join("bin");
                fs::create_dir_all(&bin_dir).unwrap();

                // Create realistic executables
                create_mock_java_executables(&bin_dir);

                // Create other JDK directories
                fs::create_dir_all(home_dir.join("lib")).unwrap();
                fs::create_dir_all(home_dir.join("conf")).unwrap();
                fs::create_dir_all(home_dir.join("jmods")).unwrap();

                // Create version file
                fs::write(
                    home_dir.join("release"),
                    format!("JAVA_VERSION=\"{version}\""),
                )
                .unwrap();
            }
            #[cfg(not(target_os = "macos"))]
            {
                create_direct_jdk_structure(jdk_path, version);
            }
        }
        "liberica" => {
            // Liberica uses direct structure
            create_direct_jdk_structure(jdk_path, version);
        }
        "zulu" => {
            // Zulu uses hybrid structure on macOS (symlinks to bundle)
            #[cfg(target_os = "macos")]
            {
                // Create bundle structure
                let home_dir = jdk_path.join("Contents").join("Home");
                let bundle_bin_dir = home_dir.join("bin");
                fs::create_dir_all(&bundle_bin_dir).unwrap();
                create_mock_java_executables(&bundle_bin_dir);

                // Create symlinks at root
                #[cfg(unix)]
                {
                    use std::os::unix::fs::symlink;
                    symlink(&bundle_bin_dir, jdk_path.join("bin")).unwrap();
                    symlink(home_dir.join("lib"), jdk_path.join("lib")).unwrap();
                }

                fs::write(
                    home_dir.join("release"),
                    format!("JAVA_VERSION=\"{version}\""),
                )
                .unwrap();
            }
            #[cfg(not(target_os = "macos"))]
            {
                create_direct_jdk_structure(jdk_path, version);
            }
        }
        _ => {
            // Default to direct structure for other vendors
            create_direct_jdk_structure(jdk_path, version);
        }
    }
}

fn create_direct_jdk_structure(jdk_path: &Path, version: &str) {
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    create_mock_java_executables(&bin_dir);

    fs::create_dir_all(jdk_path.join("lib")).unwrap();
    fs::create_dir_all(jdk_path.join("conf")).unwrap();
    fs::create_dir_all(jdk_path.join("jmods")).unwrap();

    fs::write(
        jdk_path.join("release"),
        format!("JAVA_VERSION=\"{version}\""),
    )
    .unwrap();
}

fn create_mock_java_executables(bin_dir: &Path) {
    let executables = ["java", "javac", "jar", "jshell", "jdeps", "javap"];

    for exe in &executables {
        let exe_path = bin_dir.join(exe);
        #[cfg(unix)]
        {
            fs::write(&exe_path, format!("#!/bin/sh\necho \"Mock {exe} version\"")).unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe_path, perms).unwrap();
        }
        #[cfg(windows)]
        {
            fs::write(
                exe_path.with_extension("exe"),
                format!("@echo Mock {exe} version"),
            )
            .unwrap();
        }
    }
}

#[test]
fn test_e2e_temurin_installation_workflow() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Simulate Temurin installation
    let temurin_path = jdks_dir.join("temurin-21.0.1");
    simulate_real_jdk_structure(&temurin_path, "temurin", "21.0.1");

    // Verify installation
    let jdks = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
    let temurin_jdk = jdks
        .iter()
        .find(|j| j.distribution == "temurin")
        .expect("Temurin JDK not found");

    // Test path resolution
    let java_home = temurin_jdk.resolve_java_home();
    #[cfg(target_os = "macos")]
    assert_eq!(java_home, temurin_path.join("Contents").join("Home"));
    #[cfg(not(target_os = "macos"))]
    assert_eq!(java_home, temurin_path);

    // Test bin path resolution
    let bin_path = temurin_jdk.resolve_bin_path().unwrap();
    let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
    assert!(bin_path.join(java_binary).exists());

    // Test setting as current version
    fs::write(test_home.path().join(".kopi-version"), "temurin@21.0.1").unwrap();

    // Run 'current' command to verify
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(test_home.path());

    let output = cmd.output().expect("Failed to execute kopi current");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed with exit code: {:?}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        output.status.code()
    );

    assert!(
        stdout.contains("temurin@21"),
        "Expected 'temurin@21' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
}

#[test]
fn test_e2e_multiple_vendors_switching() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Install multiple JDKs with different structures
    let vendors = [
        ("temurin", "21.0.1"),
        ("liberica", "17.0.9"),
        ("zulu", "11.0.21"),
    ];

    for (vendor, version) in &vendors {
        let jdk_path = jdks_dir.join(format!("{vendor}-{version}"));
        simulate_real_jdk_structure(&jdk_path, vendor, version);
    }

    // List all JDKs
    let jdks = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
    assert_eq!(jdks.len(), 3);

    // Test switching between versions
    for (vendor, version) in &vendors {
        // Set version
        fs::write(
            test_home.path().join(".kopi-version"),
            format!("{vendor}@{version}"),
        )
        .unwrap();

        // Run env command
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
        cmd.args(["env"])
            .env("KOPI_HOME", test_home.kopi_home())
            .env("HOME", test_home.path())
            .current_dir(test_home.path());

        let output = cmd.output().expect("Failed to execute kopi env");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Command failed with exit code: {:?}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
            output.status.code()
        );

        assert!(
            stdout.contains("JAVA_HOME="),
            "Expected 'JAVA_HOME=' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}\nExit code: {}",
            output.status.code().unwrap_or(-1)
        );

        // Verify the JDK is correctly resolved
        let jdk = jdks.iter().find(|j| j.distribution == *vendor).unwrap();
        let _java_home = jdk.resolve_java_home();
        // The output path might have different formats on different platforms
        // Just check that the vendor name and version are in the output
        assert!(
            stdout.contains(vendor),
            "Expected vendor {vendor} in output: {stdout}"
        );
        assert!(
            stdout.contains(version),
            "Expected version {version} in output: {stdout}"
        );
    }
}

#[test]
fn test_e2e_project_hierarchy() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Install JDKs
    simulate_real_jdk_structure(&jdks_dir.join("temurin-21.0.1"), "temurin", "21.0.1");
    simulate_real_jdk_structure(&jdks_dir.join("liberica-17.0.9"), "liberica", "17.0.9");

    // Create project hierarchy
    let project_root = test_home.path().join("my-project");
    let subproject = project_root.join("subproject");
    fs::create_dir_all(&subproject).unwrap();

    // Set different versions at different levels
    fs::write(project_root.join(".kopi-version"), "temurin@21.0.1").unwrap();
    fs::write(subproject.join(".java-version"), "17.0.9").unwrap(); // Test .java-version compatibility

    // Test from project root
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(&project_root);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed with exit code: {:?}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        output.status.code()
    );

    assert!(
        stdout.contains("temurin@21"),
        "Expected 'temurin@21' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );

    // Test from subproject (should use .java-version)
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(&subproject);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed with exit code: {:?}\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("17.0.9"),
        "Expected '17.0.9' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn test_e2e_macos_specific_structures() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Test all three structure types on macOS
    let test_cases = [
        ("temurin-21.0.1", "temurin", "21.0.1", "bundle"),
        ("liberica-21.0.1", "liberica", "21.0.1", "direct"),
        ("zulu-21.0.1", "zulu", "21.0.1", "hybrid"),
    ];

    for (dir_name, vendor, version, expected_type) in &test_cases {
        let jdk_path = jdks_dir.join(dir_name);
        simulate_real_jdk_structure(&jdk_path, vendor, version);

        // Set as current version
        fs::write(
            test_home.path().join(".kopi-version"),
            format!("{vendor}@{version}"),
        )
        .unwrap();

        // Get env output
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
        cmd.args(["env"])
            .env("KOPI_HOME", test_home.kopi_home())
            .env("HOME", test_home.path())
            .current_dir(test_home.path());

        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "Failed for {}: {:?}",
            vendor,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Debug output
        eprintln!("Testing {vendor} ({expected_type}): {}", stdout.trim());

        // Verify JAVA_HOME is set correctly based on structure type
        match *expected_type {
            "bundle" => {
                assert!(
                    stdout.contains(&format!(
                        "JAVA_HOME=\"{}/Contents/Home\"",
                        jdk_path.display()
                    )),
                    "Expected bundle JAVA_HOME for {vendor}, got: {stdout}"
                );
            }
            "direct" => {
                assert!(
                    stdout.contains(&format!("JAVA_HOME=\"{}\"", jdk_path.display())),
                    "Expected direct JAVA_HOME for {vendor}, got: {stdout}"
                );
            }
            "hybrid" => {
                // For hybrid, could be either the root (if symlinks work) or Contents/Home
                let has_root = stdout.contains(&format!("JAVA_HOME=\"{}\"", jdk_path.display()));
                let has_bundle = stdout.contains(&format!(
                    "JAVA_HOME=\"{}/Contents/Home\"",
                    jdk_path.display()
                ));
                assert!(
                    has_root || has_bundle,
                    "Expected hybrid JAVA_HOME for {vendor} to be either root or bundle, got: {stdout}"
                );
            }
            _ => panic!("Unknown structure type"),
        }
    }
}

#[test]
fn test_e2e_version_resolution_priority() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Install a JDK
    simulate_real_jdk_structure(&jdks_dir.join("temurin-21.0.1"), "temurin", "21.0.1");

    // Set global default
    let global_version_file = test_home.kopi_home().join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Create a project directory
    let project_dir = test_home.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Test 1: Global version is used when no local version
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(&project_dir);

    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("temurin@21"));

    // Test 2: Local version overrides global
    simulate_real_jdk_structure(&jdks_dir.join("liberica-17.0.9"), "liberica", "17.0.9");
    fs::write(project_dir.join(".kopi-version"), "liberica@17.0.9").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(&project_dir);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed with exit code: {:?}\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("liberica@17"),
        "Expected 'liberica@17' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );

    // Test 3: Environment variable has highest priority
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(["current"])
        .env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .env("KOPI_JAVA_VERSION", "temurin@21.0.1")
        .current_dir(&project_dir);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed with exit code: {:?}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        output.status.code()
    );

    assert!(
        stdout.contains("temurin@21"),
        "Expected 'temurin@21' in stdout, but got:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
}
