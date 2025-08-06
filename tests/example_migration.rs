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

// Example showing how to migrate from TempDir to TestHomeGuard

mod common;
use assert_cmd::Command;
use common::TestHomeGuard;

// OLD PATTERN (using TempDir directly)
/*
use tempfile::TempDir;
use std::fs;
use std::path::{Path, PathBuf};

fn setup_test_home() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().join(".kopi");
    fs::create_dir_all(&kopi_home).unwrap();
    (temp_dir, kopi_home)
}

#[test]
fn test_old_pattern() {
    let (_temp_dir, kopi_home) = setup_test_home();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.arg("list");

    cmd.assert().success();
}
*/

// NEW PATTERN (using TestHomeGuard)
#[test]
fn test_new_pattern() {
    // Create test home with random 8-character directory under target/home
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &kopi_home);
    cmd.arg("list");

    cmd.assert().success();

    // Cleanup happens automatically when test_home is dropped
}

// Benefits of TestHomeGuard:
// 1. Creates directories under target/home/<random-8-chars>/ instead of system temp
// 2. Automatically sets up .kopi directory structure
// 3. Provides consistent cleanup on drop
// 4. Uses 8-character random names as requested
// 5. Less boilerplate code needed in each test
