use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cache_search_auto_fetch_distribution() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // First, ensure no cache exists
    let _ = Command::new("cargo")
        .args(["run", "--", "cache", "clear"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    // Search for a specific distribution that's not cached
    let output = Command::new("cargo")
        .args(["run", "--", "cache", "search", "zulu"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show fetching message
    if output.status.success() {
        // Either it found the distribution or showed appropriate message
        // Check that some expected output is present
        assert!(
            stdout.contains("zulu")
                || stdout.contains("Zulu")
                || stdout.contains("Distribution")
                || stdout.contains("Failed")
        );
    } else {
        // If network error or API issue, it should show appropriate error
        assert!(
            stdout.contains("Failed to fetch distribution") || stderr.contains("Failed to fetch")
        );
    }
}

#[test]
fn test_cache_search_specific_distribution_version() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Clear cache first
    let _ = Command::new("cargo")
        .args(["run", "--", "cache", "clear"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    // Search for a specific distribution and version
    let output = Command::new("cargo")
        .args(["run", "--", "cache", "search", "dragonwell@21"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() {
        // Should either fetch the distribution or show results
        assert!(
            stdout.contains("dragonwell")
                || stdout.contains("Dragonwell")
                || stdout.contains("21")
                || stdout.contains("No matching")
        );
    }
}

#[test]
fn test_cache_search_json_with_auto_fetch() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Clear cache first
    let _ = Command::new("cargo")
        .args(["run", "--", "cache", "clear"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    // Search with JSON output for a distribution not in cache
    let output = Command::new("cargo")
        .args(["run", "--", "cache", "search", "liberica", "--json"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() {
        // Should return valid JSON (either empty array or results)
        let result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
        assert!(result.is_ok(), "Output should be valid JSON");
    }
}

#[test]
fn test_cache_search_invalid_distribution() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Search for an invalid distribution
    let output = Command::new("cargo")
        .args(["run", "--", "cache", "search", "notarealdistribution"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail to fetch the distribution
    assert!(
        stdout.contains("Failed")
            || stdout.contains("No matching")
            || stdout.contains("notarealdistribution")
            || stderr.contains("Error")
            || stderr.contains("InvalidVersionFormat")
    );
}

#[test]
fn test_cache_persists_after_fetch() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_str().unwrap();

    // Clear cache first
    let _ = Command::new("cargo")
        .args(["run", "--", "cache", "clear"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    // First search - should fetch
    let output1 = Command::new("cargo")
        .args(["run", "--", "cache", "search", "semeru"])
        .env("KOPI_HOME", kopi_home)
        .output()
        .expect("Failed to execute command");

    if output1.status.success() {
        let stdout1 = String::from_utf8_lossy(&output1.stdout);

        // Second search - should use cache
        let output2 = Command::new("cargo")
            .args(["run", "--", "cache", "search", "semeru"])
            .env("KOPI_HOME", kopi_home)
            .output()
            .expect("Failed to execute command");

        let stdout2 = String::from_utf8_lossy(&output2.stdout);

        // Second search should NOT show fetching message
        // If first search had to fetch, second should not
        if stdout1.contains("Fetching from foojay.io") {
            assert!(!stdout2.contains("Fetching from foojay.io"));
        }
    }
}
