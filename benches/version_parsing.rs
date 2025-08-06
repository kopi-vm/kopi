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

use criterion::{BenchmarkId, Criterion, black_box};
use kopi::config::KopiConfig;
use kopi::version::Version;
use kopi::version::parser::VersionParser;
use std::env;
use std::str::FromStr;

pub fn bench_version_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_parsing");

    // Create a config instance for benchmarking
    let config = KopiConfig::new(env::temp_dir()).unwrap();
    let parser = VersionParser::new(&config);

    // Benchmark simple version parsing
    group.bench_function("simple_version", |b| {
        b.iter(|| parser.parse(black_box("21")))
    });

    // Benchmark version with minor
    group.bench_function("version_with_minor", |b| {
        b.iter(|| parser.parse(black_box("21.0")))
    });

    // Benchmark full version
    group.bench_function("full_version", |b| {
        b.iter(|| parser.parse(black_box("21.0.1")))
    });

    // Benchmark complex version with build
    group.bench_function("version_with_build", |b| {
        b.iter(|| parser.parse(black_box("21.0.1+12")))
    });

    // Benchmark version with pre-release
    group.bench_function("version_with_prerelease", |b| {
        b.iter(|| parser.parse(black_box("21.0.1-ea")))
    });

    // Benchmark distribution with version
    group.bench_function("distribution_with_version", |b| {
        b.iter(|| parser.parse(black_box("temurin@21.0.1")))
    });

    // Benchmark latest keyword
    group.bench_function("latest_keyword", |b| {
        b.iter(|| parser.parse(black_box("latest")))
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
