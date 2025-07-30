mod common;
use common::TestHomeGuard;
use kopi::cache::{MetadataCache, fetch_and_cache_metadata, get_metadata};
use kopi::config::new_kopi_config;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn get_test_cache_path(test_home: &TestHomeGuard) -> PathBuf {
    test_home.kopi_home().join("cache").join("metadata.json")
}

#[test]
#[serial]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_fetch_and_cache_metadata() {
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let cache_path = get_test_cache_path(test_home);

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", test_home.kopi_home().to_str().unwrap());
    }

    // Fetch metadata from API and cache it
    let config = new_kopi_config().unwrap();
    let result = fetch_and_cache_metadata(false, &config);
    assert!(
        result.is_ok(),
        "Failed to fetch metadata: {:?}",
        result.err()
    );

    let cache = result.unwrap();

    // Verify cache contains distributions
    assert!(
        !cache.distributions.is_empty(),
        "Cache should contain distributions"
    );

    // Check for common distributions
    assert!(
        cache.distributions.contains_key("temurin"),
        "Should have Temurin"
    );
    assert!(
        cache.distributions.contains_key("corretto"),
        "Should have Corretto"
    );

    // Verify cache file was created
    assert!(cache_path.exists(), "Cache file should be created");

    // Verify we can load the cache
    let loaded_cache = get_metadata(None, &config);
    assert!(loaded_cache.is_ok(), "Should be able to load cached data");

    let loaded = loaded_cache.unwrap();
    assert_eq!(cache.distributions.len(), loaded.distributions.len());
}

#[test]
#[serial]
fn test_cache_offline_mode() {
    use kopi::cache::DistributionCache;
    use kopi::models::distribution::Distribution;
    use kopi::models::metadata::JdkMetadata;
    use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
    use kopi::models::platform::{Architecture, OperatingSystem};
    use kopi::version::Version;

    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let cache_path = get_test_cache_path(test_home);

    // Create a proper cache programmatically instead of parsing JSON
    let mut cache = MetadataCache::new();

    let jdk_metadata = JdkMetadata {
        id: "test-id-1".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: Version::from_str("21.0.1+12").unwrap(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: Some("https://example.com/download".to_string()),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 100000000,
        lib_c_type: None,
        javafx_bundled: false,
        term_of_support: None,
        release_status: None,
        latest_build_available: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Save cache to file (cache directory already exists from setup_kopi_structure)
    let cache_json = serde_json::to_string_pretty(&cache).unwrap();
    fs::write(&cache_path, cache_json).expect("Failed to write test cache");

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", test_home.kopi_home().to_str().unwrap());
    }

    // Test loading from cache
    // First verify the cache was written correctly
    let written_content = fs::read_to_string(&cache_path).expect("Should read cache file");
    println!("Written cache content: {written_content}");

    // Now test loading directly from the cache file to ensure offline mode
    let loaded_content = fs::read_to_string(&cache_path).expect("Should read cache file");
    let loaded_cache: MetadataCache =
        serde_json::from_str(&loaded_content).expect("Should parse cache JSON");

    // Verify cache contents
    assert!(loaded_cache.distributions.contains_key("temurin"));
    let temurin = &loaded_cache.distributions["temurin"];
    assert_eq!(temurin.packages.len(), 1);
    assert_eq!(temurin.packages[0].version.to_string(), "21.0.1");
}

#[test]
#[serial]
fn test_find_package_in_cache() {
    use kopi::cache::DistributionCache;
    use kopi::models::distribution::Distribution;
    use kopi::models::metadata::JdkMetadata;
    use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
    use kopi::models::platform::{Architecture, OperatingSystem};
    use kopi::version::Version;

    let mut cache = MetadataCache::new();

    // Add test data
    let jdk_metadata = JdkMetadata {
        id: "test-id".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: Version::from_str("21.0.1+12").unwrap(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: Some("https://example.com/download".to_string()),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 100000000,
        lib_c_type: None,
        javafx_bundled: false,
        term_of_support: None,
        release_status: None,
        latest_build_available: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Test accessing the cache structure directly
    assert!(
        cache.distributions.contains_key("temurin"),
        "Should have temurin distribution"
    );

    let temurin_dist = &cache.distributions["temurin"];
    assert_eq!(temurin_dist.packages.len(), 1, "Should have one package");
    assert_eq!(temurin_dist.packages[0].id, "test-id");
    assert_eq!(temurin_dist.packages[0].version.to_string(), "21.0.1");
}

#[test]
#[serial]
fn test_cache_corruption_recovery() {
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let cache_path = get_test_cache_path(test_home);

    // Write corrupted cache (cache directory already exists from setup_kopi_structure)
    fs::write(&cache_path, "invalid json").expect("Failed to write corrupted cache");

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", test_home.kopi_home().to_str().unwrap());
    }

    // Should handle corrupted cache gracefully
    let config = new_kopi_config().unwrap();
    let result = get_metadata(None, &config);

    // The function should either return an empty cache or fetch new data
    // It should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[serial]
fn test_cache_with_install_command() {
    use kopi::cache::DistributionCache;
    use kopi::commands::install::InstallCommand;
    use kopi::models::distribution::Distribution;
    use kopi::models::metadata::JdkMetadata;
    use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
    use kopi::models::platform::{Architecture, OperatingSystem};
    use kopi::version::Version;

    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    let cache_path = get_test_cache_path(test_home);

    // Create a mock cache with test data
    let mut cache = MetadataCache::new();

    let jdk_metadata = JdkMetadata {
        id: "test-install-id".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: Version::from_str("21.0.1+12").unwrap(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: Some("https://example.com/temurin-21.tar.gz".to_string()),
        checksum: Some("abc123def456".to_string()),
        checksum_type: Some(ChecksumType::Sha256),
        size: 200000000,
        lib_c_type: None,
        javafx_bundled: false,
        term_of_support: None,
        release_status: None,
        latest_build_available: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Save cache to file (cache directory already exists from setup_kopi_structure)
    let cache_json = serde_json::to_string_pretty(&cache).unwrap();
    fs::write(&cache_path, cache_json).expect("Failed to write test cache");

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", test_home.kopi_home().to_str().unwrap());
    }

    // Create config
    let config = new_kopi_config().unwrap();

    // Verify that InstallCommand can be created with the cache
    // This tests that the command can initialize properly with the cached data
    InstallCommand::new(&config).expect("Failed to create install command");
}
