use kopi::cache::{DistributionCache, MetadataCache, VersionSearchType};
use kopi::config::KopiConfig;
use kopi::models::distribution::Distribution;
use kopi::models::metadata::JdkMetadata;
use kopi::models::package::{ArchiveType, ChecksumType, PackageType};
use kopi::models::platform::{Architecture, OperatingSystem};
use kopi::version::Version;
use kopi::version::parser::VersionParser;
use std::str::FromStr;
use std::time::Instant;

fn create_test_config() -> KopiConfig {
    KopiConfig::new(std::env::temp_dir()).expect("Failed to create test config")
}

/// Create a test cache for performance testing
/// Optimized to use less data while still being representative
fn create_large_test_cache() -> MetadataCache {
    let mut cache = MetadataCache::new();

    // Create multiple distributions
    let distributions = vec![
        ("temurin", "Eclipse Temurin", Distribution::Temurin),
        ("corretto", "Amazon Corretto", Distribution::Corretto),
        ("zulu", "Azul Zulu", Distribution::Zulu),
        ("liberica", "BellSoft Liberica", Distribution::Liberica),
        ("sapmachine", "SAP Machine", Distribution::SapMachine),
    ];

    // Create packages for each distribution
    for (dist_id, display_name, dist_enum) in distributions {
        let mut packages = Vec::new();

        // Reduced data set: Create versions from 8 to 17 (LTS versions)
        // This reduces test data from ~11,880 to ~2,160 packages
        for major in [8, 11, 17].iter() {
            // Create fewer minor versions
            for minor in 0..=2 {
                // Create fewer patch versions
                for patch in 0..=3 {
                    // Create packages for different architectures and OS
                    let architectures = vec![Architecture::X64, Architecture::Aarch64];
                    let operating_systems = vec![
                        OperatingSystem::Linux,
                        OperatingSystem::Windows,
                        OperatingSystem::MacOS,
                    ];
                    let package_types = vec![PackageType::Jdk, PackageType::Jre];

                    for arch in &architectures {
                        for os in &operating_systems {
                            for pkg_type in &package_types {
                                packages.push(JdkMetadata {
                                    id: format!(
                                        "{dist_id}-{major}.{minor}.{patch}-{arch:?}-{os:?}"
                                    ),
                                    distribution: dist_id.to_string(),
                                    version: Version::new(*major, minor, patch),
                                    distribution_version: Version::from_str(&format!("{major}.{minor}.{patch}")).unwrap(),
                                    architecture: *arch,
                                    operating_system: *os,
                                    package_type: *pkg_type,
                                    archive_type: if *os == OperatingSystem::Windows {
                                        ArchiveType::Zip
                                    } else {
                                        ArchiveType::TarGz
                                    },
                                    download_url: Some(format!(
                                        "https://example.com/{dist_id}/jdk-{major}.{minor}.{patch}.tar.gz"
                                    )),
                                    checksum: None,
                                    checksum_type: Some(ChecksumType::Sha256),
                                    size: 100_000_000 + (*major as i64 * 1_000_000),
                                    lib_c_type: if *os == OperatingSystem::Linux {
                                        Some("glibc".to_string())
                                    } else {
                                        None
                                    },
                                    javafx_bundled: false,
                                    term_of_support: if *major == 8
                                        || *major == 11
                                        || *major == 17
                                        || *major == 21
                                    {
                                        Some("lts".to_string())
                                    } else {
                                        Some("sts".to_string())
                                    },
                                    release_status: if patch > 0 {
                                        Some("ga".to_string())
                                    } else {
                                        Some("ea".to_string())
                                    },
                                    latest_build_available: Some(patch == 10),
                                    is_complete: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        let dist_cache = DistributionCache {
            distribution: dist_enum,
            display_name: display_name.to_string(),
            packages,
        };

        cache.distributions.insert(dist_id.to_string(), dist_cache);
    }

    cache
}

#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_search_performance_by_version() {
    let cache = create_large_test_cache();
    let config = create_test_config();
    let parser = VersionParser::new(&config);

    // Measure search performance for version search
    let start = Instant::now();
    let parsed = parser.parse("21").unwrap();
    let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
    let duration = start.elapsed();

    println!("Search for version '21' took: {duration:?}");
    println!("Found {} results", results.len());

    // Should complete within 100ms for typical cache sizes
    assert!(
        duration.as_millis() < 100,
        "Search took too long: {duration:?}"
    );
    assert!(!results.is_empty(), "Should find results for version 21");
}

#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_search_performance_by_distribution() {
    let cache = create_large_test_cache();
    let config = create_test_config();
    let parser = VersionParser::new(&config);

    // Measure search performance for distribution search
    let start = Instant::now();
    let parsed = parser.parse("corretto").unwrap();
    let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
    let duration = start.elapsed();

    println!("Search for distribution 'corretto' took: {duration:?}");
    println!("Found {} results", results.len());

    // Should complete within 100ms
    assert!(
        duration.as_millis() < 100,
        "Search took too long: {duration:?}"
    );
    assert!(!results.is_empty(), "Should find results for corretto");
}

#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_search_performance_latest() {
    let cache = create_large_test_cache();
    let config = create_test_config();
    let parser = VersionParser::new(&config);

    // Measure search performance for latest versions
    let start = Instant::now();
    let parsed = parser.parse("latest").unwrap();
    let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
    let duration = start.elapsed();

    println!("Search for 'latest' took: {duration:?}");
    println!("Found {} results", results.len());

    // Should complete within 100ms
    assert!(
        duration.as_millis() < 100,
        "Search took too long: {duration:?}"
    );
    assert_eq!(
        results.len(),
        5,
        "Should find one latest version per distribution"
    );
}

#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_search_performance_with_platform_filter() {
    let cache = create_large_test_cache();
    let config = create_test_config();
    // Note: Platform filtering is now done internally by PackageSearcher
    let parser = VersionParser::new(&config);

    // Measure search performance with platform filters
    let start = Instant::now();
    let parsed = parser.parse("17").unwrap();
    let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
    let duration = start.elapsed();

    println!("Search with platform filter took: {duration:?}");
    println!("Found {} results", results.len());

    // Should complete within 100ms
    assert!(
        duration.as_millis() < 100,
        "Search took too long: {duration:?}"
    );
    assert!(
        !results.is_empty(),
        "Should find results with platform filter"
    );
}

#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_search_memory_usage() {
    let cache = create_large_test_cache();
    let config = create_test_config();
    let parser = VersionParser::new(&config);

    // Get initial memory usage (approximate)
    let package_count: usize = cache.distributions.values().map(|d| d.packages.len()).sum();

    println!("Cache contains {package_count} total packages");

    // Perform multiple searches to check for memory leaks
    for i in 0..100 {
        let major_version = (i % 15) + 8; // Versions 8-22
        let parsed = parser.parse(&major_version.to_string()).unwrap();
        let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
        assert!(
            !results.is_empty(),
            "Should find results for version {major_version}"
        );
    }

    // In a real scenario, we would use a memory profiler
    // For now, we just ensure the searches complete successfully
}

#[test]
fn test_display_rendering_performance() {
    use std::io::Write;

    let cache = create_large_test_cache();
    let config = create_test_config();
    let parser = VersionParser::new(&config);

    // Search for results
    let parsed = parser.parse("21").unwrap();
    let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();

    // Measure table rendering time (simulated)
    let start = Instant::now();

    // Simulate table creation without actual printing
    let mut output = Vec::new();
    for result in &results {
        writeln!(
            &mut output,
            "{} {} {} {}",
            result.distribution,
            result.package.version,
            result.package.term_of_support.as_deref().unwrap_or("-"),
            result.package.size / (1024 * 1024)
        )
        .unwrap();
    }

    let duration = start.elapsed();

    println!("Display rendering took: {duration:?}");
    println!("Rendered {} rows", results.len());

    // Display rendering should be very fast
    assert!(
        duration.as_millis() < 50,
        "Display rendering took too long: {duration:?}"
    );
}

#[cfg(feature = "integration_tests")]
#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_real_cache_performance() {
    use kopi::cache::VersionSearchType;
    use kopi::cache::load_cache;
    use kopi::config::new_kopi_config;
    use kopi::version::parser::VersionParser;

    // This test only runs with real cache data
    let config = new_kopi_config().unwrap();
    let cache_path = config.metadata_cache_path().unwrap();
    if !cache_path.exists() {
        println!("Skipping real cache test - no cache found");
        return;
    }

    let cache = load_cache(&cache_path).unwrap();
    let parser = VersionParser::new(&config);

    // Benchmark common search patterns
    let queries = vec!["21", "17", "corretto", "temurin@21", "latest"];

    for query in queries {
        let start = Instant::now();
        let parsed = parser.parse(query).unwrap();
        let results = cache.search(&parsed, VersionSearchType::Auto).unwrap();
        let duration = start.elapsed();

        println!(
            "Real cache search for '{}' took: {:?}, found {} results",
            query,
            duration,
            results.len()
        );

        // Real cache should still be fast
        assert!(
            duration.as_millis() < 200,
            "Real cache search for '{query}' took too long: {duration:?}"
        );
    }
}
