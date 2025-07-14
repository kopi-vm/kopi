use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_kopi_with_home(args: &[&str], kopi_home: &str) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);
    cmd.env("KOPI_HOME", kopi_home);

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

fn run_kopi(args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kopi"));
    cmd.args(args);

    let output = cmd.output().expect("Failed to execute kopi");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status.success())
}

fn create_mock_jdk(kopi_home: &Path, distribution: &str, version: &str) {
    let jdk_path = kopi_home
        .join("jdks")
        .join(format!("{}-{}", distribution, version));

    // Create directory structure
    fs::create_dir_all(&jdk_path).unwrap();
    fs::create_dir_all(jdk_path.join("bin")).unwrap();
    fs::create_dir_all(jdk_path.join("lib")).unwrap();

    // Create dummy files
    fs::write(
        jdk_path.join("bin").join("java"),
        "#!/bin/sh\necho mock java\n",
    )
    .unwrap();
    fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21.0.5\"\n").unwrap();

    // Create metadata file
    let meta_path = jdk_path.with_extension("meta.json");
    let metadata = format!(
        r#"{{
            "distribution": "{distribution}",
            "version": "{version}",
            "installed_at": "2024-01-01T00:00:00Z",
            "source": "foojay_api"
        }}"#
    );
    fs::write(meta_path, metadata).unwrap();
}

#[test]
fn test_uninstall_help() {
    let (stdout, _, success) = run_kopi(&["uninstall", "--help"]);
    assert!(success);
    assert!(stdout.contains("Uninstall a JDK version"));
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--all"));
}

#[test]
fn test_uninstall_aliases() {
    // Test 'u' alias
    let (stdout, _, success) = run_kopi(&["u", "--help"]);
    assert!(success);
    assert!(stdout.contains("Uninstall a JDK version"));

    // Test 'remove' alias
    let (stdout2, _, success2) = run_kopi(&["remove", "--help"]);
    assert!(success2);
    assert!(stdout2.contains("Uninstall a JDK version"));
}

#[test]
fn test_uninstall_not_installed() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Create empty jdks directory
    fs::create_dir_all(temp_dir.path().join("jdks")).unwrap();

    let (_, stderr, success) = run_kopi_with_home(&["uninstall", "temurin@21.0.5+11"], kopi_home);
    assert!(!success);
    assert!(stderr.contains("is not installed"));
}

#[test]
fn test_uninstall_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create a mock JDK
    create_mock_jdk(&kopi_home, "temurin", "21.0.5+11");

    let (stdout, _, success) = run_kopi_with_home(
        &["uninstall", "temurin@21.0.5+11", "--dry-run"],
        kopi_home.to_str().unwrap(),
    );
    assert!(success);
    assert!(stdout.contains("Would uninstall: temurin@21.0.5+11"));
    assert!(stdout.contains("Would free:"));

    // Verify JDK still exists
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.5+11");
    assert!(jdk_path.exists());
}

#[test]
fn test_uninstall_force() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create a mock JDK
    create_mock_jdk(&kopi_home, "corretto", "17.0.13.11.1");

    let (stdout, _, success) = run_kopi_with_home(
        &["uninstall", "corretto@17.0.13.11.1", "--force"],
        kopi_home.to_str().unwrap(),
    );
    assert!(success);
    assert!(stdout.contains("Successfully uninstalled: corretto@17.0.13.11.1"));

    // Verify JDK was removed
    let jdk_path = kopi_home.join("jdks").join("corretto-17.0.13.11.1");
    assert!(!jdk_path.exists());
}

#[test]
fn test_uninstall_multiple_matches_error() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create multiple JDKs with same major version
    create_mock_jdk(&kopi_home, "temurin", "21.0.5+11");
    create_mock_jdk(&kopi_home, "corretto", "21.0.13.11.1");

    let (_, stderr, success) =
        run_kopi_with_home(&["uninstall", "21"], kopi_home.to_str().unwrap());
    assert!(!success);
    assert!(stderr.contains("Multiple JDKs match"));
    assert!(stderr.contains("temurin@21.0.5+11"));
    assert!(stderr.contains("corretto@21.0.13.11.1"));
    assert!(stderr.contains("Please specify which JDK to uninstall"));
}

#[test]
fn test_uninstall_invalid_version_format() {
    let (_, stderr, success) = run_kopi(&["uninstall", "invalid@version@format"]);
    assert!(!success);
    assert!(stderr.contains("Invalid") || stderr.contains("invalid"));
}

#[test]
fn test_uninstall_all_flag() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create multiple versions of same distribution
    create_mock_jdk(&kopi_home, "temurin", "21.0.5+11");
    create_mock_jdk(&kopi_home, "temurin", "17.0.9+9");
    create_mock_jdk(&kopi_home, "corretto", "21.0.13.11.1");

    let (stdout, _, success) = run_kopi_with_home(
        &["uninstall", "temurin", "--all", "--force"],
        kopi_home.to_str().unwrap(),
    );
    assert!(success);
    assert!(
        stdout.contains("Successfully uninstalled 2 JDKs")
            || stdout.contains("Batch uninstall summary")
    );

    // Verify only Temurin JDKs were removed
    assert!(!kopi_home.join("jdks").join("temurin-21.0.5+11").exists());
    assert!(!kopi_home.join("jdks").join("temurin-17.0.9+9").exists());
    assert!(
        kopi_home
            .join("jdks")
            .join("corretto-21.0.13.11.1")
            .exists()
    );
}

#[test]
fn test_uninstall_all_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create multiple versions
    create_mock_jdk(&kopi_home, "zulu", "11.0.25+9");
    create_mock_jdk(&kopi_home, "zulu", "8.0.422+5");

    let (stdout, _, success) = run_kopi_with_home(
        &["uninstall", "zulu", "--all", "--dry-run"],
        kopi_home.to_str().unwrap(),
    );
    assert!(success);
    assert!(stdout.contains("JDKs to be removed:") || stdout.contains("Would uninstall 2 JDK(s)"));
    assert!(stdout.contains("zulu@11.0.25+9"));
    assert!(stdout.contains("zulu@8.0.422+5"));
    assert!(stdout.contains("Total: 2 JDKs"));

    // Verify JDKs still exist
    assert!(kopi_home.join("jdks").join("zulu-11.0.25+9").exists());
    assert!(kopi_home.join("jdks").join("zulu-8.0.422+5").exists());
}

#[test]
fn test_uninstall_with_version_only() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create a single JDK
    create_mock_jdk(&kopi_home, "temurin", "17.0.9+9");

    let (stdout, _, success) =
        run_kopi_with_home(&["uninstall", "17", "--force"], kopi_home.to_str().unwrap());
    assert!(success);
    assert!(stdout.contains("Successfully uninstalled: temurin@17.0.9+9"));

    // Verify JDK was removed
    assert!(!kopi_home.join("jdks").join("temurin-17.0.9+9").exists());
}

#[test]
fn test_uninstall_exit_codes() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Test JDK not found (exit code 4)
    let output = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(&["uninstall", "nonexistent@1.0.0"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));

    // Test invalid arguments (exit code 2)
    let output2 = Command::new(env!("CARGO_BIN_EXE_kopi"))
        .args(&["uninstall"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .unwrap();
    assert_eq!(output2.status.code(), Some(2));
}
