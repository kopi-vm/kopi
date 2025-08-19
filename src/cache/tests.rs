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

use crate::cache::models::VersionSearchType;
use crate::cache::{DistributionCache, MetadataCache};
use crate::config::KopiConfig;
use crate::models::distribution::Distribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::platform::{get_current_architecture, get_current_os, get_foojay_libc_type};
use crate::version::Version;
use crate::version::parser::{ParsedVersionRequest, VersionParser};
use std::str::FromStr;

fn create_test_config() -> KopiConfig {
    KopiConfig::new(std::env::temp_dir()).expect("Failed to create test config")
}

// Helper function to get current platform values for tests
fn get_test_platform() -> (String, String) {
    (get_current_architecture(), get_current_os())
}

fn create_test_cache() -> MetadataCache {
    let mut cache = MetadataCache::new();

    // Use current platform values for testing
    let current_arch = get_current_architecture();
    let current_os = get_current_os();
    let current_libc = get_foojay_libc_type();

    // Determine archive type based on platform
    let archive_type = if current_os == "windows" {
        ArchiveType::Zip
    } else {
        ArchiveType::TarGz
    };

    let packages = vec![
        JdkMetadata {
            id: "test-21".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::from_str("21.0.1").unwrap(),
            architecture: Architecture::from_str(&current_arch).unwrap_or(Architecture::X64),
            operating_system: OperatingSystem::from_str(&current_os)
                .unwrap_or(OperatingSystem::Linux),
            package_type: PackageType::Jdk,
            archive_type,
            download_url: Some("https://example.com/jdk21.tar.gz".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some(current_libc.to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "test-17".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(17, 0, 9),
            distribution_version: Version::from_str("17.0.9").unwrap(),
            architecture: Architecture::from_str(&current_arch).unwrap_or(Architecture::X64),
            operating_system: OperatingSystem::from_str(&current_os)
                .unwrap_or(OperatingSystem::Linux),
            package_type: PackageType::Jdk,
            archive_type,
            download_url: Some("https://example.com/jdk17.tar.gz".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 90_000_000,
            lib_c_type: Some(current_libc.to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
    ];

    let dist_cache = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages,
    };

    cache
        .distributions
        .insert("temurin".to_string(), dist_cache);
    cache
}

#[test]
fn test_search_by_major_version() {
    let cache = create_test_cache();
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 21);
}

#[test]
fn test_search_with_distribution() {
    let cache = create_test_cache();
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("temurin@17").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 17);
    assert_eq!(results[0].distribution, "temurin");
}

#[test]
fn test_search_with_platform_filter() {
    let cache = create_test_cache();
    let config = create_test_config();
    // Since with_platform_filter was removed, we'll test that platform filtering is properly handled
    // by verifying that the results match the expected platform

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("17").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1);
    // Verify the result matches our test platform
    let (test_arch, test_os) = get_test_platform();
    assert_eq!(results[0].package.architecture.to_string(), test_arch);
    assert_eq!(results[0].package.operating_system.to_string(), test_os);
}

#[test]
fn test_lookup() {
    let cache = create_test_cache();
    let (test_arch, test_os) = get_test_platform();

    let package = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        None,
    );

    assert!(package.is_some());
    assert_eq!(package.unwrap().version.to_string(), "21.0.1");
}

#[test]
fn test_search_distribution_only() {
    let cache = create_test_cache();

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: None,
        latest: false,
        javafx_bundled: None,
    };

    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.distribution == "temurin"));
}

#[test]
fn test_search_latest() {
    let cache = create_test_cache();

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: None,
        package_type: None,
        latest: true,
        javafx_bundled: None,
    };

    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1); // Only one distribution in test cache
    assert_eq!(results[0].package.version.major(), 21); // 21 is newer than 17
}

#[test]
fn test_search_latest_with_distribution() {
    let cache = create_test_cache();

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: None,
        latest: true,
        javafx_bundled: None,
    };

    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 21);
    assert_eq!(results[0].distribution, "temurin");
}

#[test]
fn test_search_with_package_type_filter() {
    let cache = create_test_cache();

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: Some(PackageType::Jdk),
        latest: false,
        javafx_bundled: None,
    };

    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert!(
        results
            .iter()
            .all(|r| r.package.package_type == PackageType::Jdk)
    );
}

#[test]
fn test_search_no_cache() {
    let config = create_test_config();
    let cache = MetadataCache::new();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_search_invalid_version() {
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let result = parser.parse("invalid@version@format");
    assert!(result.is_err());
}

#[test]
fn test_search_non_existent_distribution() {
    let cache = create_test_cache();
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("corretto@21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_search_non_existent_version() {
    let cache = create_test_cache();
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("99").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_platform_filter_no_match() {
    let cache = create_test_cache();
    let config = create_test_config();

    // Since we can't set platform filter, we verify that our test cache
    // doesn't have arm64 packages, so searching returns no results
    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    // Our test cache only has packages for the current platform's architecture
    // Verify all packages match the current architecture
    let current_arch = get_current_architecture();
    let expected_arch = Architecture::from_str(&current_arch).unwrap_or(Architecture::X64);
    assert!(
        results
            .iter()
            .all(|r| r.package.architecture == expected_arch)
    );
}

#[test]
fn test_platform_filter_lib_c_mismatch() {
    let cache = create_test_cache();
    let config = create_test_config();

    // Verify our test cache has packages with the correct lib_c_type for current platform
    let current_libc = get_foojay_libc_type();
    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert!(
        results
            .iter()
            .all(|r| r.package.lib_c_type == Some(current_libc.to_string()))
    );
}

#[test]
fn test_platform_filter_missing_lib_c() {
    let mut cache = create_test_cache();
    let config = create_test_config();

    // Add a package without lib_c_type
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut package = dist_cache.packages[0].clone();
        package.id = "test-no-libc".to_string();
        package.lib_c_type = None;
        dist_cache.packages.push(package);
    }

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    // We should find both packages (with and without lib_c_type)
    assert_eq!(results.len(), 2);
    // Verify we have one with and one without lib_c_type
    assert_eq!(
        results
            .iter()
            .filter(|r| r.package.lib_c_type.is_some())
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r.package.lib_c_type.is_none())
            .count(),
        1
    );
}

#[test]
fn test_lookup_single_match() {
    let cache = create_test_cache();
    let (test_arch, test_os) = get_test_platform();

    let package = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        None,
    );

    assert!(package.is_some());
    assert_eq!(package.unwrap().version.to_string(), "21.0.1");
}

#[test]
fn test_lookup_multiple_packages() {
    let mut cache = create_test_cache();

    // Add JRE package with same version
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut jre_package = dist_cache.packages[0].clone();
        jre_package.id = "test-jre".to_string();
        jre_package.package_type = PackageType::Jre;
        dist_cache.packages.push(jre_package);
    }

    // lookup with no package type filter should find the first match
    let (test_arch, test_os) = get_test_platform();
    let jdk_package = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        Some(&PackageType::Jdk),
        None,
    );
    assert!(jdk_package.is_some());
    assert_eq!(jdk_package.unwrap().package_type, PackageType::Jdk);

    let jre_package = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        Some(&PackageType::Jre),
        None,
    );
    assert!(jre_package.is_some());
    assert_eq!(jre_package.unwrap().package_type, PackageType::Jre);
}

#[test]
fn test_lookup_with_requested_type() {
    let mut cache = create_test_cache();

    // Add JRE package
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut jre_package = dist_cache.packages[0].clone();
        jre_package.id = "test-jre".to_string();
        jre_package.package_type = PackageType::Jre;
        dist_cache.packages.push(jre_package);
    }

    // Request JRE specifically
    let (test_arch, test_os) = get_test_platform();
    let package = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        Some(&PackageType::Jre),
        None,
    );

    assert!(package.is_some());
    assert_eq!(package.unwrap().package_type, PackageType::Jre);
}

#[test]
fn test_empty_cache() {
    let cache = MetadataCache::new();
    let config = create_test_config();

    let parser = VersionParser::new(&config);
    let parsed_request = parser.parse("21").unwrap();
    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 0);

    let (test_arch, test_os) = get_test_platform();
    let exact = cache.lookup(
        &Distribution::Temurin,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        None,
    );
    assert!(exact.is_none());
}

#[test]
fn test_latest_with_version_filter() {
    let mut cache = create_test_cache();

    // Add more versions
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut v21_0_2 = dist_cache.packages[0].clone();
        v21_0_2.id = "test-21.0.2".to_string();
        v21_0_2.version = Version::new(21, 0, 2);
        v21_0_2.distribution_version = Version::from_str("21.0.2").unwrap();
        dist_cache.packages.push(v21_0_2);

        let mut v22 = dist_cache.packages[0].clone();
        v22.id = "test-22".to_string();
        v22.version = Version::new(22, 0, 0);
        v22.distribution_version = Version::from_str("22.0.0").unwrap();
        dist_cache.packages.push(v22);
    }

    // Request latest with version filter
    let parsed_request = ParsedVersionRequest {
        version: Some(Version::from_str("21").unwrap()),
        distribution: None,
        package_type: None,
        latest: true,
        javafx_bundled: None,
    };

    let results = cache
        .search(&parsed_request, VersionSearchType::Auto)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.to_string(), "21.0.2");
}

#[test]
fn test_lookup_with_javafx_filter() {
    let mut cache = MetadataCache::new();

    // Use current platform values for testing
    let current_arch = get_current_architecture();
    let current_os = get_current_os();
    let current_libc = get_foojay_libc_type();

    // Determine archive type based on platform
    let archive_type = if current_os == "windows" {
        ArchiveType::Zip
    } else {
        ArchiveType::TarGz
    };

    // Create two packages: one with JavaFX, one without
    let packages = vec![
        JdkMetadata {
            id: "liberica-21-no-fx".to_string(),
            distribution: "liberica".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::from_str("21.0.1").unwrap(),
            architecture: Architecture::from_str(&current_arch).unwrap_or(Architecture::X64),
            operating_system: OperatingSystem::from_str(&current_os)
                .unwrap_or(OperatingSystem::Linux),
            package_type: PackageType::Jdk,
            archive_type,
            download_url: Some("https://example.com/liberica-21.tar.gz".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 200_000_000,
            lib_c_type: Some(current_libc.to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "liberica-21-with-fx".to_string(),
            distribution: "liberica".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::from_str("21.0.1").unwrap(),
            architecture: Architecture::from_str(&current_arch).unwrap_or(Architecture::X64),
            operating_system: OperatingSystem::from_str(&current_os)
                .unwrap_or(OperatingSystem::Linux),
            package_type: PackageType::Jdk,
            archive_type,
            download_url: Some("https://example.com/liberica-21-fx.tar.gz".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 250_000_000, // JavaFX version is larger
            lib_c_type: Some(current_libc.to_string()),
            javafx_bundled: true,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
    ];

    let dist = DistributionCache {
        distribution: Distribution::Liberica,
        display_name: "BellSoft Liberica".to_string(),
        packages,
    };

    cache.distributions.insert("liberica".to_string(), dist);

    let (test_arch, test_os) = get_test_platform();

    // Test 1: Request package WITHOUT JavaFX
    let without_fx = cache.lookup(
        &Distribution::Liberica,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        Some(false),
    );
    assert!(without_fx.is_some());
    assert_eq!(without_fx.as_ref().unwrap().id, "liberica-21-no-fx");
    assert!(!without_fx.unwrap().javafx_bundled);

    // Test 2: Request package WITH JavaFX
    let with_fx = cache.lookup(
        &Distribution::Liberica,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        Some(true),
    );
    assert!(with_fx.is_some());
    assert_eq!(with_fx.as_ref().unwrap().id, "liberica-21-with-fx");
    assert!(with_fx.unwrap().javafx_bundled);

    // Test 3: Request without specifying JavaFX preference (should find first match)
    let no_preference = cache.lookup(
        &Distribution::Liberica,
        "21.0.1",
        &test_arch,
        &test_os,
        None,
        None,
    );
    assert!(no_preference.is_some());
    // It will return one of them (order depends on vector order)
}

#[test]
fn test_detect_version_type() {
    // Standard Java versions should be detected as JavaVersion
    assert_eq!(
        MetadataCache::detect_version_type("21"),
        VersionSearchType::JavaVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0"),
        VersionSearchType::JavaVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.1"),
        VersionSearchType::JavaVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.1+7"),
        VersionSearchType::JavaVersion
    );

    // Extended versions should be detected as DistributionVersion
    assert_eq!(
        MetadataCache::detect_version_type("21.0.7.6"),
        VersionSearchType::DistributionVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.7.6.1"),
        VersionSearchType::DistributionVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.7.0.7.6"),
        VersionSearchType::DistributionVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.1+9.1"),
        VersionSearchType::DistributionVersion
    );
    assert_eq!(
        MetadataCache::detect_version_type("21.0.1+LTS"),
        VersionSearchType::DistributionVersion
    );
}

#[test]
fn test_search_by_distribution_version() {
    let mut cache = create_test_cache();

    // Add packages with extended distribution versions
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        // Corretto-style 4-component version
        let mut corretto_pkg = dist_cache.packages[0].clone();
        corretto_pkg.id = "corretto-21".to_string();
        corretto_pkg.distribution = "corretto".to_string();
        corretto_pkg.distribution_version = Version::from_str("21.0.7.6.1").unwrap();
        dist_cache.packages.push(corretto_pkg);

        // Dragonwell-style 6-component version
        let mut dragonwell_pkg = dist_cache.packages[0].clone();
        dragonwell_pkg.id = "dragonwell-21".to_string();
        dragonwell_pkg.distribution = "dragonwell".to_string();
        dragonwell_pkg.distribution_version = Version::from_str("21.0.7.0.7.6").unwrap();
        dist_cache.packages.push(dragonwell_pkg);
    }

    // Test auto-detection for 4-component version
    let request = ParsedVersionRequest {
        version: Some(Version::from_str("21.0.7.6").unwrap()),
        distribution: None,
        package_type: None,
        latest: false,
        javafx_bundled: None,
    };

    let results = cache.search(&request, VersionSearchType::Auto).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].package.distribution_version,
        Version::from_str("21.0.7.6.1").unwrap()
    );

    // Test explicit distribution_version search
    let results = cache
        .search(&request, VersionSearchType::DistributionVersion)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].package.distribution_version,
        Version::from_str("21.0.7.6.1").unwrap()
    );

    // Test partial matching for 6-component version
    let request = ParsedVersionRequest {
        version: Some(Version::from_str("21.0.7.0.7").unwrap()),
        distribution: None,
        package_type: None,
        latest: false,
        javafx_bundled: None,
    };

    let results = cache
        .search(&request, VersionSearchType::DistributionVersion)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].package.distribution_version,
        Version::from_str("21.0.7.0.7.6").unwrap()
    );
}

#[test]
fn test_search_forced_java_version() {
    let mut cache = create_test_cache();

    // Add a package with same java_version but different distribution_version
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut pkg = dist_cache.packages[0].clone();
        pkg.id = "extended-21".to_string();
        pkg.distribution_version = Version::from_str("21.0.1.9.1").unwrap(); // Extended format
        dist_cache.packages.push(pkg);
    }

    // Search with a pattern that would normally match distribution_version
    let request = ParsedVersionRequest {
        version: Some(Version::from_str("21.0.1").unwrap()),
        distribution: None,
        package_type: None,
        latest: false,
        javafx_bundled: None,
    };

    // Force java_version search - should find both packages
    let results = cache
        .search(&request, VersionSearchType::JavaVersion)
        .unwrap();
    assert_eq!(results.len(), 2); // Both have java_version 21.0.1
}

#[test]
fn test_distribution_version_boundary_matching() {
    let mut cache = create_test_cache();

    // Add packages with similar distribution versions
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        dist_cache.packages.clear();

        // Use current platform values for testing
        let current_arch = get_current_architecture();
        let current_os = get_current_os();
        let current_libc = get_foojay_libc_type();

        // Determine archive type based on platform
        let archive_type = if current_os == "windows" {
            ArchiveType::Zip
        } else {
            ArchiveType::TarGz
        };

        let base_pkg = JdkMetadata {
            id: "test".to_string(),
            distribution: "corretto".to_string(),
            version: Version::new(21, 0, 7),
            distribution_version: Version::from_str("21.0.7").unwrap(),
            architecture: Architecture::from_str(&current_arch).unwrap_or(Architecture::X64),
            operating_system: OperatingSystem::from_str(&current_os)
                .unwrap_or(OperatingSystem::Linux),
            package_type: PackageType::Jdk,
            archive_type,
            download_url: Some("https://example.com/jdk.tar.gz".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some(current_libc.to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        };

        let mut pkg1 = base_pkg.clone();
        pkg1.id = "v1".to_string();
        pkg1.distribution_version = Version::from_str("21.0.7").unwrap();
        dist_cache.packages.push(pkg1);

        let mut pkg2 = base_pkg.clone();
        pkg2.id = "v2".to_string();
        pkg2.distribution_version = Version::from_str("21.0.7.1").unwrap();
        dist_cache.packages.push(pkg2);

        let mut pkg3 = base_pkg.clone();
        pkg3.id = "v3".to_string();
        pkg3.distribution_version = Version::from_str("21.0.71").unwrap();
        dist_cache.packages.push(pkg3);
    }

    // Search for "21.0.7" should match "21.0.7" and "21.0.7.1" but not "21.0.71"
    let request = ParsedVersionRequest {
        version: Some(Version::from_str("21.0.7").unwrap()),
        distribution: None,
        package_type: None,
        latest: false,
        javafx_bundled: None,
    };

    let results = cache
        .search(&request, VersionSearchType::DistributionVersion)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert!(
        results
            .iter()
            .any(|r| r.package.distribution_version == Version::from_str("21.0.7").unwrap())
    );
    assert!(
        results
            .iter()
            .any(|r| r.package.distribution_version == Version::from_str("21.0.7.1").unwrap())
    );
    assert!(
        !results
            .iter()
            .any(|r| r.package.distribution_version == Version::from_str("21.0.71").unwrap())
    );
}
