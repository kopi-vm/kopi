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

mod common;
use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use serial_test::serial;
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
    // Also remove any shell-specific environment variables that might affect version detection
    cmd.env_remove("SHELL");
    cmd
}

// Helper function to setup test environment that prevents parent directory lookups
fn setup_test_environment(test_home: &TestHomeGuard, version: &str) {
    // Create a .kopi-version file in the test directory root to prevent
    // version resolution from walking up to parent directories
    let kopi_version_file = test_home.path().join(".kopi-version");
    fs::write(&kopi_version_file, version).unwrap();
}

/// Test basic env command with bash shell (Unix-like systems)
#[test]
#[serial]
#[cfg(not(target_os = "windows"))]
fn test_env_basic_bash() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21.0.1");

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export JAVA_HOME="))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test basic env command with platform default shell
#[test]
#[serial]
fn test_env_platform_default() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21.0.1");

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with platform-appropriate default shell
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env");

    // Don't specify shell, let it auto-detect
    // The output format will depend on the detected shell
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test env command with fish shell (Unix-like systems)
#[test]
#[serial]
#[cfg(not(target_os = "windows"))]
fn test_env_fish_shell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "corretto@17.0.5");

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
        .stdout(predicate::str::contains("set -gx JAVA_HOME"))
        .stdout(predicate::str::contains("corretto-17.0.5"));
}

/// Test env command with PowerShell (Windows)
#[test]
#[serial]
#[cfg(target_os = "windows")]
fn test_env_powershell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "zulu@11.0.20");

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
        .stdout(predicate::str::contains("$env:JAVA_HOME ="))
        .stdout(predicate::str::contains("zulu-11.0.20"));
}

/// Test env command with Windows CMD (Windows)
#[test]
#[serial]
#[cfg(target_os = "windows")]
fn test_env_cmd() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "microsoft@21.0.1");

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
        .stdout(predicate::str::contains("set JAVA_HOME="))
        .stdout(predicate::str::contains("microsoft-21.0.1"));
}

/// Test env command without export flag (Unix-like systems)
#[test]
#[serial]
#[cfg(not(target_os = "windows"))]
fn test_env_no_export() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@17.0.8");

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-17.0.8");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@17.0.8").unwrap();

    // Test env command without export
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .arg("--shell")
        .arg("bash")
        .arg("--export")
        .arg("false");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME="))
        .stdout(predicate::str::contains("temurin-17.0.8"))
        .stdout(predicate::str::contains("export").not());
}

/// Test env command with explicit version
#[test]
#[serial]
fn test_env_explicit_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@17.0.8");

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
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        })
        .arg("temurin@21.0.1");

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test env command with .kopi-version file
#[test]
#[serial]
fn test_env_kopi_version_file() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // No need to setup test environment as this test creates its own .kopi-version

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
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("graalvm-22.0.1"));
}

/// Test env command with JDK not installed
#[test]
#[serial]
fn test_env_jdk_not_installed() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21.0.1");

    // Set a global version without installing the JDK
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });
    cmd.assert().failure().stderr(predicate::str::contains(
        "JDK 'temurin@21.0.1' is not installed",
    ));
}

/// Test env command with no version configured
#[test]
#[serial]
fn test_env_no_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // For this test, we need to ensure no version is configured anywhere
    // Create a temporary directory outside of the test hierarchy
    let temp_dir = tempfile::TempDir::new().unwrap();
    let isolated_dir = temp_dir.path().join("isolated");
    fs::create_dir_all(&isolated_dir).unwrap();

    // Test env command without any version configured
    let mut cmd = get_test_command(&kopi_home);
    cmd.current_dir(&isolated_dir);
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });
    cmd.assert().failure().stderr(predicate::str::contains(
        "No JDK configured for current project",
    ));
}

/// Test env command with path containing spaces
#[test]
#[serial]
fn test_env_path_with_spaces() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin with spaces@21.0.1");

    // Create a JDK path with spaces (simulated)
    let jdk_path = kopi_home.join("jdks").join("temurin with spaces-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin with spaces@21.0.1").unwrap();

    // Test env command handles spaces correctly
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("temurin with spaces-21.0.1"));
}

/// Test env command with invalid shell name
#[test]
#[serial]
fn test_env_invalid_shell() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21.0.1");

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version so we don't get "No JDK configured" error
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with invalid shell
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env").arg("--shell").arg("invalid-shell");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Shell 'invalid-shell' is not supported",
    ));
}

/// Test shell auto-detection fallback
#[test]
#[serial]
fn test_env_shell_detection() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21.0.1");

    // Create a mock JDK installation
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&jdk_path).unwrap();

    // Set a global version
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "temurin@21.0.1").unwrap();

    // Test env command with platform-appropriate shells
    #[cfg(not(target_os = "windows"))]
    let shells = vec![("bash", "export JAVA_HOME="), ("zsh", "export JAVA_HOME=")];

    #[cfg(target_os = "windows")]
    let shells = vec![("powershell", "$env:JAVA_HOME ="), ("cmd", "set JAVA_HOME")];

    for (shell_name, expected_prefix) in shells {
        let mut cmd = get_test_command(&kopi_home);
        cmd.arg("env").arg("--shell").arg(shell_name);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains(expected_prefix));
    }
}

/// Test env command with malformed version file
#[test]
#[serial]
fn test_env_malformed_version_file() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "invalid@@version");

    // Create a malformed global version file
    let global_version_file = kopi_home.join("version");
    fs::write(&global_version_file, "invalid@@version").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });
    cmd.assert().failure().stderr(predicate::str::contains(
        "Invalid configuration: Unknown package type: invalid",
    ));
}

/// Test env command with environment variable override
#[test]
#[serial]
fn test_env_with_kopi_java_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@17.0.8");

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
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test env command with missing parent directories
#[test]
#[serial]
fn test_env_version_resolution_hierarchy() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // No need to setup test environment as this test creates its own .kopi-version in subdirectory

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
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("zulu-11.0.20"));
}

/// Test env command with multiple matching JDKs (should use latest)
#[test]
#[serial]
fn test_env_multiple_matching_jdks() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Setup test environment to prevent parent directory lookups
    setup_test_environment(&test_home, "temurin@21");

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
    cmd.arg("env")
        .arg("--shell")
        .arg(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

    #[cfg(target_os = "windows")]
    let expected = "set JAVA_HOME=";
    #[cfg(not(target_os = "windows"))]
    let expected = "export JAVA_HOME=";

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected))
        .stdout(predicate::str::contains("temurin-21.0.2"));
}
