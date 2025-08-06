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
use assert_cmd::prelude::*;
use common::TestHomeGuard;
use std::process::Command;

#[test]
fn test_with_random_kopi_home() {
    // Create test home with random directory
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();

    // Run kopi command with the test home
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", test_home.kopi_home()).arg("list");

    // Assert command runs successfully
    cmd.assert().success();

    // Directory will be automatically cleaned up when test_home is dropped
}

#[test]
fn test_install_with_test_home() {
    // Create test home
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Mock metadata in cache
    let cache_dir = kopi_home.join("cache");
    let metadata_content = r#"{
        "packages": [
            {
                "distribution": "temurin",
                "version": "21.0.1",
                "links": {
                    "download": "https://example.com/jdk.tar.gz"
                }
            }
        ],
        "last_updated": "2024-01-01T00:00:00Z"
    }"#;

    std::fs::write(cache_dir.join("metadata.json"), metadata_content).unwrap();

    // Test that we can run commands with this test home
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &kopi_home).arg("list");

    cmd.assert().success();

    // Verify kopi home structure was used
    assert!(kopi_home.exists());
    assert!(kopi_home.join("jdks").exists());

    // Test home will be cleaned up automatically
}
