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

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use kopi::config::KopiConfig;
use kopi::shim::security::SecurityValidator;
use kopi::shim::tools::ToolRegistry;
use kopi::version::resolver::VersionResolver;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

fn benchmark_tool_detection(c: &mut Criterion) {
    let registry = ToolRegistry::new();

    c.bench_function("tool_detection_standard", |b| {
        b.iter(|| registry.get_tool(black_box("java")).is_some())
    });

    c.bench_function("tool_detection_vendor_specific", |b| {
        b.iter(|| registry.get_tool(black_box("native-image")).is_some())
    });

    c.bench_function("tool_detection_nonexistent", |b| {
        b.iter(|| registry.get_tool(black_box("nonexistent-tool")).is_some())
    });

    // Benchmark multiple tool lookups
    let tools = vec!["java", "javac", "jar", "jshell", "native-image"];
    c.bench_function("tool_detection_batch", |b| {
        b.iter(|| {
            for tool in &tools {
                black_box(registry.get_tool(tool).is_some());
            }
        })
    });
}

fn benchmark_version_resolution(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create test directory structure with version files
    let project_dir = temp_dir.path().join("project");
    let nested_dir = project_dir.join("src").join("main");
    fs::create_dir_all(&nested_dir).unwrap();

    // Create version files at different levels
    fs::write(temp_dir.path().join(".kopi-version"), "temurin@21").unwrap();
    fs::write(project_dir.join(".java-version"), "11").unwrap();

    // Benchmark with no version file (uses environment or default)
    c.bench_function("version_resolution_no_file", |b| {
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::new(&config);
        b.iter(|| {
            std::env::set_current_dir(temp_dir.path()).unwrap();
            black_box(resolver.resolve_version())
        })
    });

    // Benchmark with version file in current directory
    c.bench_function("version_resolution_current_dir", |b| {
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::new(&config);
        b.iter(|| {
            std::env::set_current_dir(&project_dir).unwrap();
            black_box(resolver.resolve_version())
        })
    });

    // Benchmark with version file in parent directory
    c.bench_function("version_resolution_parent_dir", |b| {
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::new(&config);
        b.iter(|| {
            std::env::set_current_dir(&nested_dir).unwrap();
            black_box(resolver.resolve_version())
        })
    });

    // Benchmark with environment variable override
    c.bench_function("version_resolution_env_override", |b| {
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::new(&config);
        unsafe {
            std::env::set_var("KOPI_JAVA_VERSION", "corretto@17");
        }
        b.iter(|| {
            std::env::set_current_dir(&nested_dir).unwrap();
            black_box(resolver.resolve_version())
        });
        unsafe {
            std::env::remove_var("KOPI_JAVA_VERSION");
        }
    });
}

fn benchmark_security_validation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path()).unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
    let validator = SecurityValidator::new(&config);

    // Create test paths
    let safe_path = temp_dir.path().join("jdks").join("java-21");
    fs::create_dir_all(&safe_path).unwrap();

    c.bench_function("security_path_validation_safe", |b| {
        b.iter(|| black_box(validator.validate_path(&safe_path)))
    });

    c.bench_function("security_version_validation", |b| {
        b.iter(|| black_box(validator.validate_version("temurin@21.0.1")))
    });

    c.bench_function("security_tool_validation", |b| {
        b.iter(|| black_box(validator.validate_tool("java")))
    });

    // Benchmark multiple validations together
    c.bench_function("security_combined_validation", |b| {
        b.iter(|| {
            let _ = black_box(validator.validate_tool("java"));
            let _ = black_box(validator.validate_version("21"));
            let _ = black_box(validator.validate_path(&safe_path));
        })
    });
}

fn benchmark_total_overhead(c: &mut Criterion) {
    // This benchmark simulates the full shim overhead
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path()).unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

    // Set up test environment
    let jdk_dir = temp_dir.path().join("jdks").join("temurin-21");
    let bin_dir = jdk_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(temp_dir.path().join(".kopi-version"), "temurin@21").unwrap();

    c.bench_function("shim_total_overhead", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate shim operations
            let registry = ToolRegistry::new();
            let validator = SecurityValidator::new(&config);
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
            let resolver = VersionResolver::new(&config);

            // Tool detection
            let tool = "java";
            black_box(registry.get_tool(tool).is_some());
            let _ = black_box(validator.validate_tool(tool));

            // Version resolution
            let version_result = black_box(resolver.resolve_version());
            if let Ok((version_req, _source)) = version_result {
                let _ = black_box(validator.validate_version(&version_req.version_pattern));
            }

            // Path validation
            let tool_path = bin_dir.join("java");
            let _ = black_box(validator.validate_path(&tool_path));

            black_box(start.elapsed())
        })
    });
}

fn benchmark_tool_registry_initialization(c: &mut Criterion) {
    c.bench_function("tool_registry_new", |b| {
        b.iter(|| black_box(ToolRegistry::new()))
    });
}

fn benchmark_version_string_parsing(c: &mut Criterion) {
    let test_versions = vec![
        ("simple", "21"),
        ("with_minor", "21.0.1"),
        ("with_distribution", "temurin@21.0.1"),
        ("complex", "graalvm-ce@22.3.0+java11"),
    ];

    let mut group = c.benchmark_group("version_string_parsing");

    for (name, version) in test_versions {
        group.bench_with_input(BenchmarkId::from_parameter(name), &version, |b, version| {
            let temp_dir = TempDir::new().unwrap();
            std::fs::create_dir_all(temp_dir.path()).unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
            let validator = SecurityValidator::new(&config);

            b.iter(|| black_box(validator.validate_version(version)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_tool_detection,
    benchmark_version_resolution,
    benchmark_security_validation,
    benchmark_total_overhead,
    benchmark_tool_registry_initialization,
    benchmark_version_string_parsing
);
criterion_main!(benches);
