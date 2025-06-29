use criterion::{criterion_group, criterion_main};

mod cache_operations;
mod search_performance;
mod version_parsing;

use cache_operations::bench_cache_operations;
use search_performance::bench_search_performance;
use version_parsing::bench_version_parsing;

criterion_group!(
    benches,
    bench_version_parsing,
    bench_cache_operations,
    bench_search_performance
);
criterion_main!(benches);
