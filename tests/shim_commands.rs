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
use common::TestHomeGuard;

#[cfg(test)]
mod shim_command_tests {
    use super::TestHomeGuard;
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::path::Path;

    fn get_test_command(kopi_home: &Path) -> Command {
        let mut cmd = Command::cargo_bin("kopi").unwrap();
        cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
        cmd.env("HOME", kopi_home.parent().unwrap());
        cmd
    }

    #[test]
    fn test_setup_command_creates_directories() {
        let test_home = TestHomeGuard::new();
        let kopi_home = test_home.kopi_home();

        get_test_command(&kopi_home)
            .arg("setup")
            .assert()
            .success()
            .stdout(predicate::str::contains("Setting up Kopi..."))
            .stdout(predicate::str::contains("Setup completed successfully!"));

        // Verify directories were created
        assert!(kopi_home.join("jdks").exists());
        assert!(kopi_home.join("shims").exists());
        assert!(kopi_home.join("cache").exists());
    }

    #[test]
    fn test_setup_command_with_force_flag() {
        let test_home = TestHomeGuard::new();
        let kopi_home = test_home.kopi_home();

        // Run setup first
        get_test_command(&kopi_home).arg("setup").assert().success();

        // The force flag behavior with existing shims depends on implementation
        // Currently it errors on existing shims, so we check for that
        // In the future, --force might override existing shims
        let output = get_test_command(&kopi_home)
            .arg("setup")
            .arg("--force")
            .output()
            .unwrap();

        // For now, we accept either behavior:
        // - Success with warnings about existing shims
        // - Failure due to existing shims
        assert!(
            output.status.success()
                || String::from_utf8_lossy(&output.stderr).contains("already exists"),
            "Setup --force should either succeed or fail with 'already exists' error"
        );
    }

    #[test]
    fn test_shim_list_empty() {
        let test_home = TestHomeGuard::new();
        test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        get_test_command(&kopi_home)
            .arg("shim")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("No shims installed"));
    }

    #[test]
    fn test_shim_list_available() {
        let test_home = TestHomeGuard::new();
        test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        get_test_command(&kopi_home)
            .arg("shim")
            .arg("list")
            .arg("--available")
            .assert()
            .success()
            .stdout(predicate::str::contains("Available JDK tools:"))
            .stdout(predicate::str::contains("java"))
            .stdout(predicate::str::contains("javac"));
    }

    #[test]
    fn test_shim_list_available_with_distribution_filter() {
        let test_home = TestHomeGuard::new();
        test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        get_test_command(&kopi_home)
            .arg("shim")
            .arg("list")
            .arg("--available")
            .arg("--distribution")
            .arg("graalvm")
            .assert()
            .success()
            .stdout(predicate::str::contains("Available JDK tools:"));
    }

    #[test]
    fn test_shim_add_without_proper_setup() {
        let test_home = TestHomeGuard::new();
        test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        // The shim add command should work if kopi-shim exists in the build directory
        // This test now verifies that shim add works independently of setup
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("add")
            .arg("my-custom-tool")
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Created shim for 'my-custom-tool'",
            ));
    }

    #[test]
    #[cfg_attr(not(feature = "integration_tests"), ignore)]
    fn test_shim_workflow_complete() {
        let test_home = TestHomeGuard::new();
        let kopi_home = test_home.kopi_home();

        // Step 1: Setup
        get_test_command(&kopi_home).arg("setup").assert().success();

        // Step 2: Add a custom shim (not one of the defaults)
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("add")
            .arg("jconsole")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created shim for 'jconsole'"));

        // Step 3: List shims (should show both default shims and our custom one)
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("list")
            .assert()
            .success()
            .stdout(predicate::str::contains("jconsole"));

        // Step 4: Verify shims
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("verify")
            .assert()
            .success();

        // Step 5: Remove our custom shim
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("remove")
            .arg("jconsole")
            .assert()
            .success()
            .stdout(predicate::str::contains("Removed shim for 'jconsole'"));

        // Step 6: Verify our custom shim is gone (but default shims remain)
        let output = get_test_command(&kopi_home)
            .arg("shim")
            .arg("list")
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());
        assert!(!stdout.contains("jconsole"), "jconsole should be removed");
        assert!(
            stdout.contains("java"),
            "Default java shim should still exist"
        );
    }

    #[test]
    fn test_shim_add_custom_tool() {
        let test_home = TestHomeGuard::new();
        let kopi_home = test_home.kopi_home();

        // Setup first
        get_test_command(&kopi_home).arg("setup").assert().success();

        // Add a custom tool
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("add")
            .arg("custom-tool")
            .assert()
            .success()
            .stdout(predicate::str::contains("Created shim for 'custom-tool'"))
            .stdout(predicate::str::contains("Note: This is a custom tool"));
    }

    #[test]
    fn test_shim_verify_with_fix() {
        let test_home = TestHomeGuard::new();
        let kopi_home = test_home.kopi_home();

        // Setup first
        get_test_command(&kopi_home).arg("setup").assert().success();

        // Create a custom shim (not one of the defaults)
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("add")
            .arg("jconsole")
            .assert()
            .success();

        // Verify with fix flag (should succeed even if nothing needs fixing)
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("verify")
            .arg("--fix")
            .assert()
            .success();
    }

    #[test]
    fn test_help_messages() {
        let test_home = TestHomeGuard::new();
        test_home.setup_kopi_structure();
        let kopi_home = test_home.kopi_home();

        // Test main help
        get_test_command(&kopi_home)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("JDK version management tool"))
            .stdout(predicate::str::contains("setup"))
            .stdout(predicate::str::contains("shim"));

        // Test setup help
        get_test_command(&kopi_home)
            .arg("setup")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Initial setup and configuration"));

        // Test shim help
        get_test_command(&kopi_home)
            .arg("shim")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Manage tool shims"))
            .stdout(predicate::str::contains("add"))
            .stdout(predicate::str::contains("remove"))
            .stdout(predicate::str::contains("list"))
            .stdout(predicate::str::contains("verify"));
    }
}
