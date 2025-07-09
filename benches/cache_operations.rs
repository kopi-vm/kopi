use criterion::{BenchmarkId, Criterion, black_box};
use kopi::cache::{DistributionCache, MetadataCache};
use kopi::models::{
    distribution::Distribution as JdkDistribution,
    metadata::JdkMetadata,
    package::{ArchiveType, ChecksumType, PackageType},
    platform::{Architecture, OperatingSystem},
    version::Version,
};

fn create_test_metadata(id: &str, major: u32, minor: u32, patch: u32) -> JdkMetadata {
    JdkMetadata {
        id: id.to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(major, minor, patch),
        distribution_version: format!("{major}.{minor}.{patch}"),
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
    }
}

fn create_cache_with_size(size: usize) -> MetadataCache {
    let mut cache = MetadataCache::new();
    let mut packages = Vec::new();

    for i in 0..size {
        let major = 8 + (i / 100) as u32;
        let minor = (i / 10) as u32 % 10;
        let patch = i as u32 % 10;
        packages.push(create_test_metadata(
            &format!("pkg-{i}"),
            major,
            minor,
            patch,
        ));
    }

    let dist_cache = DistributionCache {
        distribution: JdkDistribution::Temurin,
        display_name: "Eclipse Temurin".to_string(),
        packages,
    };

    cache
        .distributions
        .insert("temurin".to_string(), dist_cache);
    cache
}

pub fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Benchmark has_version with different cache sizes
    let sizes = vec![100, 1000, 5000];
    for size in sizes {
        let cache = create_cache_with_size(size);

        group.bench_with_input(BenchmarkId::new("has_version", size), &cache, |b, cache| {
            b.iter(|| cache.has_version(black_box("21")))
        });
    }

    // Benchmark find_package_in_cache
    let cache = create_cache_with_size(1000);

    group.bench_function("find_exact_match", |b| {
        b.iter(|| {
            cache.find_package(
                black_box("temurin"),
                black_box("11.0.5"),
                black_box("x64"),
                black_box("linux"),
            )
        })
    });

    // Benchmark metadata conversion
    group.bench_function("convert_package_to_metadata", |b| {
        let pkg = create_test_metadata("test-pkg", 21, 0, 1);
        b.iter(|| {
            // Simulate conversion by cloning
            black_box(pkg.clone())
        })
    });

    // Benchmark cache serialization (to JSON)
    let small_cache = create_cache_with_size(100);
    let medium_cache = create_cache_with_size(1000);

    group.bench_with_input(
        BenchmarkId::new("serialize_cache", "small"),
        &small_cache,
        |b, cache| b.iter(|| serde_json::to_string(black_box(cache))),
    );

    group.bench_with_input(
        BenchmarkId::new("serialize_cache", "medium"),
        &medium_cache,
        |b, cache| b.iter(|| serde_json::to_string(black_box(cache))),
    );

    // Benchmark cache deserialization
    let small_json = serde_json::to_string(&small_cache).unwrap();
    let medium_json = serde_json::to_string(&medium_cache).unwrap();

    group.bench_with_input(
        BenchmarkId::new("deserialize_cache", "small"),
        &small_json,
        |b, json| b.iter(|| serde_json::from_str::<MetadataCache>(black_box(json))),
    );

    group.bench_with_input(
        BenchmarkId::new("deserialize_cache", "medium"),
        &medium_json,
        |b, json| b.iter(|| serde_json::from_str::<MetadataCache>(black_box(json))),
    );

    group.finish();
}
