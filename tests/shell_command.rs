use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

/// Tests for the shell/use command
mod shell_command_tests {
    use super::*;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        // set_var is unsafe in newer Rust versions
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }
        temp_dir
    }

    #[test]
    #[serial]
    #[ignore = "Requires JDK installation"]
    fn test_shell_command_with_installed_jdk() {
        let _temp_dir = setup_test_env();

        // First install a JDK
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["install", "temurin@21"])
            .assert()
            .success();

        // Test shell command - this would normally launch a new shell
        // but in tests we can't easily verify that, so we just check it doesn't error
        let result = Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "21"])
            .output()
            .unwrap();

        // The command should execute without error
        assert!(result.status.success() || result.status.code() == Some(0));
    }

    #[test]
    #[serial]
    fn test_shell_command_with_uninstalled_jdk() {
        let _temp_dir = setup_test_env();

        // Try to use an uninstalled JDK version
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "99.99.99"])
            .env("KOPI_AUTO_INSTALL__PROMPT", "false")
            .assert()
            .failure()
            .stderr(predicate::str::contains("is not available"));
    }

    #[test]
    #[serial]
    fn test_use_alias() {
        let _temp_dir = setup_test_env();

        // Test that 'use' works as an alias for 'shell'
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["use", "99.0.1"])
            .env("KOPI_AUTO_INSTALL__PROMPT", "false")
            .assert()
            .failure()
            .stderr(predicate::str::contains("is not available"));
    }

    #[test]
    #[serial]
    fn test_shell_override_option() {
        let _temp_dir = setup_test_env();

        // Test shell override with invalid shell
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "21", "--shell", "nonexistent_shell"])
            .env("KOPI_AUTO_INSTALL__ENABLED", "false")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not found").or(predicate::str::contains("not installed")),
            );
    }

    #[test]
    #[serial]
    fn test_shell_command_invalid_version() {
        let _temp_dir = setup_test_env();

        // Test with invalid version format
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "invalid-version"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid version"));
    }

    #[test]
    #[serial]
    fn test_shell_command_with_distribution() {
        let _temp_dir = setup_test_env();

        // Test with distribution@version format - for shell command,
        // if auto-install is disabled via prompt=false, it still installs
        // This test verifies the command can parse distribution@version format
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "corretto@99"])
            .env("KOPI_AUTO_INSTALL__PROMPT", "false")
            .assert()
            .failure()
            .stderr(predicate::str::contains("is not available"));
    }

    #[test]
    #[serial]
    fn test_shell_help() {
        // Test shell command help
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["shell", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Set JDK version for current shell session",
            ));

        // Test use command help
        Command::cargo_bin("kopi")
            .unwrap()
            .args(["use", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Set JDK version for current shell session",
            ));
    }
}
