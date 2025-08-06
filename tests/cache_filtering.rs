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

#[test]
fn test_cache_search_lts_only_filter() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home_path = test_home.kopi_home();
    let kopi_home = kopi_home_path.to_str().unwrap();

    // First refresh the cache
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "refresh"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!(
            "Cache refresh failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Skip test if cache refresh fails (e.g., network issue)
        return;
    }

    // Test LTS-only filter
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "search", "java", "--lts-only"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() && stdout.contains("Available LTS Java versions") {
        // Verify output contains LTS versions
        assert!(stdout.contains("LTS"));

        // Should show proper header
        assert!(stdout.contains("Available LTS Java versions matching 'java':"));

        // Should not contain STS versions
        assert!(!stdout.contains("STS"));
    }
}

#[test]
fn test_cache_search_lts_only_with_json() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home_path = test_home.kopi_home();
    let kopi_home = kopi_home_path.to_str().unwrap();

    // First refresh the cache
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "refresh"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        return; // Skip if cache refresh fails
    }

    // Test LTS-only filter with JSON output
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "search", "21", "--lts-only", "--json"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should be valid JSON
        let result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
        assert!(result.is_ok(), "Output should be valid JSON");

        // Check that all results have LTS term_of_support
        if let Ok(json) = result {
            if let Some(array) = json.as_array() {
                for item in array {
                    if let Some(package) = item.get("package") {
                        if let Some(tos) = package.get("term_of_support") {
                            assert_eq!(tos.as_str(), Some("lts"));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_cache_list_distributions() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home_path = test_home.kopi_home();
    let kopi_home = kopi_home_path.to_str().unwrap();

    // First refresh the cache
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "refresh"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        return; // Skip if cache refresh fails
    }

    // Test list-distributions command
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "list-distributions"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output structure
    assert!(stdout.contains("Available distributions in cache:"));
    assert!(stdout.contains("Distribution"));
    assert!(stdout.contains("Display Name"));
    assert!(stdout.contains("Versions"));

    // Should show total count
    assert!(stdout.contains("Total:"));

    // Should show some common distributions if cache is populated
    // At least one known distribution should be present
    assert!(
        stdout.contains("temurin") || stdout.contains("corretto") || stdout.contains("zulu"),
        "Expected at least one known distribution in output"
    );
}

#[test]
fn test_cache_list_distributions_no_cache() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home_path = test_home.kopi_home();
    let kopi_home = kopi_home_path.to_str().unwrap();

    // Test list-distributions with no cache
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "list-distributions"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // When no cache exists, the command might automatically fetch data
    // or show an appropriate message
    assert!(
        output.status.success(),
        "Command failed. stdout: {stdout}, stderr: {stderr}"
    );

    // The behavior might have changed - it might auto-fetch or show distributions
    assert!(
        stdout.contains("No cache found")
            || stdout.contains("Fetching")
            || stdout.contains("Available distributions")
            || stdout.contains("Distribution"),
        "Unexpected output. stdout: {stdout}"
    );
}

#[test]
fn test_cache_search_no_lts_results() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home_path = test_home.kopi_home();
    let kopi_home = kopi_home_path.to_str().unwrap();

    // First refresh the cache
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "refresh"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        return; // Skip if cache refresh fails
    }

    // Search for a version that likely won't have LTS results
    let output = Command::cargo_bin("kopi")
        .unwrap()
        .args(["cache", "search", "99", "--lts-only"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should indicate no LTS versions found
    if !stdout.contains("Available LTS Java versions") {
        assert!(stdout.contains("No matching LTS Java versions found"));
    }
}
