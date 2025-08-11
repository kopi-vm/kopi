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

use criterion::{criterion_group, criterion_main};

mod path_resolution;
mod search_performance;
mod version_parsing;

use path_resolution::{
    benchmark_before_after_comparison, benchmark_memory_usage, benchmark_metadata_loading,
    benchmark_path_resolution_with_metadata, benchmark_path_resolution_without_metadata,
    benchmark_shim_startup_time, benchmark_structure_detection,
};
use search_performance::bench_search_performance;
use version_parsing::bench_version_parsing;

criterion_group!(
    benches,
    bench_version_parsing,
    bench_search_performance,
    benchmark_path_resolution_with_metadata,
    benchmark_path_resolution_without_metadata,
    benchmark_structure_detection,
    benchmark_metadata_loading,
    benchmark_shim_startup_time,
    benchmark_memory_usage,
    benchmark_before_after_comparison
);
criterion_main!(benches);
