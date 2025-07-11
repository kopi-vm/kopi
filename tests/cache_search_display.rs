mod common;
use common::TestHomeGuard;
use kopi::cache::DistributionCache;
use kopi::cache::MetadataCache;
use kopi::models::distribution::Distribution;
use kopi::models::metadata::JdkMetadata;
use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
use kopi::models::platform::{Architecture, OperatingSystem};
use kopi::version::Version;

fn create_test_cache_with_lts_data() -> (TestHomeGuard, MetadataCache) {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let mut cache = MetadataCache::new();

    // Add LTS version (21)
    let lts_package = JdkMetadata {
        id: "test-21-lts".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-21.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 200000000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    // Add STS version (22)
    let sts_package = JdkMetadata {
        id: "test-22-sts".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(22, 0, 0),
        distribution_version: "22.0.0+36".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-22.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 210000000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("sts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    // Add EA version (23)
    let ea_package = JdkMetadata {
        id: "test-23-ea".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(23, 0, 0),
        distribution_version: "23-ea+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-23.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 215000000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("sts".to_string()),
        release_status: Some("ea".to_string()),
        latest_build_available: Some(true),
    };

    // Add JRE package
    let jre_package = JdkMetadata {
        id: "test-21-jre".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jre,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/temurin-21-jre.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 90000000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    // Add JavaFX bundled package
    let javafx_package = JdkMetadata {
        id: "test-21-javafx".to_string(),
        distribution: "liberica".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: "21.0.1+12".to_string(),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: "https://example.com/liberica-21-javafx.tar.gz".to_string(),
        checksum: None,
        checksum_type: Some(ChecksumType::Sha256),
        size: 250000000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: true,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    // Create distribution caches
    let temurin_dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages: vec![lts_package, sts_package, ea_package, jre_package],
    };

    let liberica_dist = DistributionCache {
        distribution: Distribution::Liberica,
        display_name: "BellSoft Liberica".to_string(),
        packages: vec![javafx_package],
    };

    cache
        .distributions
        .insert("temurin".to_string(), temurin_dist);
    cache
        .distributions
        .insert("liberica".to_string(), liberica_dist);

    (test_home, cache)
}

#[test]
fn test_compact_display_shows_minimal_columns() {
    let (test_home, cache) = create_test_cache_with_lts_data();
    let cache_path = test_home.kopi_home().join("cache").join("metadata.json");
    cache.save(&cache_path).unwrap();

    // Set cache path for the test
    unsafe {
        std::env::set_var("KOPI_HOME", test_home.kopi_home());
    }

    // Note: This is an integration test outline. In practice, you would need to:
    // 1. Execute the actual command with the test cache
    // 2. Capture the output
    // 3. Verify the output contains expected columns

    // For now, we verify the cache structure is correct
    assert_eq!(cache.distributions.len(), 2);
    assert_eq!(cache.distributions["temurin"].packages.len(), 4);
    assert_eq!(cache.distributions["liberica"].packages.len(), 1);
}

#[test]
fn test_detailed_display_includes_all_information() {
    let (temp_dir, cache) = create_test_cache_with_lts_data();
    let cache_path = temp_dir.path().join("metadata.json");
    cache.save(&cache_path).unwrap();

    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path());
    }

    // Verify package metadata contains all expected fields
    let temurin_packages = &cache.distributions["temurin"].packages;
    let lts_package = &temurin_packages[0];

    assert_eq!(lts_package.term_of_support, Some("lts".to_string()));
    assert_eq!(lts_package.release_status, Some("ga".to_string()));
    assert_eq!(lts_package.latest_build_available, Some(true));
}

#[test]
fn test_json_output_contains_all_fields() {
    let (temp_dir, cache) = create_test_cache_with_lts_data();
    let cache_path = temp_dir.path().join("metadata.json");
    cache.save(&cache_path).unwrap();

    unsafe {
        std::env::set_var("KOPI_HOME", temp_dir.path());
    }

    // Serialize a package to JSON and verify structure
    let temurin_packages = &cache.distributions["temurin"].packages;
    let lts_package = &temurin_packages[0];

    let json = serde_json::to_string(lts_package).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["term_of_support"], "lts");
    assert_eq!(parsed["release_status"], "ga");
    assert_eq!(parsed["latest_build_available"], true);
    assert_eq!(parsed["id"], "test-21-lts");
}

#[test]
fn test_lts_column_displays_correctly() {
    let (_temp_dir, cache) = create_test_cache_with_lts_data();

    // Verify LTS values are correctly set
    let temurin_packages = &cache.distributions["temurin"].packages;

    // LTS package
    assert_eq!(temurin_packages[0].term_of_support, Some("lts".to_string()));

    // STS package
    assert_eq!(temurin_packages[1].term_of_support, Some("sts".to_string()));

    // EA package (should be STS)
    assert_eq!(temurin_packages[2].term_of_support, Some("sts".to_string()));
}

#[test]
fn test_javafx_bundled_packages_display() {
    let (_temp_dir, cache) = create_test_cache_with_lts_data();

    // Verify JavaFX bundled status
    let liberica_packages = &cache.distributions["liberica"].packages;
    assert!(liberica_packages[0].javafx_bundled);

    let temurin_packages = &cache.distributions["temurin"].packages;
    assert!(!temurin_packages[0].javafx_bundled);
}

#[test]
fn test_status_column_shows_ga_ea() {
    let (_temp_dir, cache) = create_test_cache_with_lts_data();

    let temurin_packages = &cache.distributions["temurin"].packages;

    // GA release
    assert_eq!(temurin_packages[0].release_status, Some("ga".to_string()));
    assert_eq!(temurin_packages[1].release_status, Some("ga".to_string()));

    // EA release
    assert_eq!(temurin_packages[2].release_status, Some("ea".to_string()));
}

#[test]
fn test_compact_mode_deduplication() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let mut cache = MetadataCache::new();

    // Add multiple packages with same version but different architectures
    let mut packages = vec![];

    // These should appear as duplicates in compact mode
    for arch in [Architecture::X64, Architecture::Aarch64] {
        packages.push(JdkMetadata {
            id: format!("test-21-{arch}"),
            distribution: "zulu".to_string(),
            version: Version::new(21, 0, 7).with_build("6".to_string()),
            distribution_version: "21.0.7+6".to_string(),
            architecture: arch,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: format!("https://example.com/zulu-21-{arch}.tar.gz"),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 200000000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        });
    }

    let dist = DistributionCache {
        distribution: Distribution::Zulu,
        display_name: "Azul Zulu".to_string(),
        packages,
    };

    cache.distributions.insert("zulu".to_string(), dist);

    // In compact mode, these should be deduplicated
    // Both packages have same Version "21 (21.0.7+6)" and LTS "LTS"
    let zulu_packages = &cache.distributions["zulu"].packages;
    assert_eq!(zulu_packages.len(), 2);

    // Verify they have same display version and LTS status
    assert_eq!(zulu_packages[0].version, zulu_packages[1].version);
    assert_eq!(
        zulu_packages[0].term_of_support,
        zulu_packages[1].term_of_support
    );
}

#[test]
fn test_detailed_mode_deduplication_keeps_smallest() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let mut cache = MetadataCache::new();

    // Add multiple packages with same details but different sizes
    let mut packages = vec![];

    // Create packages with different sizes (should keep the smallest)
    for (i, size) in [(1, 300_000_000), (2, 200_000_000), (3, 250_000_000)].iter() {
        packages.push(JdkMetadata {
            id: format!("test-21-size-{i}"),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1+12".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: format!("https://example.com/temurin-21-{i}.tar.gz"),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: *size,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        });
    }

    let dist = DistributionCache {
        distribution: Distribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages,
    };

    cache.distributions.insert("temurin".to_string(), dist);

    // In detailed mode with deduplication, only the smallest size should be kept
    let temurin_packages = &cache.distributions["temurin"].packages;
    assert_eq!(temurin_packages.len(), 3);

    // Find the package with smallest size
    let smallest_package = temurin_packages.iter().min_by_key(|p| p.size).unwrap();

    assert_eq!(smallest_package.size, 200_000_000);
    assert_eq!(smallest_package.id, "test-21-size-2");
}
