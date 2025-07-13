use criterion::{Criterion, black_box};
use kopi::cache::{DistributionCache, MetadataCache};
use kopi::config::KopiConfig;
use kopi::models::{
    distribution::Distribution,
    metadata::JdkMetadata,
    package::{ArchiveType, ChecksumType, PackageType},
    platform::{Architecture, OperatingSystem},
};
use kopi::search::{PackageSearcher, PlatformFilter};
use kopi::version::Version;
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

pub fn bench_search_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_performance");
    let cache = create_realistic_cache();
    let config = KopiConfig::new(std::env::temp_dir()).expect("Failed to create config");
    let searcher = PackageSearcher::new(&cache, &config);

    // Benchmark simple version search
    group.bench_function("search_major_version", |b| {
        b.iter(|| searcher.search(black_box("21")))
    });

    // Benchmark exact version search
    group.bench_function("search_exact_version", |b| {
        b.iter(|| searcher.search(black_box("21.0.1")))
    });

    // Benchmark distribution search
    group.bench_function("search_distribution", |b| {
        b.iter(|| searcher.search(black_box("temurin")))
    });

    // Benchmark distribution with version
    group.bench_function("search_distribution_version", |b| {
        b.iter(|| searcher.search(black_box("temurin@21")))
    });

    // Benchmark latest search
    group.bench_function("search_latest", |b| {
        b.iter(|| searcher.search(black_box("latest")))
    });

    // Benchmark search with platform filter
    let platform_filter = PlatformFilter {
        architecture: Some("x64".to_string()),
        operating_system: Some("linux".to_string()),
        lib_c_type: Some("glibc".to_string()),
    };

    group.bench_function("search_with_platform_filter", |b| {
        let searcher_with_filter =
            PackageSearcher::new(&cache, &config).with_platform_filter(platform_filter.clone());
        b.iter(|| searcher_with_filter.search(black_box("21")))
    });

    // Benchmark LTS filtering
    group.bench_function("filter_lts_versions", |b| {
        b.iter(|| {
            let results = searcher.search(black_box("temurin")).unwrap();
            results
                .into_iter()
                .filter(|result| result.package.term_of_support.as_deref() == Some("lts"))
                .collect::<Vec<_>>()
        })
    });

    // Benchmark auto-selection
    group.bench_function("auto_select_package", |b| {
        let dist = Distribution::Temurin;
        b.iter(|| {
            searcher.find_auto_selected_package(
                black_box(&dist),
                black_box("21.0.0"),
                black_box("x64"),
                black_box("linux"),
                black_box(Some(PackageType::Jdk)),
            )
        })
    });

    group.finish();
}
