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

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use kopi::archive::detect_jdk_root;
use kopi::models::api::{Links, Package};
use kopi::storage::{InstallationMetadata, InstalledJdk, JdkMetadataWithInstallation};
use kopi::version::Version;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

/// Helper function to create test metadata
fn create_test_metadata(
    distribution: &str,
    version: &str,
    java_home_suffix: &str,
    structure_type: &str,
) -> JdkMetadataWithInstallation {
    JdkMetadataWithInstallation {
        package: Package {
            id: format!("{distribution}-{version}"),
            archive_type: "tar.gz".to_string(),
            distribution: distribution.to_string(),
            major_version: 21,
            java_version: version.to_string(),
            distribution_version: version.to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: format!("{distribution}-{version}.tar.gz"),
            links: Links {
                pkg_download_redirect: "https://example.com/jdk.tar.gz".to_string(),
                pkg_info_uri: None,
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 100000000,
            operating_system: "macos".to_string(),
            architecture: Some("x64".to_string()),
            lib_c_type: None,
            package_type: "jdk".to_string(),
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: Some(true),
        },
        installation_metadata: InstallationMetadata {
            java_home_suffix: java_home_suffix.to_string(),
            structure_type: structure_type.to_string(),
            platform: "macos".to_string(),
            metadata_version: 1,
        },
    }
}

/// Benchmark path resolution with metadata (cache hit scenario)
pub fn benchmark_path_resolution_with_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_resolution_with_metadata");

    // Create test setup with metadata
    group.bench_function("resolve_java_home_cached", |b| {
        b.iter_batched(
            || {
                // Setup
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path();
                let jdk_path = jdks_dir.join("temurin-21.0.1");
                fs::create_dir_all(&jdk_path).unwrap();

                // Create metadata file
                let metadata = create_test_metadata("temurin", "21.0.1", "Contents/Home", "bundle");

                let metadata_file = jdks_dir.join("temurin-21.0.1.meta.json");
                fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                // Create InstalledJdk with pre-loaded cache
                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                // Pre-load the cache by calling resolve_java_home once
                let _ = jdk.resolve_java_home();

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                // Benchmark the cached access
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("resolve_bin_path_cached", |b| {
        b.iter_batched(
            || {
                // Setup similar to above
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path();
                let jdk_path = jdks_dir.join("liberica-21.0.1");
                fs::create_dir_all(&jdk_path).unwrap();

                // Create bin directory for validation
                fs::create_dir_all(jdk_path.join("bin")).unwrap();

                // Create metadata file
                let metadata = create_test_metadata("liberica", "21.0.1", "", "direct");

                let metadata_file = jdks_dir.join("liberica-21.0.1.meta.json");
                fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                let jdk = InstalledJdk::new(
                    "liberica".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                // Pre-load the cache
                let _ = jdk.resolve_bin_path();

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                // Benchmark the cached access
                let _ = black_box(jdk.resolve_bin_path());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark path resolution without metadata (fallback scenario)
pub fn benchmark_path_resolution_without_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_resolution_without_metadata");

    // Direct structure (fastest fallback)
    group.bench_function("resolve_java_home_direct_fallback", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdk_path = temp_dir.path().join("liberica-21.0.1");
                fs::create_dir_all(jdk_path.join("bin")).unwrap();

                let jdk = InstalledJdk::new(
                    "liberica".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    // Bundle structure (requires detection)
    #[cfg(target_os = "macos")]
    group.bench_function("resolve_java_home_bundle_fallback", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdk_path = temp_dir.path().join("temurin-21.0.1");
                let contents_home = jdk_path.join("Contents").join("Home");
                fs::create_dir_all(contents_home.join("bin")).unwrap();

                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark structure detection performance
pub fn benchmark_structure_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("structure_detection");

    // Direct structure detection
    group.bench_function("detect_direct_structure", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdk_path = temp_dir.path();
                fs::create_dir_all(jdk_path.join("bin")).unwrap();
                fs::create_dir_all(jdk_path.join("lib")).unwrap();
                let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
                fs::File::create(jdk_path.join("bin").join(java_binary)).unwrap();
                (jdk_path.to_path_buf(), temp_dir)
            },
            |(jdk_path, _temp_dir)| {
                let _ = black_box(detect_jdk_root(&jdk_path));
            },
            BatchSize::SmallInput,
        );
    });

    // Bundle structure detection
    #[cfg(target_os = "macos")]
    group.bench_function("detect_bundle_structure", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let bundle_path = temp_dir.path();
                let contents_home = bundle_path.join("Contents").join("Home");
                fs::create_dir_all(contents_home.join("bin")).unwrap();
                fs::File::create(contents_home.join("bin").join("java")).unwrap();
                (bundle_path.to_path_buf(), temp_dir)
            },
            |(bundle_path, _temp_dir)| {
                let _ = black_box(detect_jdk_root(&bundle_path));
            },
            BatchSize::SmallInput,
        );
    });

    // Nested structure detection (worst case)
    group.bench_function("detect_nested_structure", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let extracted_dir = temp_dir.path();
                let jdk_subdir = extracted_dir.join("jdk-21.0.1");
                fs::create_dir_all(jdk_subdir.join("bin")).unwrap();
                let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
                fs::File::create(jdk_subdir.join("bin").join(java_binary)).unwrap();
                (extracted_dir.to_path_buf(), temp_dir)
            },
            |(extracted_dir, _temp_dir)| {
                let _ = black_box(detect_jdk_root(&extracted_dir));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark metadata loading performance
pub fn benchmark_metadata_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_loading");

    // Benchmark initial metadata load
    group.bench_function("load_metadata_from_disk", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path();
                let jdk_path = jdks_dir.join("temurin-21.0.1");
                fs::create_dir_all(&jdk_path).unwrap();

                // Create metadata file
                let metadata = create_test_metadata("temurin", "21.0.1", "Contents/Home", "bundle");

                let metadata_file = jdks_dir.join("temurin-21.0.1.meta.json");
                fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                // This will trigger metadata loading
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    // Benchmark parsing corrupted metadata
    group.bench_function("parse_corrupted_metadata", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path();
                let jdk_path = jdks_dir.join("broken-21.0.1");
                fs::create_dir_all(&jdk_path).unwrap();

                // Create corrupted metadata file
                let metadata_file = jdks_dir.join("broken-21.0.1.meta.json");
                fs::write(&metadata_file, "{ invalid json }").unwrap();

                let jdk = InstalledJdk::new(
                    "broken".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                // This will try to parse corrupted metadata and fall back
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark shim startup time with different scenarios
pub fn benchmark_shim_startup_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("shim_startup_time");

    // Simulate shim operations with metadata
    group.bench_function("shim_with_metadata", |b| {
        b.iter_batched(
            || {
                // Setup environment similar to real shim
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path().join("jdks");
                let jdk_path = jdks_dir.join("temurin-21.0.1");
                fs::create_dir_all(jdk_path.join("bin")).unwrap();

                // Create metadata
                let metadata = create_test_metadata("temurin", "21.0.1", "", "direct");

                let metadata_file = jdks_dir.join("temurin-21.0.1.meta.json");
                fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                // Create version file
                fs::write(temp_dir.path().join(".kopi-version"), "temurin@21.0.1").unwrap();

                (temp_dir, jdk_path)
            },
            |(temp_dir, jdk_path)| {
                let start = Instant::now();

                // Simulate shim operations
                // 1. Read version file
                let _version = fs::read_to_string(temp_dir.path().join(".kopi-version")).ok();

                // 2. Create InstalledJdk
                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                // 3. Resolve bin path
                let _ = jdk.resolve_bin_path();

                black_box(start.elapsed());
            },
            BatchSize::SmallInput,
        );
    });

    // Simulate shim operations without metadata
    group.bench_function("shim_without_metadata", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path().join("jdks");
                let jdk_path = jdks_dir.join("temurin-21.0.1");
                fs::create_dir_all(jdk_path.join("bin")).unwrap();

                // No metadata file created

                // Create version file
                fs::write(temp_dir.path().join(".kopi-version"), "temurin@21.0.1").unwrap();

                (temp_dir, jdk_path)
            },
            |(temp_dir, jdk_path)| {
                let start = Instant::now();

                // Simulate shim operations
                // 1. Read version file
                let _version = fs::read_to_string(temp_dir.path().join(".kopi-version")).ok();

                // 2. Create InstalledJdk
                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                // 3. Resolve bin path (will need structure detection)
                let _ = jdk.resolve_bin_path();

                black_box(start.elapsed());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark memory usage patterns
pub fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Benchmark multiple JDK instances with metadata caching
    group.bench_function("multiple_jdks_with_cache", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let mut jdks = Vec::new();

                // Create 10 different JDKs with metadata
                for i in 0..10 {
                    let distribution = format!("dist{i}");
                    let version = Version::new(21, 0, i as u32);
                    let jdk_path = temp_dir.path().join(format!("{distribution}-{version}"));
                    fs::create_dir_all(&jdk_path).unwrap();

                    // Create metadata
                    let metadata = create_test_metadata(
                        &distribution,
                        &version.to_string(),
                        if i % 2 == 0 { "" } else { "Contents/Home" },
                        if i % 2 == 0 { "direct" } else { "bundle" },
                    );

                    let metadata_file = temp_dir
                        .path()
                        .join(format!("{distribution}-{version}.meta.json"));
                    fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                    jdks.push(InstalledJdk::new(distribution, version, jdk_path));
                }

                (jdks, temp_dir)
            },
            |(jdks, _temp_dir)| {
                // Access all JDKs to load metadata into cache
                for jdk in &jdks {
                    let _ = black_box(jdk.resolve_java_home());
                }

                // Simulate repeated access
                for _ in 0..10 {
                    for jdk in &jdks {
                        let _ = black_box(jdk.resolve_java_home());
                    }
                }
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark comparison: before vs after metadata implementation
pub fn benchmark_before_after_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("before_after_comparison");

    // Simulate "before" behavior (always runtime detection)
    group.bench_function("before_always_detect", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdk_path = temp_dir.path().join("temurin-21.0.1");
                let contents_home = jdk_path.join("Contents").join("Home");
                fs::create_dir_all(contents_home.join("bin")).unwrap();
                (jdk_path, temp_dir)
            },
            |(jdk_path, _temp_dir)| {
                // Simulate always doing structure detection
                let _ = black_box(detect_jdk_root(&jdk_path));
            },
            BatchSize::SmallInput,
        );
    });

    // Simulate "after" behavior (cached metadata)
    group.bench_function("after_with_cache", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let jdks_dir = temp_dir.path();
                let jdk_path = jdks_dir.join("temurin-21.0.1");
                fs::create_dir_all(&jdk_path).unwrap();

                // Create metadata
                let metadata = create_test_metadata("temurin", "21.0.1", "Contents/Home", "bundle");

                let metadata_file = jdks_dir.join("temurin-21.0.1.meta.json");
                fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

                let jdk = InstalledJdk::new(
                    "temurin".to_string(),
                    Version::new(21, 0, 1),
                    jdk_path.clone(),
                );

                // Pre-load cache
                let _ = jdk.resolve_java_home();

                (jdk, temp_dir)
            },
            |(jdk, _temp_dir)| {
                // Access with cache
                black_box(jdk.resolve_java_home());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_path_resolution_with_metadata,
    benchmark_path_resolution_without_metadata,
    benchmark_structure_detection,
    benchmark_metadata_loading,
    benchmark_shim_startup_time,
    benchmark_memory_usage,
    benchmark_before_after_comparison
);
criterion_main!(benches);
