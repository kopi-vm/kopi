mod common;

use common::TestHomeGuard;
use std::process::Command;

#[test]
fn test_doctor_command_basic() {
    let _guard = TestHomeGuard::new();

    // Run doctor command
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor"])
        .output()
        .expect("Failed to execute kopi doctor");

    // Should complete without panic
    assert!(
        output.status.success()
            || output.status.code() == Some(1)
            || output.status.code() == Some(2)
    );

    // Should output human-readable format
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kopi Doctor Report"));
    assert!(stdout.contains("Summary"));
}

#[test]
fn test_doctor_json_output() {
    let _guard = TestHomeGuard::new();

    // Run doctor command with JSON output
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor", "--json"])
        .output()
        .expect("Failed to execute kopi doctor --json");

    // Should complete successfully or with warning/error
    assert!(
        output.status.success()
            || output.status.code() == Some(1)
            || output.status.code() == Some(2)
    );

    // Should output valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify JSON structure
    assert!(json["version"].is_string());
    assert!(json["timestamp"].is_string());
    assert!(json["summary"].is_object());
    assert!(json["categories"].is_array());
}

#[test]
fn test_doctor_verbose_flag() {
    let _guard = TestHomeGuard::new();

    // Run doctor command with verbose flag
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor", "--verbose"])
        .output()
        .expect("Failed to execute kopi doctor --verbose");

    // Should complete without panic
    assert!(
        output.status.success()
            || output.status.code() == Some(1)
            || output.status.code() == Some(2)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kopi Doctor Report"));
}

#[test]
fn test_doctor_invalid_category() {
    let _guard = TestHomeGuard::new();

    // Run doctor command with invalid category
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor", "--check", "invalid_category"])
        .output()
        .expect("Failed to execute kopi doctor with invalid category");

    // Should fail with error
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid check category"));
    assert!(stderr.contains("Valid categories"));
}

#[test]
fn test_doctor_specific_category() {
    let _guard = TestHomeGuard::new();

    // Test each valid category
    let categories = [
        "installation",
        "shell",
        "jdks",
        "permissions",
        "network",
        "cache",
    ];

    for category in &categories {
        let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
            .args(["doctor", "--check", category])
            .output()
            .expect(&format!(
                "Failed to execute kopi doctor --check {}",
                category
            ));

        // Should complete without panic
        assert!(
            output.status.success()
                || output.status.code() == Some(1)
                || output.status.code() == Some(2)
        );

        // Output should still have proper format
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Kopi Doctor Report"));
    }
}

#[test]
fn test_doctor_exit_codes() {
    let _guard = TestHomeGuard::new();

    // In Phase 1, with no actual checks, it should exit with 0
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor"])
        .output()
        .expect("Failed to execute kopi doctor");

    // Should exit with code 0 since no checks are implemented yet
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_doctor_json_exit_code_field() {
    let _guard = TestHomeGuard::new();

    // Run doctor command with JSON output
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(["doctor", "--json"])
        .output()
        .expect("Failed to execute kopi doctor --json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // JSON should include exit_code in summary
    assert!(json["summary"]["exit_code"].is_number());
    assert_eq!(json["summary"]["exit_code"], 0);
}
