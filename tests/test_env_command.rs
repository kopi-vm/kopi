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

#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    let parent = kopi_home.parent().unwrap();
    cmd.env("HOME", parent);
    // On Windows, dirs crate uses USERPROFILE, not HOME
    #[cfg(windows)]
    cmd.env("USERPROFILE", parent);
    cmd
}

/// Test basic env command with bash shell (default)
#[test]
fn test_env_basic_bash() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a mock JDK installation with proper structure
    let jdk_path = kopi_home.join("jdks").join("temurin-21.0.1");
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create mock executables
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    fs::write(bin_dir.join(format!("java{exe_ext}")), "mock java").unwrap();
    fs::write(bin_dir.join(format!("javac{exe_ext}")), "mock javac").unwrap();

    // Test env command
    let mut cmd = get_test_command(&kopi_home);
    cmd.env("KOPI_JAVA_VERSION", "temurin@21.0.1");
    cmd.arg("env").arg("--shell").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export JAVA_HOME="))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test env command with JDK not installed
#[test]
fn test_env_jdk_not_installed() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test env command with version set but JDK not installed
    let mut cmd = get_test_command(&kopi_home);
    cmd.env("KOPI_JAVA_VERSION", "temurin@21.0.1");
    cmd.arg("env");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Error: JDK 'temurin@21.0.1' is not installed",
    ));
}
