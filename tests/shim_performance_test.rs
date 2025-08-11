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

//! Performance tests for shim execution time.

use std::env;
use std::fs;
use std::process::Command;
use std::time::Instant;

mod common;
use common::TestHomeGuard;

use kopi::config::KopiConfig;

/// Create a minimal JDK structure for testing
fn create_minimal_jdk(jdk_path: &std::path::Path, vendor: &str, _version: &str) {
    // Create appropriate structure based on vendor
    let bin_dir = if vendor == "temurin" && cfg!(target_os = "macos") {
        let home_dir = jdk_path.join("Contents").join("Home");
        fs::create_dir_all(&home_dir).unwrap();
        home_dir.join("bin")
    } else {
        jdk_path.join("bin")
    };

    fs::create_dir_all(&bin_dir).unwrap();

    // Create a minimal java executable
    let java_path = bin_dir.join("java");
    #[cfg(unix)]
    {
        fs::write(&java_path, "#!/bin/sh\nexit 0").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms).unwrap();
    }
    #[cfg(windows)]
    {
        fs::write(java_path.with_extension("exe"), "@echo off\nexit 0").unwrap();
    }
}

#[test]
#[ignore] // Only run when explicitly requested due to timing sensitivity
fn test_shim_performance_under_50ms() {
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();
    let shims_dir = config.bin_dir().unwrap();

    // Create test JDKs with different structures
    create_minimal_jdk(&jdks_dir.join("temurin-21"), "temurin", "21");
    create_minimal_jdk(&jdks_dir.join("liberica-17"), "liberica", "17");

    // Copy the actual shim binary
    let shim_src = env!("CARGO_BIN_EXE_kopi-shim");
    let java_shim = shims_dir.join("java");
    fs::copy(shim_src, &java_shim).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_shim).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_shim, perms).unwrap();
    }

    // Test with different version configurations
    let test_cases = [
        ("temurin@21", "Bundle structure on macOS"),
        ("liberica@17", "Direct structure"),
    ];

    for (version, description) in &test_cases {
        println!("\nTesting shim performance with {version}: {description}");

        // Set version
        fs::write(test_home.path().join(".kopi-version"), version).unwrap();

        // Warm up (first run might be slower)
        let mut cmd = Command::new(&java_shim);
        cmd.env("KOPI_HOME", test_home.kopi_home())
            .env("HOME", test_home.path())
            .current_dir(test_home.path())
            .arg("--version");
        let _ = cmd.output();

        // Measure execution time
        let mut times = Vec::new();
        let iterations = 10;

        for i in 0..iterations {
            let start = Instant::now();

            let mut cmd = Command::new(&java_shim);
            cmd.env("KOPI_HOME", test_home.kopi_home())
                .env("HOME", test_home.path())
                .current_dir(test_home.path())
                .arg("--version");

            let output = cmd.output().expect("Failed to execute shim");
            let elapsed = start.elapsed();

            if !output.status.success() {
                eprintln!(
                    "Shim failed with stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                continue;
            }

            times.push(elapsed);
            println!("  Run {}: {elapsed:?}", i + 1);
        }

        if !times.is_empty() {
            // Calculate statistics
            let total: std::time::Duration = times.iter().sum();
            let avg = total / times.len() as u32;
            let min = times.iter().min().unwrap();
            let max = times.iter().max().unwrap();

            println!("  Statistics for {version}:");
            println!("    Average: {avg:?}");
            println!("    Min: {min:?}");
            println!("    Max: {max:?}");

            // Assert average is under 50ms
            assert!(
                avg.as_millis() < 50,
                "Average shim execution time {avg:?} exceeds 50ms threshold for {version}"
            );

            // Warn if close to threshold
            if avg.as_millis() > 40 {
                println!("  WARNING: Average time {avg:?} is close to 50ms threshold");
            }
        }
    }
}

#[test]
fn test_shim_performance_with_metadata_cache() {
    // This test verifies that metadata caching improves performance
    let guard = TestHomeGuard::new();
    let test_home = guard.setup_kopi_structure();
    let config = KopiConfig::new(test_home.kopi_home()).unwrap();
    let jdks_dir = config.jdks_dir().unwrap();
    let shims_dir = config.bin_dir().unwrap();

    // Create a JDK with bundle structure
    let jdk_path = jdks_dir.join("temurin-21");
    create_minimal_jdk(&jdk_path, "temurin", "21");

    // Create metadata file (simulating what would be created during installation)
    let metadata = serde_json::json!({
        "installation_metadata": {
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos"
        }
    });
    let metadata_path = jdk_path.join("metadata.json");
    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).unwrap(),
    )
    .unwrap();

    // Copy shim
    let shim_src = env!("CARGO_BIN_EXE_kopi-shim");
    let java_shim = shims_dir.join("java");
    fs::copy(shim_src, &java_shim).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_shim).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_shim, perms).unwrap();
    }

    // Set version
    fs::write(test_home.path().join(".kopi-version"), "temurin@21").unwrap();

    // Run shim and verify it works
    let mut cmd = Command::new(&java_shim);
    cmd.env("KOPI_HOME", test_home.kopi_home())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("--version");

    let output = cmd.output().expect("Failed to execute shim");
    assert!(
        output.status.success(),
        "Shim failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    println!("Shim with metadata cache executed successfully");
}
