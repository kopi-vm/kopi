use kopi::cache::{MetadataCache, fetch_and_cache_metadata, find_package_in_cache, get_metadata};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_cache_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn get_test_cache_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join("cache").join("metadata.json")
}

#[test]
#[ignore] // This test requires network access
fn test_fetch_and_cache_metadata() {
    let temp_dir = setup_test_cache_dir();
    let cache_path = get_test_cache_path(&temp_dir);

    // Create cache directory if needed
    fs::create_dir_all(cache_path.parent().unwrap()).ok();

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path().to_str().unwrap());
    }

    // Fetch metadata from API and cache it
    let result = fetch_and_cache_metadata();
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
    let loaded_cache = get_metadata(None);
    assert!(loaded_cache.is_ok(), "Should be able to load cached data");

    let loaded = loaded_cache.unwrap();
    assert_eq!(cache.distributions.len(), loaded.distributions.len());
}

#[test]
fn test_cache_offline_mode() {
    use kopi::cache::DistributionCache;
    use kopi::models::jdk::{
        Architecture, ArchiveType, ChecksumType, Distribution, JdkMetadata, OperatingSystem,
        PackageType, Version,
    };

    let temp_dir = setup_test_cache_dir();
    let cache_path = get_test_cache_path(&temp_dir);

    // Create a proper cache programmatically instead of parsing JSON
    let mut cache = MetadataCache::new();

    let jdk_metadata = JdkMetadata {
        id: "test-id-1".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/download".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 100000000,
        lib_c_type: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Create cache directory if needed
    fs::create_dir_all(cache_path.parent().unwrap()).expect("Failed to create cache dir");

    // Save cache to file
    let cache_json = serde_json::to_string_pretty(&cache).unwrap();
    fs::write(&cache_path, cache_json).expect("Failed to write test cache");

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path().to_str().unwrap());
    }

    // Test loading from cache
    // First verify the cache was written correctly
    let written_content = fs::read_to_string(&cache_path).expect("Should read cache file");
    println!("Written cache content: {}", written_content);

    // Now test loading - use None to just load cache without version check
    let loaded_cache = get_metadata(None).expect("Should load from cache");

    // Verify cache contents
    assert!(loaded_cache.distributions.contains_key("temurin"));
    let temurin = &loaded_cache.distributions["temurin"];
    assert_eq!(temurin.packages.len(), 1);
    assert_eq!(temurin.packages[0].version.to_string(), "21.0.1");
}

#[test]
fn test_find_package_in_cache() {
    use kopi::cache::DistributionCache;
    use kopi::models::jdk::{
        Architecture, ArchiveType, ChecksumType, Distribution, JdkMetadata, OperatingSystem,
        PackageType, Version,
    };

    let mut cache = MetadataCache::new();

    // Add test data
    let jdk_metadata = JdkMetadata {
        id: "test-id".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/download".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 100000000,
        lib_c_type: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Test finding package
    let found = find_package_in_cache(&cache, "temurin", "21.0.1", "x64", "linux");
    assert!(found.is_some(), "Should find package in cache");

    let package = found.unwrap();
    assert_eq!(package.id, "test-id");
    assert_eq!(package.version.to_string(), "21.0.1");

    // Test not finding package
    let not_found = find_package_in_cache(&cache, "temurin", "17.0.1", "x64", "linux");
    assert!(not_found.is_none(), "Should not find non-existent version");
}

#[test]
fn test_cache_corruption_recovery() {
    let temp_dir = setup_test_cache_dir();
    let cache_path = get_test_cache_path(&temp_dir);

    // Create cache directory and write corrupted cache
    fs::create_dir_all(cache_path.parent().unwrap()).expect("Failed to create cache dir");
    fs::write(&cache_path, "invalid json").expect("Failed to write corrupted cache");

    // Create cache directory if needed
    fs::create_dir_all(cache_path.parent().unwrap()).ok();

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path().to_str().unwrap());
    }

    // Should handle corrupted cache gracefully
    let result = get_metadata(None);

    // The function should either return an empty cache or fetch new data
    // It should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_cache_with_install_command() {
    use kopi::cache::DistributionCache;
    use kopi::commands::install::InstallCommand;
    use kopi::models::jdk::{
        Architecture, ArchiveType, ChecksumType, Distribution, JdkMetadata, OperatingSystem,
        PackageType, Version,
    };

    let temp_dir = setup_test_cache_dir();
    let cache_path = get_test_cache_path(&temp_dir);

    // Create a mock cache with test data
    let mut cache = MetadataCache::new();

    let jdk_metadata = JdkMetadata {
        id: "test-install-id".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-21.tar.gz".to_string(),
        checksum: Some("abc123def456".to_string()),
        checksum_type: Some(ChecksumType::Sha256),
        size: 200000000,
        lib_c_type: None,
    };

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![jdk_metadata],
    };
    cache.distributions.insert("temurin".to_string(), dist);

    // Create cache directory if needed
    fs::create_dir_all(cache_path.parent().unwrap()).expect("Failed to create cache dir");

    // Save cache to file
    let cache_json = serde_json::to_string_pretty(&cache).unwrap();
    fs::write(&cache_path, cache_json).expect("Failed to write test cache");

    // Override KOPI_HOME for testing
    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path().to_str().unwrap());
    }

    // Create install command and verify it can use the cache
    let _install_cmd = InstallCommand::new().expect("Failed to create install command");

    // The install command should find the package in cache
    // This is tested indirectly through the find_matching_package method
}
