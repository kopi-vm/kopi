mod common;
use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.env("HOME", kopi_home.parent().unwrap());
    // Change to home directory to avoid picking up project's .kopi-version
    cmd.current_dir(kopi_home.parent().unwrap());
    // Clear KOPI_JAVA_VERSION to prevent it from overriding test setups
    cmd.env_remove("KOPI_JAVA_VERSION");
    cmd
}

/// Test basic env command with bash shell (default)
#[test]
fn test_env_basic_bash() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with fish shell
#[test]
fn test_env_fish_shell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("corretto-17.0.5");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "corretto@17.0.5").unwrap();

    // Test env command with fish shell
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("fish");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "set -gx JAVA_HOME \"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with PowerShell
#[test]
fn test_env_powershell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("zulu-11.0.20");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "zulu@11.0.20").unwrap();

    // Test env command with PowerShell
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("powershell");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "$env:JAVA_HOME = \"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with Windows CMD
#[test]
fn test_env_cmd() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("microsoft-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "microsoft@21.0.1").unwrap();

    // Test env command with CMD
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("cmd");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "set JAVA_HOME={}",
            jdk_path.display()
        )));
}

/// Test env command without export flag
#[test]
fn test_env_no_export() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-17.0.8");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@17.0.8").unwrap();

    // Test env command without export
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--export=false");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "JAVA_HOME=\"{}\"",
            jdk_path.display()
        )))
        .stdout(predicate::str::contains("export").not());
}

/// Test env command with explicit version
#[test]
fn test_env_explicit_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create multiple JDK installations
    let jdk17_path = kopi_home.join("jdks").join("temurin-17.0.8");
    let jdk21_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk17_path).unwrap();
    fs::create_dir_all(&jdk21_path).unwrap();

    // Set global version to 17
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@17.0.8").unwrap();

    // Test env command with explicit version (should use 21)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("temurin@21.0.1");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk21_path.display()
        )));
}

/// Test env command with .kopi-version file
#[test]
fn test_env_kopi_version_file() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("graalvm-22.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Create a project directory with .kopi-version
    let project_dir = test_home.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join(".kopi-version"), "graalvm@22.0.1").unwrap();

    // Test env command from project directory
    let mut cmd = get_test_command(&kopi_home);
    cmd.current_dir(&project_dir);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with quiet flag
#[test]
fn test_env_quiet_flag() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with quiet flag
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--quiet");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )))
        .stderr(predicate::str::is_empty());
}

/// Test env command with JDK not installed
#[test]
fn test_env_jdk_not_installed() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Set a global version without installing the JDK
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert().failure().stderr(predicate::str::contains(
        "JDK temurin@21.0.1 is not installed",
    ));
}

/// Test env command with no version configured
#[test]
fn test_env_no_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test env command without any version configured
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No Java version configured"));
}

/// Test env command with path containing spaces
#[test]
fn test_env_path_with_spaces() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a JDK path with spaces (simulated)
    let jdk_path = kopi_home.join("jdks").join("temurin with spaces-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin with spaces@21.0.1").unwrap();

    // Test env command handles spaces correctly
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with invalid shell name
#[test]
fn test_env_invalid_shell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test env command with invalid shell
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("invalid-shell");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Unknown shell"));
}

/// Test shell auto-detection fallback
#[test]
fn test_env_shell_detection() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with different SHELL env vars
    let shells = vec![
        ("bash", "export JAVA_HOME="),
        ("zsh", "export JAVA_HOME="),
        ("fish", "set -gx JAVA_HOME"),
        ("pwsh", "$env:JAVA_HOME ="),
    ];

    for (shell_name, expected_prefix) in shells {
        let mut cmd = get_test_command(&kopi_home);
        cmd.env("SHELL", format!("/bin/{}", shell_name));
        cmd.arg("env");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains(expected_prefix));
    }
}

/// Test env command with malformed version file
#[test]
fn test_env_malformed_version_file() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a malformed global version file
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "invalid@@version").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid version format"));
}

/// Test env command with environment variable override
#[test]
fn test_env_with_kopi_java_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create two JDK installations
    let jdk17_path = kopi_home.join("jdks").join("temurin-17.0.8");
    let jdk21_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk17_path).unwrap();
    fs::create_dir_all(&jdk21_path).unwrap();

    // Set global version to 17
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@17.0.8").unwrap();

    // Test with KOPI_JAVA_VERSION environment variable (should override global)
    let mut cmd = get_test_command(&kopi_home);
    cmd.env("KOPI_JAVA_VERSION", "temurin@21.0.1");
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk21_path.display()
        )));
}

/// Test env command with missing parent directories
#[test]
fn test_env_version_resolution_hierarchy() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a JDK installation
    let jdk_path = kopi_home.join("jdks").join("zulu-11.0.20");
    fs::create_dir_all(&jdk_path).unwrap();

    // Create nested project structure
    let project_root = test_home.path().join("project");
    let sub_dir = project_root.join("src").join("main").join("java");
    fs::create_dir_all(&sub_dir).unwrap();

    // Create .kopi-version in project root
    fs::write(project_root.join(".kopi-version"), "zulu@11.0.20").unwrap();

    // Test from subdirectory (should find parent .kopi-version)
    let mut cmd = get_test_command(&kopi_home);
    cmd.current_dir(&sub_dir);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_path.display()
        )));
}

/// Test env command with multiple matching JDKs (should use latest)
#[test]
fn test_env_multiple_matching_jdks() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create multiple JDK installations with same major version
    let jdk_old_path = kopi_home.join("jdks").join("temurin-21.0.1");
    let jdk_new_path = kopi_home.join("jdks").join("temurin-21.0.2");
    fs::create_dir_all(&jdk_old_path).unwrap();
    fs::create_dir_all(&jdk_new_path).unwrap();

    // Set a global version with just major version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21").unwrap();

    // Test env command (should use latest version)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "export JAVA_HOME=\"{}\"",
            jdk_new_path.display()
        )));
}

/// Test env command with stderr message suppression
#[test]
fn test_env_stderr_messages() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test without quiet flag (should show help message)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("eval \"$(kopi env)\""));

    // Test with quiet flag (should not show help message)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--quiet");
    cmd.assert().success().stderr(predicate::str::is_empty());
}
