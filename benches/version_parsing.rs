use criterion::{BenchmarkId, Criterion, black_box};
use kopi::models::jdk::Version;
use kopi::version::parser::VersionParser;
use std::str::FromStr;

pub fn bench_version_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_parsing");

    // Benchmark simple version parsing
    group.bench_function("simple_version", |b| {
        b.iter(|| VersionParser::parse(black_box("21")))
    });

    // Benchmark version with minor
    group.bench_function("version_with_minor", |b| {
        b.iter(|| VersionParser::parse(black_box("21.0")))
    });

    // Benchmark full version
    group.bench_function("full_version", |b| {
        b.iter(|| VersionParser::parse(black_box("21.0.1")))
    });

    // Benchmark complex version with build
    group.bench_function("version_with_build", |b| {
        b.iter(|| VersionParser::parse(black_box("21.0.1+12")))
    });

    // Benchmark version with pre-release
    group.bench_function("version_with_prerelease", |b| {
        b.iter(|| VersionParser::parse(black_box("21.0.1-ea")))
    });

    // Benchmark distribution with version
    group.bench_function("distribution_with_version", |b| {
        b.iter(|| VersionParser::parse(black_box("temurin@21.0.1")))
    });

    // Benchmark latest keyword
    group.bench_function("latest_keyword", |b| {
        b.iter(|| VersionParser::parse(black_box("latest")))
    });

    // Benchmark Version parsing (direct from_str)
    let versions = vec![("short", "21"), ("medium", "21.0.1"), ("long", "21.0.1+12")];

    for (name, version) in versions {
        group.bench_with_input(
            BenchmarkId::new("version_from_str", name),
            &version,
            |b, v| b.iter(|| Version::from_str(black_box(v))),
        );
    }

    group.finish();
}
