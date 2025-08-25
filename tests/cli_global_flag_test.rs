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

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn test_global_no_progress_flag_in_help() {
    Command::cargo_bin("kopi")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--no-progress"))
        .stdout(contains("Disable progress indicators"));
}

#[test]
fn test_global_no_progress_flag_on_install() {
    // Test that the flag is accepted on install command
    Command::cargo_bin("kopi")
        .unwrap()
        .args(["--no-progress", "install", "--help"])
        .assert()
        .success();
}

#[test]
fn test_global_no_progress_flag_on_cache() {
    // Test that the flag is accepted on cache command
    Command::cargo_bin("kopi")
        .unwrap()
        .args(["--no-progress", "cache", "--help"])
        .assert()
        .success();
}

#[test]
fn test_global_no_progress_flag_on_uninstall() {
    // Test that the flag is accepted on uninstall command
    Command::cargo_bin("kopi")
        .unwrap()
        .args(["--no-progress", "uninstall", "--help"])
        .assert()
        .success();
}

#[test]
fn test_global_no_progress_flag_position() {
    // Test that the global flag can be placed before the subcommand
    Command::cargo_bin("kopi")
        .unwrap()
        .args(["--no-progress", "list"])
        .assert()
        .success();
}

#[test]
fn test_no_progress_flag_not_duplicated_in_install() {
    // Test that install command help includes the global --no-progress flag
    // Note: clap shows global flags in subcommand help output
    Command::cargo_bin("kopi")
        .unwrap()
        .args(["install", "--help"])
        .assert()
        .success()
        .stdout(contains("--force"))
        .stdout(contains("--dry-run"))
        .stdout(contains("--no-progress"))
        .stdout(contains("Disable progress indicators"));
}
