mod common;

use common::TestHomeGuard;
use kopi::cache::MetadataCache;
use kopi::config::KopiConfig;
use kopi::doctor::{CheckCategory, CheckStatus, DiagnosticEngine};
use std::fs;
use std::time::Duration;

#[test]
fn test_network_checks_pass_with_connectivity() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Network]), false);

    // We should have 4 network checks
    assert_eq!(results.len(), 4);

    // Find each check by name
    let api_check = results.iter().find(|r| r.name == "API Connectivity");
    let dns_check = results.iter().find(|r| r.name == "DNS Resolution");
    let proxy_check = results.iter().find(|r| r.name == "Proxy Configuration");
    let tls_check = results.iter().find(|r| r.name == "TLS/SSL Verification");

    assert!(api_check.is_some(), "API connectivity check not found");
    assert!(dns_check.is_some(), "DNS resolution check not found");
    assert!(proxy_check.is_some(), "Proxy configuration check not found");
    assert!(tls_check.is_some(), "TLS verification check not found");

    // In a working environment, these should typically pass
    // DNS resolution should always work
    assert_eq!(dns_check.unwrap().status, CheckStatus::Pass);
}

#[test]
fn test_cache_checks_with_no_cache() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Ensure cache doesn't exist
    let cache_path = config.kopi_home().join("cache").join("metadata.json");
    if cache_path.exists() {
        fs::remove_file(&cache_path).ok();
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Cache]), false);

    // We should have 5 cache checks
    assert_eq!(results.len(), 5);

    // Find the file existence check
    let file_check = results
        .iter()
        .find(|r| r.name == "Cache File Existence")
        .unwrap();
    assert_eq!(file_check.status, CheckStatus::Warning);
    assert!(file_check.message.contains("does not exist"));

    // Other checks should skip when no cache exists
    for result in &results {
        if result.name != "Cache File Existence" {
            assert_eq!(result.status, CheckStatus::Skip);
        }
    }
}

#[test]
fn test_cache_checks_with_valid_cache() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create cache directory
    let cache_dir = config.kopi_home().join("cache");
    fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

    // Create a valid cache
    let cache = MetadataCache::new();
    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Cache]), false);

    // Find each check by name
    let file_check = results
        .iter()
        .find(|r| r.name == "Cache File Existence")
        .unwrap();
    let perm_check = results
        .iter()
        .find(|r| r.name == "Cache File Permissions")
        .unwrap();
    let format_check = results
        .iter()
        .find(|r| r.name == "Cache Format Validation")
        .unwrap();
    let stale_check = results
        .iter()
        .find(|r| r.name == "Cache Staleness")
        .unwrap();
    let size_check = results
        .iter()
        .find(|r| r.name == "Cache Size Analysis")
        .unwrap();

    // All checks should pass with a fresh, valid cache
    assert_eq!(file_check.status, CheckStatus::Pass);
    assert_eq!(perm_check.status, CheckStatus::Pass);
    assert_eq!(format_check.status, CheckStatus::Pass);
    assert_eq!(stale_check.status, CheckStatus::Pass); // Fresh cache
    assert_eq!(size_check.status, CheckStatus::Pass); // Small cache
}

#[test]
fn test_cache_checks_with_invalid_json() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create cache directory
    let cache_dir = config.kopi_home().join("cache");
    fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

    // Create invalid JSON cache
    let cache_path = cache_dir.join("metadata.json");
    fs::write(&cache_path, "{ invalid json }").expect("Failed to write invalid cache");

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Cache]), false);

    // Find format check
    let format_check = results
        .iter()
        .find(|r| r.name == "Cache Format Validation")
        .unwrap();
    assert_eq!(format_check.status, CheckStatus::Fail);
    assert!(format_check.message.contains("invalid JSON"));
}

#[test]
#[cfg(unix)]
fn test_cache_permissions_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create cache
    let cache_dir = config.kopi_home().join("cache");
    fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

    let cache = MetadataCache::new();
    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    // Make cache unreadable (for testing)
    let mut perms = fs::metadata(&cache_path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&cache_path, perms).expect("Failed to set permissions");

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Cache]), false);

    // Permission check should fail
    let perm_check = results
        .iter()
        .find(|r| r.name == "Cache File Permissions")
        .unwrap();
    assert_eq!(perm_check.status, CheckStatus::Fail);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&cache_path).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&cache_path, perms).ok();
}

#[test]
fn test_proxy_environment_detection() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Test with proxy environment variables
    unsafe {
        std::env::set_var("HTTP_PROXY", "http://proxy.example.com:8080");
        std::env::set_var("HTTPS_PROXY", "https://proxy.example.com:8080");
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Network]), false);

    let proxy_check = results
        .iter()
        .find(|r| r.name == "Proxy Configuration")
        .unwrap();
    assert_eq!(proxy_check.status, CheckStatus::Pass);
    assert!(proxy_check.message.contains("Proxy configuration detected"));

    // Clean up
    unsafe {
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
    }
}

#[test]
fn test_network_checks_performance() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let start = std::time::Instant::now();
    let results = engine.run_checks(Some(vec![CheckCategory::Network]), false);
    let total_duration = start.elapsed();

    // Network checks should complete within reasonable time
    assert!(
        total_duration < Duration::from_secs(10),
        "Network checks took too long: {:?}",
        total_duration
    );

    // Each individual check should be reasonably fast
    for result in results {
        assert!(
            result.duration < Duration::from_secs(6),
            "Check '{}' took too long: {:?}",
            result.name,
            result.duration
        );
    }
}

#[test]
fn test_cache_staleness_detection() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create cache directory
    let cache_dir = config.kopi_home().join("cache");
    fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

    // Create a cache with old timestamp
    let mut cache = MetadataCache::new();
    // Set timestamp older than configured max age (default is 720 hours = 30 days)
    // We'll use 35 days to ensure it's considered stale
    let stale_days = (config.cache.max_age_hours / 24) + 5;
    cache.last_updated = chrono::Utc::now() - chrono::Duration::days(stale_days as i64);

    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Cache]), false);

    let stale_check = results
        .iter()
        .find(|r| r.name == "Cache Staleness")
        .unwrap();
    assert_eq!(stale_check.status, CheckStatus::Warning);
    // Check that the message contains the age and max age
    assert!(stale_check.message.contains("days old (max age:"));
}

#[test]
fn test_all_network_and_cache_checks() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create a valid cache for cache checks
    let cache_dir = config.kopi_home().join("cache");
    fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    let cache = MetadataCache::new();
    cache
        .save(&cache_dir.join("metadata.json"))
        .expect("Failed to save cache");

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(
        Some(vec![CheckCategory::Network, CheckCategory::Cache]),
        false,
    );

    // We should have 4 network + 5 cache = 9 total checks
    assert_eq!(results.len(), 9);

    // All checks should complete successfully (no panics)
    for result in &results {
        println!("{}: {} - {}", result.name, result.status, result.message);
    }
}
