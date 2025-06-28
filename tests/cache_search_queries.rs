use kopi::cache::{DistributionCache, MetadataCache, get_cache_path, save_cache};
use kopi::commands::cache::CacheCommand;
use kopi::models::jdk::{
    Architecture, ArchiveType, ChecksumType, Distribution, JdkMetadata, OperatingSystem,
    PackageType, Version,
};
use std::env;
use tempfile::TempDir;

fn setup_test_cache() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        env::set_var("KOPI_HOME", temp_dir.path());
    }

    // Create test metadata
    let mut cache = MetadataCache::new();

    // Add Temurin packages
    let mut temurin_packages = Vec::new();

    // Temurin 21.0.1
    temurin_packages.push(JdkMetadata {
        id: "temurin-21".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-21.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 100_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    });

    // Temurin 17.0.9
    temurin_packages.push(JdkMetadata {
        id: "temurin-17".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(17, 0, 9),
        distribution_version: "17.0.9".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-17.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 90_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    });

    // Temurin 11.0.21
    temurin_packages.push(JdkMetadata {
        id: "temurin-11".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(11, 0, 21),
        distribution_version: "11.0.21".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-11.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 85_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    });

    cache.distributions.insert(
        "temurin".to_string(),
        DistributionCache {
            distribution: Distribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: temurin_packages,
        },
    );

    // Add Corretto packages
    let mut corretto_packages = Vec::new();

    // Corretto 21.0.2
    corretto_packages.push(JdkMetadata {
        id: "corretto-21".to_string(),
        distribution: "corretto".to_string(),
        version: Version::new(21, 0, 2),
        distribution_version: "21.0.2".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/corretto-21.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 105_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    });

    // Corretto 17.0.10
    corretto_packages.push(JdkMetadata {
        id: "corretto-17".to_string(),
        distribution: "corretto".to_string(),
        version: Version::new(17, 0, 10),
        distribution_version: "17.0.10".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/corretto-17.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 95_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    });

    cache.distributions.insert(
        "corretto".to_string(),
        DistributionCache {
            distribution: Distribution::Corretto,
            display_name: "Amazon Corretto".to_string(),
            packages: corretto_packages,
        },
    );

    // Save cache
    let cache_path = get_cache_path().unwrap();
    // Ensure parent directory exists
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    save_cache(&cache_path, &cache).unwrap();

    temp_dir
}

#[test]
fn test_search_distribution_only() {
    let _temp_dir = setup_test_cache();

    // Search for all Corretto versions
    let cmd = CacheCommand::Search {
        version: "corretto".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed and return all Corretto versions
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_latest_all_distributions() {
    let _temp_dir = setup_test_cache();

    // Search for latest version across all distributions
    let cmd = CacheCommand::Search {
        version: "latest".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed and return the latest version from each distribution
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_latest_specific_distribution() {
    let _temp_dir = setup_test_cache();

    // Search for latest Temurin version
    let cmd = CacheCommand::Search {
        version: "temurin@latest".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed and return only the latest Temurin version
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_backward_compatibility() {
    let _temp_dir = setup_test_cache();

    // Test that existing version queries still work
    let cmd = CacheCommand::Search {
        version: "21".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed and return version 21 (defaulting to Temurin)
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_distribution_with_version() {
    let _temp_dir = setup_test_cache();

    // Test searching for specific distribution and version
    let cmd = CacheCommand::Search {
        version: "corretto@17".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed and return Corretto 17
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_invalid_distribution() {
    let _temp_dir = setup_test_cache();

    // Test searching for invalid distribution
    let cmd = CacheCommand::Search {
        version: "invalid_distro".to_string(),
        compact: false,
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should fail with an error about unknown distribution
    assert!(cmd.execute().is_err());
}

#[test]
fn test_search_jre_latest() {
    let _temp_dir = setup_test_cache();

    // Test searching for latest JRE
    let cmd = CacheCommand::Search {
        version: "jre@latest".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };

    // This should succeed (even if no JRE packages exist, it should return empty results)
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_search_display_modes() {
    let _temp_dir = setup_test_cache();

    // Test compact mode
    let cmd_compact = CacheCommand::Search {
        version: "temurin".to_string(),
        compact: true,
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
    };
    assert!(cmd_compact.execute().is_ok());

    // Test detailed mode
    let cmd_detailed = CacheCommand::Search {
        version: "temurin".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
    };
    assert!(cmd_detailed.execute().is_ok());

    // Test JSON mode
    let cmd_json = CacheCommand::Search {
        version: "temurin".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
    };
    assert!(cmd_json.execute().is_ok());
}
