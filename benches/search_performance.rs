use criterion::{BenchmarkId, Criterion, black_box};
use kopi::cache::{DistributionCache, MetadataCache};
use kopi::cache::{PackageSearcher, VersionSearchType};
use kopi::config::KopiConfig;
use kopi::models::{
    distribution::Distribution,
    metadata::JdkMetadata,
    package::{ArchiveType, ChecksumType, PackageType},
    platform::{Architecture, OperatingSystem},
};
use kopi::version::Version;
use kopi::version::parser::VersionParser;
use std::str::FromStr;

fn create_realistic_cache() -> MetadataCache {
    let mut cache = MetadataCache::new();

    // Create multiple distributions
    let distributions = vec![
        ("temurin", "Eclipse Temurin"),
        ("corretto", "Amazon Corretto"),
        ("zulu", "Azul Zulu"),
    ];

    for (dist_id, display_name) in distributions {
        let mut packages = Vec::new();

        // Create realistic version distribution
        let versions = vec![
            (8, vec![0, 252, 265, 272, 282, 292, 302, 312]),
            (11, vec![0, 8, 9, 10, 11, 12, 13, 14]),
            (17, vec![0, 1, 2, 3, 4, 5, 6, 7]),
            (21, vec![0, 1, 2]),
        ];

        for (major, patches) in versions {
            for patch in patches {
                // Multiple architectures and OS
                for arch in [Architecture::X64, Architecture::Aarch64].iter() {
                    for os in [
                        OperatingSystem::Linux,
                        OperatingSystem::Windows,
                        OperatingSystem::MacOS,
                    ]
                    .iter()
                    {
                        for pkg_type in [PackageType::Jdk, PackageType::Jre].iter() {
                            packages.push(JdkMetadata {
                                id: format!("{dist_id}-{major}.0.{patch}-{arch:?}-{os:?}"),
                                distribution: dist_id.to_string(),
                                version: Version::new(major, 0, patch),
                                distribution_version: Version::new(major, 0, patch),
                                architecture: *arch,
                                operating_system: *os,
                                package_type: *pkg_type,
                                archive_type: if *os == OperatingSystem::Windows {
                                    ArchiveType::Zip
                                } else {
                                    ArchiveType::TarGz
                                },
                                download_url: format!(
                                    "https://example.com/{dist_id}-{major}.0.{patch}.tar.gz"
                                ),
                                checksum: None,
                                checksum_type: Some(ChecksumType::Sha256),
                                size: 100_000_000 + (major as u64 * 1_000_000),
                                lib_c_type: if *os == OperatingSystem::Linux {
                                    Some("glibc".to_string())
                                } else {
                                    None
                                },
                                javafx_bundled: false,
                                term_of_support: if major == 8
                                    || major == 11
                                    || major == 17
                                    || major == 21
                                {
                                    Some("lts".to_string())
                                } else {
                                    Some("sts".to_string())
                                },
                                release_status: Some("ga".to_string()),
                                latest_build_available: Some(true),
                            });
                        }
                    }
                }
            }
        }

        cache.distributions.insert(
            dist_id.to_string(),
            DistributionCache {
                distribution: Distribution::from_str(dist_id).unwrap(),
                display_name: display_name.to_string(),
                packages,
            },
        );
    }

    cache
}

fn create_cache_with_size(size: usize) -> MetadataCache {
    let mut cache = MetadataCache::new();
    let mut packages = Vec::new();

    for i in 0..size {
        let major = 8 + (i / 100) as u32;
        let minor = (i / 10) as u32 % 10;
        let patch = i as u32 % 10;
        packages.push(JdkMetadata {
            id: format!("pkg-{i}"),
            distribution: "temurin".to_string(),
            version: Version::new(major, minor, patch),
            distribution_version: Version::new(major, minor, patch),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: format!("https://example.com/jdk-{major}.{minor}.{patch}.tar.gz"),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: if major == 8 || major == 11 || major == 17 || major == 21 {
                Some("lts".to_string())
            } else {
                Some("sts".to_string())
            },
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        });
    }

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

pub fn bench_search_performance(c: &mut Criterion) {
    // Search performance benchmarks
    let mut group = c.benchmark_group("search_performance");
    let cache = create_realistic_cache();
    let config = KopiConfig::new(std::env::temp_dir()).expect("Failed to create config");
    let searcher = PackageSearcher::new(&cache, &config);
    let parser = VersionParser::new(&config);

    // Benchmark simple version search
    group.bench_function("search_major_version", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("21")).unwrap();
            searcher.search(&parsed, VersionSearchType::Auto)
        })
    });

    // Benchmark exact version search
    group.bench_function("search_exact_version", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("21.0.1")).unwrap();
            searcher.search(&parsed, VersionSearchType::Auto)
        })
    });

    // Benchmark distribution search
    group.bench_function("search_distribution", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("temurin")).unwrap();
            searcher.search(&parsed, VersionSearchType::Auto)
        })
    });

    // Benchmark distribution with version
    group.bench_function("search_distribution_version", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("temurin@21")).unwrap();
            searcher.search(&parsed, VersionSearchType::Auto)
        })
    });

    // Benchmark latest search
    group.bench_function("search_latest", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("latest")).unwrap();
            searcher.search(&parsed, VersionSearchType::Auto)
        })
    });

    // Benchmark search with platform check (we can't filter anymore)
    group.bench_function("search_with_platform_check", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("21")).unwrap();
            let results = searcher.search(&parsed, VersionSearchType::Auto).unwrap();
            // Filter results manually to simulate platform filtering
            results
                .into_iter()
                .filter(|r| {
                    r.package.architecture.to_string() == "x64"
                        && r.package.operating_system.to_string() == "linux"
                        && r.package.lib_c_type.as_deref() == Some("glibc")
                })
                .collect::<Vec<_>>()
        })
    });

    // Benchmark LTS filtering
    group.bench_function("filter_lts_versions", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box("temurin")).unwrap();
            let results = searcher.search(&parsed, VersionSearchType::Auto).unwrap();
            results
                .into_iter()
                .filter(|result| result.package.term_of_support.as_deref() == Some("lts"))
                .collect::<Vec<_>>()
        })
    });

    // Benchmark exact package finding
    group.bench_function("lookup", |b| {
        let dist = Distribution::Temurin;
        b.iter(|| {
            searcher.lookup(
                black_box(&dist),
                black_box("21.0.0"),
                black_box("x64"),
                black_box("linux"),
                black_box(Some(&PackageType::Jdk)),
            )
        })
    });

    group.finish();

    // Cache operations benchmarks
    let mut cache_group = c.benchmark_group("cache_operations");

    // Benchmark has_version with different cache sizes
    let sizes = vec![100, 1000, 5000];
    for size in sizes {
        let cache = create_cache_with_size(size);

        cache_group.bench_with_input(BenchmarkId::new("has_version", size), &cache, |b, cache| {
            b.iter(|| cache.has_version(black_box("21")))
        });
    }

    // Benchmark metadata conversion
    cache_group.bench_function("convert_package_to_metadata", |b| {
        let pkg = JdkMetadata {
            id: "test-pkg".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::new(21, 0, 1),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/jdk-21.0.1.tar.gz".to_string(),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100_000_000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        };
        b.iter(|| {
            // Simulate conversion by cloning
            black_box(pkg.clone())
        })
    });

    // Benchmark cache serialization (to JSON)
    let small_cache = create_cache_with_size(100);
    let medium_cache = create_cache_with_size(1000);

    cache_group.bench_with_input(
        BenchmarkId::new("serialize_cache", "small"),
        &small_cache,
        |b, cache| b.iter(|| serde_json::to_string(black_box(cache))),
    );

    cache_group.bench_with_input(
        BenchmarkId::new("serialize_cache", "medium"),
        &medium_cache,
        |b, cache| b.iter(|| serde_json::to_string(black_box(cache))),
    );

    // Benchmark cache deserialization
    let small_json = serde_json::to_string(&small_cache).unwrap();
    let medium_json = serde_json::to_string(&medium_cache).unwrap();

    cache_group.bench_with_input(
        BenchmarkId::new("deserialize_cache", "small"),
        &small_json,
        |b, json| b.iter(|| serde_json::from_str::<MetadataCache>(black_box(json))),
    );

    cache_group.bench_with_input(
        BenchmarkId::new("deserialize_cache", "medium"),
        &medium_json,
        |b, json| b.iter(|| serde_json::from_str::<MetadataCache>(black_box(json))),
    );

    cache_group.finish();
}
