use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use kopi::{
    api::client::ApiClient,
    metadata::{
        MetadataSource, foojay::FoojayMetadataSource, local::LocalDirectorySource,
        provider::MetadataProvider,
    },
};
use std::sync::Arc;
use tempfile::TempDir;

/// Benchmark API client vs metadata sources
fn benchmark_metadata_sources(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_sources");
    group.measurement_time(std::time::Duration::from_secs(10));

    // Test data
    let distribution = "temurin";
    let version = "21";

    // Direct API client (old implementation)
    group.bench_function("api_client_direct", |b| {
        let client = ApiClient::new();
        b.iter(|| {
            // ApiClient uses get_packages method with a query
            use kopi::api::query::PackageQuery;
            let query = PackageQuery {
                distribution: Some(distribution.to_string()),
                version: Some(version.to_string()),
                architecture: None,
                package_type: None,
                operating_system: None,
                lib_c_type: None,
                archive_types: None,
                latest: None,
                directly_downloadable: None,
                javafx_bundled: None,
            };
            let _ = black_box(client.get_packages(Some(query)));
        });
    });

    // Foojay metadata source (wraps API client)
    group.bench_function("foojay_source", |b| {
        let source = FoojayMetadataSource::new();
        b.iter(|| {
            // FoojayMetadataSource uses fetch_distribution
            let _ = black_box(source.fetch_distribution(black_box(distribution)));
        });
    });

    // Metadata provider with single source
    group.bench_function("provider_single_source", |b| {
        let foojay = Box::new(FoojayMetadataSource::new());
        let provider = MetadataProvider::new_with_source(foojay);
        b.iter(|| {
            let _ = black_box(provider.fetch_distribution(black_box(distribution)));
        });
    });

    group.finish();
}

/// Benchmark concurrent access patterns
fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_metadata_access");

    // Number of concurrent requests to simulate
    let thread_counts = vec![1, 2, 4, 8];

    for thread_count in thread_counts {
        group.bench_with_input(
            BenchmarkId::new("foojay_concurrent", thread_count),
            &thread_count,
            |b, &thread_count| {
                let source = Arc::new(FoojayMetadataSource::new());

                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|i| {
                            let source = Arc::clone(&source);
                            std::thread::spawn(move || {
                                let dist = if i % 2 == 0 { "temurin" } else { "zulu" };
                                let _ = source.fetch_distribution(dist);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark local directory source with test data
fn benchmark_local_source(c: &mut Criterion) {
    // Create test metadata in a temp directory
    let temp_dir = TempDir::new().unwrap();
    let metadata_dir = temp_dir.path().join("metadata");
    std::fs::create_dir(&metadata_dir).unwrap();

    // Create a simple index.json
    let index = serde_json::json!({
        "version": "1.0",
        "generated": "2024-01-01T00:00:00Z",
        "packages": ["temurin", "zulu", "corretto"]
    });
    std::fs::write(
        metadata_dir.join("index.json"),
        serde_json::to_string(&index).unwrap(),
    )
    .unwrap();

    // Create some test metadata files
    for dist in ["temurin", "zulu", "corretto"] {
        let dist_dir = metadata_dir.join(dist);
        std::fs::create_dir(&dist_dir).unwrap();

        let metadata = serde_json::json!({
            "packages": [
                {
                    "distribution": dist,
                    "version": "21.0.1",
                    "architecture": "x64",
                    "operating_system": "linux",
                    "download_url": format!("https://example.com/{}/21.0.1.tar.gz", dist),
                    "checksum": "abc123",
                    "checksum_type": "sha256"
                }
            ]
        });
        std::fs::write(
            dist_dir.join("linux-x64.json"),
            serde_json::to_string(&metadata).unwrap(),
        )
        .unwrap();
    }

    let mut group = c.benchmark_group("local_source");

    group.bench_function("local_directory_search", |b| {
        let source = LocalDirectorySource::new(metadata_dir.to_path_buf());
        b.iter(|| {
            let _ = black_box(source.fetch_distribution(black_box("temurin")));
        });
    });

    group.bench_function("local_directory_fetch_all", |b| {
        let source = LocalDirectorySource::new(metadata_dir.to_path_buf());
        b.iter(|| {
            let _ = black_box(source.fetch_all());
        });
    });

    group.finish();
}

/// Benchmark metadata provider with fallback
fn benchmark_provider_fallback(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider_fallback");

    // Create a provider with multiple sources
    let temp_dir = TempDir::new().unwrap();
    let metadata_dir = temp_dir.path().join("metadata");
    std::fs::create_dir(&metadata_dir).unwrap();

    // Create empty index to make local source fail
    std::fs::write(
        metadata_dir.join("index.json"),
        r#"{"version":"1.0","packages":[]}"#,
    )
    .unwrap();

    group.bench_function("provider_with_fallback", |b| {
        // Local source will fail, should fall back to foojay
        // Since MetadataProvider doesn't have builder pattern, we'll skip this for now
        // TODO: Add benchmark when builder pattern is implemented
        let local = Box::new(LocalDirectorySource::new(metadata_dir.to_path_buf()));
        let provider = MetadataProvider::new_with_source(local);

        b.iter(|| {
            let _ = black_box(provider.fetch_distribution(black_box("temurin")));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_metadata_sources,
    benchmark_concurrent_access,
    benchmark_local_source,
    benchmark_provider_fallback
);
criterion_main!(benches);
