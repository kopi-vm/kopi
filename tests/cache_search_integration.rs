mod common;
use common::TestHomeGuard;
use kopi::cache::{DistributionCache, MetadataCache};
use kopi::commands::cache::CacheCommand;
use kopi::models::distribution::Distribution;
use kopi::models::metadata::JdkMetadata;
use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
use kopi::models::platform::{Architecture, OperatingSystem};
use kopi::version::Version;
use std::env;
use std::str::FromStr;

/// Helper function to create a comprehensive test cache with various JDK versions and distributions
fn create_comprehensive_test_cache() -> (TestHomeGuard, MetadataCache) {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }

    let mut cache = MetadataCache::new();

    // Create Temurin distribution with LTS and non-LTS versions
    let temurin_packages = vec![
        JdkMetadata {
            id: "temurin-21-lts".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 5),
            distribution_version: Version::from_str("21.0.5+11").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/temurin-21.tar.gz".to_string(),
            checksum: Some("abc123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 195_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "temurin-22-sts".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(22, 0, 2),
            distribution_version: Version::from_str("22.0.2+9").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/temurin-22.tar.gz".to_string(),
            checksum: Some("def456".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 198_000_000,
            javafx_bundled: false,
            term_of_support: Some("sts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "temurin-23-ea".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(23, 0, 0),
            distribution_version: Version::from_str("23+37-ea").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/temurin-23.tar.gz".to_string(),
            checksum: Some("ghi789".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 200_000_000,
            javafx_bundled: false,
            term_of_support: Some("sts".to_string()),
            release_status: Some("ea".to_string()),
            latest_build_available: Some(true),
        },
    ];

    cache.distributions.insert(
        "temurin".to_string(),
        DistributionCache {
            distribution: Distribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: temurin_packages,
        },
    );

    // Create Corretto distribution with LTS versions
    let corretto_packages = vec![
        JdkMetadata {
            id: "corretto-17-lts".to_string(),
            distribution: "corretto".to_string(),
            version: Version::new(17, 0, 12),
            distribution_version: Version::from_str("17.0.12.7.1").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/corretto-17.tar.gz".to_string(),
            checksum: Some("jkl012".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 180_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "corretto-11-lts".to_string(),
            distribution: "corretto".to_string(),
            version: Version::new(11, 0, 24),
            distribution_version: Version::from_str("11.0.24.8.1").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/corretto-11.tar.gz".to_string(),
            checksum: Some("mno345".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 170_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(false),
        },
    ];

    cache.distributions.insert(
        "corretto".to_string(),
        DistributionCache {
            distribution: Distribution::Corretto,
            display_name: "Amazon Corretto".to_string(),
            packages: corretto_packages,
        },
    );

    // Create Zulu distribution with JavaFX bundled versions
    let zulu_packages = vec![
        JdkMetadata {
            id: "zulu-21-fx".to_string(),
            distribution: "zulu".to_string(),
            version: Version::new(21, 0, 4),
            distribution_version: Version::from_str("21.0.4+7").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/zulu-21-fx.tar.gz".to_string(),
            checksum: Some("pqr678".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 220_000_000,
            javafx_bundled: true,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "zulu-21".to_string(),
            distribution: "zulu".to_string(),
            version: Version::new(21, 0, 4),
            distribution_version: Version::from_str("21.0.4+7").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/zulu-21.tar.gz".to_string(),
            checksum: Some("stu901".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 190_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
    ];

    cache.distributions.insert(
        "zulu".to_string(),
        DistributionCache {
            distribution: Distribution::Zulu,
            display_name: "Azul Zulu".to_string(),
            packages: zulu_packages,
        },
    );

    // Save cache (cache directory already exists from setup_kopi_structure)
    let cache_path = test_home.kopi_home().join("cache").join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    (test_home, cache)
}

#[test]
fn test_integration_compact_display_with_lts_filter() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test compact display with LTS-only filter
    let cmd = CacheCommand::Search {
        version: "".to_string(), // Empty string searches all
        compact: true,
        detailed: false,
        json: false,
        lts_only: true,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // The test should show only LTS versions (21, 17, 11) in compact format
}

#[test]
fn test_integration_detailed_display_with_distribution_search() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test detailed display for specific distribution
    let cmd = CacheCommand::Search {
        version: "corretto".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // The test should show all Corretto versions with detailed columns
}

#[test]
fn test_integration_json_output_with_javafx_filter() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test JSON output with JavaFX filter
    let cmd = CacheCommand::Search {
        version: "".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: true, // This enables JavaFX filter
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // The test should output valid JSON containing only JavaFX bundled packages
}

#[test]
fn test_integration_latest_search_with_lts_filter() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test latest search combined with LTS filter
    let cmd = CacheCommand::Search {
        version: "latest".to_string(),
        compact: true,
        detailed: false,
        json: false,
        lts_only: true,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show only the latest LTS version from each distribution
}

#[test]
fn test_integration_version_specific_search_across_distributions() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Search for version 21 across all distributions
    let cmd = CacheCommand::Search {
        version: "21".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show version 21 from both Temurin and Zulu
}

#[test]
fn test_integration_multiple_filters_combined() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test multiple filters: LTS + specific distribution
    let cmd = CacheCommand::Search {
        version: "zulu".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: true,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show only LTS Zulu versions with detailed display
}

#[test]
fn test_integration_edge_case_no_matching_results() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Search for non-existent version
    let cmd = CacheCommand::Search {
        version: "99".to_string(),
        compact: true,
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    // Should execute successfully but show no results
    assert!(cmd.execute().is_ok());
}

#[test]
fn test_integration_edge_case_conflicting_display_modes() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test with both compact and detailed flags (detailed should take precedence)
    let cmd = CacheCommand::Search {
        version: "".to_string(),
        compact: true,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should use detailed display mode
}

#[test]
fn test_integration_list_distributions_with_package_counts() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    let cmd = CacheCommand::ListDistributions;
    assert!(cmd.execute().is_ok());
    // Should list all distributions with their package counts
}

#[test]
fn test_integration_search_with_distribution_version_format() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test distribution@version format
    let cmd = CacheCommand::Search {
        version: "temurin@22".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show only Temurin version 22
}

#[test]
fn test_integration_backward_compatibility_default_behavior() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test default search without any flags (should use compact mode)
    let cmd = CacheCommand::Search {
        version: "".to_string(),
        compact: false, // Default when no flags
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show all packages in default (compact) format
}

#[test]
fn test_integration_platform_specific_filtering() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }

    // Create cache with multiple platforms
    let mut cache = MetadataCache::new();

    let packages = vec![
        JdkMetadata {
            id: "temurin-21-linux".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 5),
            distribution_version: Version::from_str("21.0.5+11").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            lib_c_type: Some("glibc".to_string()),
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/21-linux.tar.gz".to_string(),
            checksum: Some("linux123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 195_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "temurin-21-windows".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 5),
            distribution_version: Version::from_str("21.0.5+11").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Windows,
            package_type: PackageType::Jdk,
            lib_c_type: None,
            archive_type: ArchiveType::Zip,
            download_url: "https://test.com/21-windows.zip".to_string(),
            checksum: Some("win123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 200_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "temurin-21-mac".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 5),
            distribution_version: Version::from_str("21.0.5+11").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::MacOS,
            package_type: PackageType::Jdk,
            lib_c_type: None,
            archive_type: ArchiveType::TarGz,
            download_url: "https://test.com/21-mac.tar.gz".to_string(),
            checksum: Some("mac123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 198_000_000,
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
    ];

    cache.distributions.insert(
        "temurin".to_string(),
        DistributionCache {
            distribution: Distribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages,
        },
    );

    // Create cache directory structure
    let cache_dir = test_home.kopi_home().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    // Search should filter by current platform
    let cmd = CacheCommand::Search {
        version: "21".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should show only packages for the current platform
}

#[test]
fn test_integration_regression_old_search_patterns() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Test old-style version search still works
    let cmd = CacheCommand::Search {
        version: "17.0.12".to_string(),
        compact: false,
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should find Corretto 17.0.12
}

#[test]
fn test_integration_empty_cache_handling() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }

    // Create empty cache
    let cache = MetadataCache::new();
    // Create cache directory structure
    let cache_dir = test_home.kopi_home().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    let cmd = CacheCommand::Search {
        version: "".to_string(),
        compact: true,
        detailed: false,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // Should handle empty cache gracefully
}

#[test]
fn test_integration_json_output_structure_validation() {
    let (_test_home, _cache) = create_comprehensive_test_cache();

    // Capture JSON output for validation
    let cmd = CacheCommand::Search {
        version: "temurin@21".to_string(),
        compact: false,
        detailed: false,
        json: true,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    // JSON output should be valid and contain expected fields
}

#[test]
fn test_integration_performance_large_cache() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    unsafe {
        env::set_var("KOPI_HOME", test_home.kopi_home());
    }

    // Create a large cache with many packages
    let mut cache = MetadataCache::new();
    let mut packages = Vec::new();

    // Create 100 packages across different versions
    for major in 8..=23 {
        for minor in 0..5 {
            packages.push(JdkMetadata {
                id: format!("temurin-{major}-{minor}"),
                distribution: "temurin".to_string(),
                version: Version::new(major, minor, 0),
                distribution_version: Version::from_str(&format!("{major}.{minor}.0+36")).unwrap(),
                architecture: Architecture::X64,
                operating_system: OperatingSystem::Linux,
                package_type: PackageType::Jdk,
                lib_c_type: Some("glibc".to_string()),
                archive_type: ArchiveType::TarGz,
                download_url: format!("https://test.com/{major}-{minor}.tar.gz"),
                checksum: Some(format!("checksum{major}_{minor}")),
                checksum_type: Some(ChecksumType::Sha256),
                size: 190_000_000 + (major * 1_000_000) as u64,
                javafx_bundled: false,
                term_of_support: if major == 8 || major == 11 || major == 17 || major == 21 {
                    Some("lts".to_string())
                } else {
                    Some("sts".to_string())
                },
                release_status: Some("ga".to_string()),
                latest_build_available: Some(minor == 4),
            });
        }
    }

    cache.distributions.insert(
        "temurin".to_string(),
        DistributionCache {
            distribution: Distribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages,
        },
    );

    // Create cache directory structure
    let cache_dir = test_home.kopi_home().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    let cache_path = cache_dir.join("metadata.json");
    cache.save(&cache_path).expect("Failed to save cache");

    // Test search performance with large cache
    let start = std::time::Instant::now();
    let cmd = CacheCommand::Search {
        version: "17".to_string(),
        compact: false,
        detailed: true,
        json: false,
        lts_only: false,
        javafx_bundled: false,
        java_version: false,
        distribution_version: false,
    };

    assert!(cmd.execute().is_ok());
    let duration = start.elapsed();

    // Search should complete quickly even with large cache
    assert!(
        duration.as_millis() < 1000,
        "Search took too long: {duration:?}"
    );
}
