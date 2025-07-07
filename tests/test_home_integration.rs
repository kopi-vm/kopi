mod common;
use assert_cmd::prelude::*;
use common::TestHomeGuard;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_install_with_isolated_home() {
    // Create isolated test environment
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Run list command in empty environment
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &kopi_home).arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Listing installed JDK versions"));
}

#[test]
fn test_cache_in_isolated_home() {
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Run cache info command
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &kopi_home).arg("cache").arg("info");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No cache found"));
}

#[test]
fn test_multiple_test_homes_isolation() {
    use std::fs;

    // Create two separate test environments
    let test_home1 = TestHomeGuard::new();
    let test_home1 = test_home1.setup_kopi_structure();
    let kopi_home1 = test_home1.kopi_home();

    let test_home2 = TestHomeGuard::new();
    let test_home2 = test_home2.setup_kopi_structure();
    let kopi_home2 = test_home2.kopi_home();

    // Verify they are different paths
    assert_ne!(kopi_home1, kopi_home2);

    // Create unique content in each environment
    let test_file1 = kopi_home1.join("test_env1.txt");
    let test_file2 = kopi_home2.join("test_env2.txt");
    fs::write(&test_file1, "Environment 1 data").unwrap();
    fs::write(&test_file2, "Environment 2 data").unwrap();

    // Create different config files
    let config1 = kopi_home1.join("config.toml");
    let config2 = kopi_home2.join("config.toml");
    fs::write(&config1, "[global]\ndefault = \"temurin@11\"").unwrap();
    fs::write(&config2, "[global]\ndefault = \"temurin@17\"").unwrap();

    // Create mock JDK installations
    let jdk_dir1 = kopi_home1.join("jdks").join("temurin-11.0.20");
    let jdk_dir2 = kopi_home2.join("jdks").join("temurin-17.0.8");
    fs::create_dir_all(&jdk_dir1).unwrap();
    fs::create_dir_all(&jdk_dir2).unwrap();

    // Verify files exist only in their respective environments
    assert!(test_file1.exists(), "File should exist in environment 1");
    assert!(test_file2.exists(), "File should exist in environment 2");
    assert!(
        !kopi_home1.join("test_env2.txt").exists(),
        "Environment 2 file should not exist in environment 1"
    );
    assert!(
        !kopi_home2.join("test_env1.txt").exists(),
        "Environment 1 file should not exist in environment 2"
    );

    // Verify different configs are isolated
    let config1_content = fs::read_to_string(&config1).unwrap();
    let config2_content = fs::read_to_string(&config2).unwrap();
    assert!(
        config1_content.contains("temurin@11"),
        "Config 1 should contain temurin@11"
    );
    assert!(
        config2_content.contains("temurin@17"),
        "Config 2 should contain temurin@17"
    );
    assert!(
        !config1_content.contains("temurin@17"),
        "Config 1 should not contain temurin@17"
    );
    assert!(
        !config2_content.contains("temurin@11"),
        "Config 2 should not contain temurin@11"
    );

    // Verify JDK installations are isolated
    assert!(jdk_dir1.exists(), "JDK should exist in environment 1");
    assert!(jdk_dir2.exists(), "JDK should exist in environment 2");
    assert!(
        !kopi_home1.join("jdks").join("temurin-17.0.8").exists(),
        "Environment 2 JDK should not exist in environment 1"
    );
    assert!(
        !kopi_home2.join("jdks").join("temurin-11.0.20").exists(),
        "Environment 1 JDK should not exist in environment 2"
    );

    // Since list command is not implemented yet, verify JDK directories directly
    // This still proves environment isolation
    let jdks1: Vec<_> = fs::read_dir(kopi_home1.join("jdks"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    let jdks2: Vec<_> = fs::read_dir(kopi_home2.join("jdks"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    assert_eq!(
        jdks1,
        vec!["temurin-11.0.20"],
        "Environment 1 should have exactly one JDK"
    );
    assert_eq!(
        jdks2,
        vec!["temurin-17.0.8"],
        "Environment 2 should have exactly one JDK"
    );
}

#[test]
fn test_cleanup_verification() {
    use std::path::PathBuf;

    // Capture the path before the guard is dropped
    let test_path: PathBuf = {
        let test_home = TestHomeGuard::new();
        let path = test_home.path().to_path_buf();

        // Verify it exists while guard is alive
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("target/home"));

        // Create some files
        let test_home = test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();
        std::fs::write(kopi_home.join("test.txt"), "test content").unwrap();

        path
    }; // Guard is dropped here

    // Verify the directory has been cleaned up
    assert!(
        !test_path.exists(),
        "Test directory should be cleaned up after guard is dropped"
    );
}

#[test]
fn test_user_home_directory_isolation() {
    use std::fs;

    // Get the actual user home directory
    let user_home = dirs::home_dir().expect("Failed to get user home directory");
    let user_kopi_dir = user_home.join(".kopi");

    // Create a marker file in user's .kopi if it exists (to verify it's not modified)
    let user_marker = user_kopi_dir.join("test_marker_do_not_delete.tmp");
    let user_kopi_exists = user_kopi_dir.exists();
    let marker_content = "This file verifies user's .kopi is not modified by tests";

    if user_kopi_exists {
        // Create a temporary marker to ensure we don't modify user's directory
        let _ = fs::write(&user_marker, marker_content);
    }

    // Create isolated test environment
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let test_kopi_home = test_home.kopi_home();

    // Verify test environment is not in user's home
    assert!(
        !test_kopi_home.starts_with(&user_home),
        "Test environment should not be in user's home directory"
    );
    assert!(
        test_kopi_home.to_string_lossy().contains("target/home"),
        "Test environment should be in target/home directory"
    );

    // Create test data in isolated environment
    fs::write(test_kopi_home.join("test_isolated.txt"), "Test data").unwrap();

    // Run command with explicit KOPI_HOME
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &test_kopi_home).arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Listing installed JDK versions"));

    // Verify user's .kopi was not affected
    if user_kopi_exists {
        assert!(
            !user_kopi_dir.join("test_isolated.txt").exists(),
            "Test file should not exist in user's .kopi directory"
        );

        // Verify marker file is unchanged
        if user_marker.exists() {
            let content = fs::read_to_string(&user_marker).unwrap();
            assert_eq!(
                content, marker_content,
                "User's .kopi marker file should not be modified"
            );

            // Clean up marker file
            let _ = fs::remove_file(&user_marker);
        }
    }
}

#[test]
fn test_environment_variables_isolation() {
    use std::env;
    use std::fs;

    // Save current KOPI_HOME if it exists
    let original_kopi_home = env::var("KOPI_HOME").ok();

    // Create two test environments
    let test_home1 = TestHomeGuard::new();
    let test_home1 = test_home1.setup_kopi_structure();
    let kopi_home1 = test_home1.kopi_home();

    let test_home2 = TestHomeGuard::new();
    let test_home2 = test_home2.setup_kopi_structure();
    let kopi_home2 = test_home2.kopi_home();

    // Create different content in each
    fs::write(kopi_home1.join("env1.txt"), "Environment 1").unwrap();
    fs::write(kopi_home2.join("env2.txt"), "Environment 2").unwrap();

    // Test that commands respect KOPI_HOME environment variable
    let mut cmd1 = Command::cargo_bin("kopi").unwrap();
    cmd1.env("KOPI_HOME", &kopi_home1).arg("cache").arg("info");

    let mut cmd2 = Command::cargo_bin("kopi").unwrap();
    cmd2.env("KOPI_HOME", &kopi_home2).arg("cache").arg("info");

    // Both should succeed independently
    cmd1.assert().success();
    cmd2.assert().success();

    // Verify original KOPI_HOME is unchanged
    match (original_kopi_home, env::var("KOPI_HOME").ok()) {
        (Some(original), Some(current)) => {
            assert_eq!(
                original, current,
                "KOPI_HOME environment variable should not be permanently modified"
            );
        }
        (None, None) => {
            // Both were unset, which is correct
        }
        _ => {
            // Environment variable state changed, which shouldn't happen
            panic!("KOPI_HOME environment variable state was modified by tests");
        }
    }
}

#[test]
fn test_concurrent_environment_access() {
    use std::fs;
    use std::thread;

    // Create multiple test environments
    let num_environments = 3;
    let mut handles = vec![];

    for i in 0..num_environments {
        let handle = thread::spawn(move || {
            // Each thread creates its own isolated environment
            let test_home = TestHomeGuard::new();
            let test_home = test_home.setup_kopi_structure();
            let kopi_home = test_home.kopi_home();

            // Create unique content for this environment
            let unique_file = format!("thread_{i}_data.txt");
            let unique_content = format!("Data from thread {i}");
            fs::write(kopi_home.join(&unique_file), &unique_content).unwrap();

            // Create a mock JDK specific to this thread
            let jdk_name = format!("temurin-{}.0.0", 11 + i);
            let jdk_dir = kopi_home.join("jdks").join(&jdk_name);
            fs::create_dir_all(&jdk_dir).unwrap();

            // Since list command is not implemented, verify JDK directory exists
            let jdk_path = kopi_home.join("jdks").join(&jdk_name);
            assert!(
                jdk_path.exists(),
                "Thread {i} should have its JDK directory: {jdk_name}"
            );

            // Verify only this thread's JDK exists
            let jdks: Vec<_> = fs::read_dir(kopi_home.join("jdks"))
                .unwrap()
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect();

            assert_eq!(jdks.len(), 1, "Thread {i} should have exactly one JDK");
            assert_eq!(jdks[0], jdk_name, "Thread {i} should only have its own JDK");
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All threads have completed successfully, proving isolation during concurrent access
}

#[test]
fn test_simultaneous_operations_different_environments() {
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::thread;

    // Create a barrier to synchronize thread starts
    let barrier = Arc::new(Barrier::new(2));

    // Thread 1: Creates and modifies environment 1
    let barrier_clone = barrier.clone();
    let handle1 = thread::spawn(move || {
        let test_home = TestHomeGuard::new();
        let test_home = test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        // Wait for both threads to be ready
        barrier_clone.wait();

        // Perform operations
        for i in 0..5 {
            let file_name = format!("env1_file_{i}.txt");
            fs::write(
                kopi_home.join(&file_name),
                format!("Environment 1 - File {i}"),
            )
            .unwrap();

            // Simulate JDK operation
            let jdk_dir = kopi_home.join("jdks").join(format!("env1-jdk-{i}"));
            fs::create_dir_all(&jdk_dir).unwrap();
        }

        // Verify all files exist
        for i in 0..5 {
            assert!(kopi_home.join(format!("env1_file_{i}.txt")).exists());
        }

        kopi_home
    });

    // Thread 2: Creates and modifies environment 2
    let barrier_clone = barrier.clone();
    let handle2 = thread::spawn(move || {
        let test_home = TestHomeGuard::new();
        let test_home = test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        // Wait for both threads to be ready
        barrier_clone.wait();

        // Perform operations
        for i in 0..5 {
            let file_name = format!("env2_file_{i}.txt");
            fs::write(
                kopi_home.join(&file_name),
                format!("Environment 2 - File {i}"),
            )
            .unwrap();

            // Simulate JDK operation
            let jdk_dir = kopi_home.join("jdks").join(format!("env2-jdk-{i}"));
            fs::create_dir_all(&jdk_dir).unwrap();
        }

        // Verify all files exist
        for i in 0..5 {
            assert!(kopi_home.join(format!("env2_file_{i}.txt")).exists());
        }

        kopi_home
    });

    // Wait for both threads to complete
    let kopi_home1 = handle1.join().expect("Thread 1 panicked");
    let kopi_home2 = handle2.join().expect("Thread 2 panicked");

    // Verify environments remained isolated
    for i in 0..5 {
        // Environment 1 should not have environment 2's files
        assert!(
            !kopi_home1.join(format!("env2_file_{i}.txt")).exists(),
            "Environment 1 should not contain environment 2's files"
        );

        // Environment 2 should not have environment 1's files
        assert!(
            !kopi_home2.join(format!("env1_file_{i}.txt")).exists(),
            "Environment 2 should not contain environment 1's files"
        );
    }
}
