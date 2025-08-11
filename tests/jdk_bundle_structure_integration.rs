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

//! Integration tests for JDK bundle structure handling on macOS.
//! Tests the complete workflow from installation through execution.

use kopi::archive::{JdkStructureType, detect_jdk_root};
use kopi::config::KopiConfig;
use kopi::storage::{InstalledJdk, JdkLister};
use kopi::version::Version;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tempfile::TempDir;

mod common;
use common::TestHomeGuard;

fn run_kopi_with_home(home: &TestHomeGuard, args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);
    cmd.env("KOPI_HOME", home.kopi_home());
    cmd.env("HOME", home.path());
    cmd.current_dir(home.path()); // Set working directory to test home

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

fn create_mock_jdk_structure(base_path: &Path, structure_type: &str) -> PathBuf {
    match structure_type {
        "direct" => {
            // Direct structure: bin/ at root
            let bin_dir = base_path.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho \"Mock Java\"").unwrap();
            fs::write(bin_dir.join("javac"), "#!/bin/sh\necho \"Mock Javac\"").unwrap();
            base_path.to_path_buf()
        }
        "bundle" => {
            // Bundle structure: Contents/Home/bin/
            let home_dir = base_path.join("Contents").join("Home");
            let bin_dir = home_dir.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho \"Mock Java\"").unwrap();
            fs::write(bin_dir.join("javac"), "#!/bin/sh\necho \"Mock Javac\"").unwrap();
            base_path.to_path_buf()
        }
        "hybrid" => {
            // Hybrid structure: symlinks at root pointing to bundle
            let contents_dir = base_path.join("Contents");
            let home_dir = contents_dir.join("Home");
            let bin_dir = home_dir.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho \"Mock Java\"").unwrap();
            fs::write(bin_dir.join("javac"), "#!/bin/sh\necho \"Mock Javac\"").unwrap();

            // Create symlinks at root
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                symlink(&bin_dir, base_path.join("bin")).unwrap();
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::symlink_dir;
                symlink_dir(&bin_dir, base_path.join("bin")).unwrap();
            }

            base_path.to_path_buf()
        }
        _ => panic!("Unknown structure type: {structure_type}"),
    }
}

#[test]
fn test_structure_detection_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Test direct structure
    let direct_jdk = temp_dir.path().join("direct-jdk");
    create_mock_jdk_structure(&direct_jdk, "direct");
    let info = detect_jdk_root(&direct_jdk).unwrap();
    assert_eq!(info.jdk_root, direct_jdk);
    assert_eq!(info.java_home_suffix, "");
    assert!(matches!(info.structure_type, JdkStructureType::Direct));

    // Test bundle structure on macOS
    #[cfg(target_os = "macos")]
    {
        let bundle_jdk = temp_dir.path().join("bundle-jdk");
        create_mock_jdk_structure(&bundle_jdk, "bundle");
        let info = detect_jdk_root(&bundle_jdk).unwrap();
        assert_eq!(info.jdk_root, bundle_jdk.join("Contents").join("Home"));
        assert_eq!(info.java_home_suffix, "Contents/Home");
        assert!(matches!(info.structure_type, JdkStructureType::Bundle));

        // Test hybrid structure
        let hybrid_jdk = temp_dir.path().join("hybrid-jdk");
        create_mock_jdk_structure(&hybrid_jdk, "hybrid");
        let info = detect_jdk_root(&hybrid_jdk).unwrap();
        // For hybrid structures, the root is where the symlinks are
        assert_eq!(info.jdk_root, hybrid_jdk);
        assert_eq!(info.java_home_suffix, "");
        assert!(matches!(info.structure_type, JdkStructureType::Hybrid));
    }
}

#[test]
fn test_installed_jdk_path_resolution() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Create mock JDK installations
    let direct_jdk_path = jdks_dir.join("liberica-21");
    create_mock_jdk_structure(&direct_jdk_path, "direct");

    #[cfg(target_os = "macos")]
    let bundle_jdk_path = jdks_dir.join("temurin-21");
    #[cfg(target_os = "macos")]
    create_mock_jdk_structure(&bundle_jdk_path, "bundle");

    // Test InstalledJdk path resolution
    let direct_jdk = InstalledJdk::new(
        "liberica".to_string(),
        Version::from_str("21.0.0").unwrap(),
        direct_jdk_path.clone(),
    );

    // Test resolve_java_home
    let java_home = direct_jdk.resolve_java_home();
    assert_eq!(java_home, direct_jdk_path);

    // Test resolve_bin_path
    let bin_path = direct_jdk.resolve_bin_path().unwrap();
    assert_eq!(bin_path, direct_jdk_path.join("bin"));
    assert!(bin_path.exists());

    #[cfg(target_os = "macos")]
    {
        let bundle_jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.0").unwrap(),
            bundle_jdk_path.clone(),
        );

        // Test resolve_java_home for bundle structure
        let java_home = bundle_jdk.resolve_java_home();
        assert_eq!(java_home, bundle_jdk_path.join("Contents").join("Home"));

        // Test resolve_bin_path for bundle structure
        let bin_path = bundle_jdk.resolve_bin_path().unwrap();
        assert_eq!(
            bin_path,
            bundle_jdk_path.join("Contents").join("Home").join("bin")
        );
        assert!(bin_path.exists());
    }
}

#[test]
fn test_version_switching_workflow() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Create two mock JDKs with different structures
    let liberica_path = jdks_dir.join("liberica-17");
    create_mock_jdk_structure(&liberica_path, "direct");

    #[cfg(target_os = "macos")]
    let temurin_path = jdks_dir.join("temurin-21");
    #[cfg(target_os = "macos")]
    create_mock_jdk_structure(&temurin_path, "bundle");

    // Create .kopi-version files for testing
    fs::write(test_home.path().join(".kopi-version"), "liberica@17").unwrap();

    // Test listing installed JDKs
    let jdks = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
    assert!(
        jdks.iter()
            .any(|j| j.distribution == "liberica" && j.version.to_string() == "17")
    );

    #[cfg(target_os = "macos")]
    {
        assert!(
            jdks.iter()
                .any(|j| j.distribution == "temurin" && j.version.to_string() == "21")
        );

        // Test switching to bundle structure JDK
        fs::write(test_home.path().join(".kopi-version"), "temurin@21").unwrap();

        // Verify the correct JDK would be selected
        let selected_jdk = jdks
            .iter()
            .find(|j| j.distribution == "temurin" && j.version.to_string() == "21")
            .unwrap();

        // Verify path resolution works correctly
        let java_home = selected_jdk.resolve_java_home();
        assert!(java_home.join("bin").join("java").exists());
    }
}

#[test]
fn test_env_command_with_different_structures() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();

    // Create a direct structure JDK
    let liberica_path = jdks_dir.join("liberica-17");
    create_mock_jdk_structure(&liberica_path, "direct");
    fs::write(test_home.path().join(".kopi-version"), "liberica@17").unwrap();

    // Run env command
    let (stdout, stderr, success) = run_kopi_with_home(test_home, &["env"]);
    if !success {
        eprintln!("env command failed with stderr: {stderr}");
        eprintln!("stdout: {stdout}");
    }
    assert!(success);
    assert!(stdout.contains(&format!("JAVA_HOME=\"{}\"", liberica_path.display())));

    #[cfg(target_os = "macos")]
    {
        // Create a bundle structure JDK
        let temurin_path = jdks_dir.join("temurin-21");
        create_mock_jdk_structure(&temurin_path, "bundle");
        fs::write(test_home.path().join(".kopi-version"), "temurin@21").unwrap();

        // Run env command with bundle structure
        let (stdout, _, success) = run_kopi_with_home(test_home, &["env"]);
        assert!(success);
        let expected_java_home = temurin_path.join("Contents").join("Home");
        assert!(stdout.contains(&format!("JAVA_HOME=\"{}\"", expected_java_home.display())));
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_shim_execution_performance() {
    use std::time::Instant;

    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();
    let shims_dir = config.bin_dir().unwrap();

    // Create a bundle structure JDK
    let temurin_path = jdks_dir.join("temurin-21");
    create_mock_jdk_structure(&temurin_path, "bundle");

    // Make the mock java executable
    let java_path = temurin_path
        .join("Contents")
        .join("Home")
        .join("bin")
        .join("java");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms).unwrap();
    }

    // Copy the actual shim binary to the test environment
    let shim_src = env!("CARGO_BIN_EXE_kopi-shim");
    let shim_dst = shims_dir.join("java");
    fs::copy(shim_src, &shim_dst).unwrap();

    // Make shim executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shim_dst).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shim_dst, perms).unwrap();
    }

    // Set up version file
    fs::write(test_home.path().join(".kopi-version"), "temurin@21").unwrap();

    // Measure shim execution time
    let mut total_time = std::time::Duration::ZERO;
    let iterations = 5;

    for _ in 0..iterations {
        let start = Instant::now();

        let mut cmd = Command::new(&shim_dst);
        cmd.env("KOPI_HOME", test_home.kopi_home());
        cmd.env("HOME", test_home.path());
        cmd.arg("--version");

        let output = cmd.output().expect("Failed to execute shim");
        let elapsed = start.elapsed();

        if output.status.success() {
            total_time += elapsed;
        }
    }

    let avg_time = total_time / iterations as u32;
    println!("Average shim execution time: {avg_time:?}");

    // Assert that average execution time is less than 50ms
    assert!(
        avg_time.as_millis() < 50,
        "Shim execution time {avg_time:?} exceeds 50ms threshold"
    );
}

#[test]
fn test_error_handling_invalid_structure() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_jdk = temp_dir.path().join("invalid-jdk");
    fs::create_dir_all(&invalid_jdk).unwrap();

    // Create an invalid structure (no bin directory)
    fs::write(invalid_jdk.join("README.txt"), "This is not a JDK").unwrap();

    // Test that detection fails appropriately
    let result = detect_jdk_root(&invalid_jdk);
    assert!(result.is_err());
}

#[test]
fn test_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let guard = TestHomeGuard::new();
    let test_home_guard = guard.setup_kopi_structure();
    let test_home = Arc::new(test_home_guard.path().to_path_buf());
    let kopi_home = test_home.join(".kopi");
    let config = Arc::new(KopiConfig::new(kopi_home).unwrap());
    let jdks_dir = config.jdks_dir().unwrap();

    // Create multiple JDKs
    for i in 0..3 {
        let jdk_path = jdks_dir.join(format!("test-jdk-{i}"));
        create_mock_jdk_structure(&jdk_path, "direct");
    }

    // Spawn multiple threads to access JDKs concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let config_clone = Arc::clone(&config);
        let handle = thread::spawn(move || {
            // Each thread lists JDKs multiple times
            for _ in 0..10 {
                let jdks_dir = config_clone.jdks_dir().unwrap();
                let jdks = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
                assert!(jdks.len() >= 3);

                // Resolve paths for each JDK
                for jdk in &jdks {
                    let _java_home = jdk.resolve_java_home();
                    let _bin_path = jdk.resolve_bin_path();
                }
            }
            println!("Thread {i} completed");
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}
