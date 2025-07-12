use super::*;
use crate::cache::{DistributionCache, MetadataCache};
use crate::config::KopiConfig;
use crate::models::distribution::Distribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::version::Version;
use crate::version::parser::ParsedVersionRequest;
use std::str::FromStr;

fn create_test_config() -> KopiConfig {
    KopiConfig::new(std::env::temp_dir()).expect("Failed to create test config")
}

fn create_test_cache() -> MetadataCache {
    let mut cache = MetadataCache::new();

    let packages = vec![
        JdkMetadata {
            id: "test-21".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/jdk21.tar.gz".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        },
        JdkMetadata {
            id: "test-17".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(17, 0, 9),
            distribution_version: "17.0.9".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/jdk17.tar.gz".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 90_000_000,
            lib_c_type: Some("glibc".to_string()),
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
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("21").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 21);
}

#[test]
fn test_search_with_distribution() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("temurin@17").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 17);
    assert_eq!(results[0].distribution, "temurin");
}

#[test]
fn test_search_with_platform_filter() {
    let cache = create_test_cache();
    let config = create_test_config();
    let filter = PlatformFilter {
        architecture: Some("x64".to_string()),
        operating_system: Some("linux".to_string()),
        lib_c_type: Some("glibc".to_string()),
    };
    let searcher = PackageSearcher::new(&cache, &config).with_platform_filter(filter);

    let results = searcher.search("17").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_find_exact_package() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let package =
        searcher.find_exact_package(&Distribution::Temurin, "21.0.1", "x64", "linux", None);

    assert!(package.is_some());
    assert_eq!(package.unwrap().version.to_string(), "21.0.1");
}

#[test]
fn test_search_distribution_only() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: None,
        latest: false,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.distribution == "temurin"));
}

#[test]
fn test_search_latest() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: None,
        package_type: None,
        latest: true,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
    assert_eq!(results.len(), 1); // Only one distribution in test cache
    assert_eq!(results[0].package.version.major(), 21); // 21 is newer than 17
}

#[test]
fn test_search_latest_with_distribution() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: None,
        latest: true,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.major(), 21);
    assert_eq!(results[0].distribution, "temurin");
}

#[test]
fn test_search_with_package_type_filter() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: Some(PackageType::Jdk),
        latest: false,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
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
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("21").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_search_invalid_version() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let result = searcher.search("invalid@version@format");
    assert!(result.is_err());
}

#[test]
fn test_search_non_existent_distribution() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("corretto@21").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_search_non_existent_version() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("99").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_platform_filter_no_match() {
    let cache = create_test_cache();
    let config = create_test_config();
    let filter = PlatformFilter {
        architecture: Some("arm64".to_string()),
        operating_system: Some("linux".to_string()),
        lib_c_type: Some("glibc".to_string()),
    };
    let searcher = PackageSearcher::new(&cache, &config).with_platform_filter(filter);

    let results = searcher.search("21").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_platform_filter_lib_c_mismatch() {
    let cache = create_test_cache();
    let config = create_test_config();
    let filter = PlatformFilter {
        architecture: Some("x64".to_string()),
        operating_system: Some("linux".to_string()),
        lib_c_type: Some("musl".to_string()),
    };
    let searcher = PackageSearcher::new(&cache, &config).with_platform_filter(filter);

    let results = searcher.search("21").unwrap();
    assert_eq!(results.len(), 0);
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

    let filter = PlatformFilter {
        architecture: None,
        operating_system: None,
        lib_c_type: Some("glibc".to_string()),
    };
    let searcher = PackageSearcher::new(&cache, &config).with_platform_filter(filter);

    let results = searcher.search("21").unwrap();
    // Should only find the package with lib_c_type
    assert_eq!(results.len(), 1);
    assert!(results[0].package.lib_c_type.is_some());
}

#[test]
fn test_find_auto_selected_package_single_match() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let package =
        searcher.find_auto_selected_package(&Distribution::Temurin, "21.0.1", "x64", "linux", None);

    assert!(package.is_some());
    assert_eq!(package.unwrap().version.to_string(), "21.0.1");
}

#[test]
fn test_find_auto_selected_package_multiple_packages() {
    let mut cache = create_test_cache();
    let config = create_test_config();

    // Add JRE package with same version
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut jre_package = dist_cache.packages[0].clone();
        jre_package.id = "test-jre".to_string();
        jre_package.package_type = PackageType::Jre;
        dist_cache.packages.push(jre_package);
    }

    let searcher = PackageSearcher::new(&cache, &config);

    // Should prefer JDK over JRE
    let package =
        searcher.find_auto_selected_package(&Distribution::Temurin, "21.0.1", "x64", "linux", None);

    assert!(package.is_some());
    assert_eq!(package.unwrap().package_type, PackageType::Jdk);
}

#[test]
fn test_find_auto_selected_package_with_requested_type() {
    let mut cache = create_test_cache();
    let config = create_test_config();

    // Add JRE package
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut jre_package = dist_cache.packages[0].clone();
        jre_package.id = "test-jre".to_string();
        jre_package.package_type = PackageType::Jre;
        dist_cache.packages.push(jre_package);
    }

    let searcher = PackageSearcher::new(&cache, &config);

    // Request JRE specifically
    let package = searcher.find_auto_selected_package(
        &Distribution::Temurin,
        "21.0.1",
        "x64",
        "linux",
        Some(PackageType::Jre),
    );

    assert!(package.is_some());
    assert_eq!(package.unwrap().package_type, PackageType::Jre);
}

#[test]
fn test_search_refs_produces_same_results() {
    let cache = create_test_cache();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let parsed_request = ParsedVersionRequest {
        version: None,
        distribution: Some(Distribution::Temurin),
        package_type: None,
        latest: false,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
    let ref_results = searcher.search_parsed_refs(&parsed_request).unwrap();

    assert_eq!(results.len(), ref_results.len());
    for (result, ref_result) in results.iter().zip(ref_results.iter()) {
        assert_eq!(result.distribution, ref_result.distribution);
        assert_eq!(result.display_name, ref_result.display_name);
        assert_eq!(result.package.id, ref_result.package.id);
    }
}

#[test]
fn test_empty_cache() {
    let cache = MetadataCache::new();
    let config = create_test_config();
    let searcher = PackageSearcher::new(&cache, &config);

    let results = searcher.search("21").unwrap();
    assert_eq!(results.len(), 0);

    let exact = searcher.find_exact_package(&Distribution::Temurin, "21.0.1", "x64", "linux", None);
    assert!(exact.is_none());
}

#[test]
fn test_latest_with_version_filter() {
    let mut cache = create_test_cache();
    let config = create_test_config();

    // Add more versions
    if let Some(dist_cache) = cache.distributions.get_mut("temurin") {
        let mut v21_0_2 = dist_cache.packages[0].clone();
        v21_0_2.id = "test-21.0.2".to_string();
        v21_0_2.version = Version::new(21, 0, 2);
        v21_0_2.distribution_version = "21.0.2".to_string();
        dist_cache.packages.push(v21_0_2);

        let mut v22 = dist_cache.packages[0].clone();
        v22.id = "test-22".to_string();
        v22.version = Version::new(22, 0, 0);
        v22.distribution_version = "22.0.0".to_string();
        dist_cache.packages.push(v22);
    }

    let searcher = PackageSearcher::new(&cache, &config);

    // Request latest with version filter
    let parsed_request = ParsedVersionRequest {
        version: Some(Version::from_str("21").unwrap()),
        distribution: None,
        package_type: None,
        latest: true,
    };

    let results = searcher.search_parsed(&parsed_request).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].package.version.to_string(), "21.0.2");
}
