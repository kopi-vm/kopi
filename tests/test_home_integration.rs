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
        .stdout(predicate::str::contains("Cache"));
}

#[test]
fn test_multiple_test_homes_isolation() {
    // Create two separate test environments
    let test_home1 = TestHomeGuard::new();
    let test_home1 = test_home1.setup_kopi_structure();
    let kopi_home1 = test_home1.kopi_home();

    let test_home2 = TestHomeGuard::new();
    let test_home2 = test_home2.setup_kopi_structure();
    let kopi_home2 = test_home2.kopi_home();

    // Verify they are different paths
    assert_ne!(kopi_home1, kopi_home2);

    // Both should be empty initially
    let mut cmd1 = Command::cargo_bin("kopi").unwrap();
    cmd1.env("KOPI_HOME", &kopi_home1).arg("list");

    let mut cmd2 = Command::cargo_bin("kopi").unwrap();
    cmd2.env("KOPI_HOME", &kopi_home2).arg("list");

    cmd1.assert()
        .success()
        .stdout(predicate::str::contains("Listing installed JDK versions"));

    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("Listing installed JDK versions"));
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
