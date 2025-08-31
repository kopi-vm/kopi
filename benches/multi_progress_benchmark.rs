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

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use kopi::indicator::{ProgressConfig, ProgressFactory, ProgressStyle};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn benchmark_single_vs_multi_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vs_multi");

    group.bench_function("single_progress_bar", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut progress| {
                let config = ProgressConfig::new(ProgressStyle::Bytes).with_total(100_000_000);
                progress.start(config);

                // Simulate 100 updates
                for i in 0..100 {
                    progress.update(i * 1_000_000, None);
                    progress.set_message(format!("{i} MB"));
                }

                progress.complete(Some("Done".to_string()));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("parent_with_child_progress", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut parent| {
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(10);
                parent.start(parent_config);

                let mut child = parent.create_child();
                let child_config =
                    ProgressConfig::new(ProgressStyle::Bytes).with_total(100_000_000);
                child.start(child_config);

                // Simulate 100 updates on child
                for i in 0..100 {
                    child.update(i * 1_000_000, None);
                    child.set_message(format!("{i} MB"));

                    // Update parent every 10 iterations
                    if i % 10 == 0 {
                        parent.update(i / 10, None);
                    }
                }

                child.complete(None);
                parent.complete(Some("All done".to_string()));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn benchmark_multiple_children(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_children");

    group.bench_function("three_children_sequential", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut parent| {
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
                parent.start(parent_config);

                for child_idx in 0..3 {
                    let mut child = parent.create_child();
                    let child_config =
                        ProgressConfig::new(ProgressStyle::Bytes).with_total(50_000_000);
                    child.start(child_config);

                    // Simulate 50 updates per child
                    for i in 0..50 {
                        child.update(i * 1_000_000, None);
                        child.set_message(format!("Child {child_idx} - {i} MB"));
                    }

                    child.complete(None);
                    parent.update(child_idx + 1, None);
                }

                parent.complete(None);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("three_children_concurrent", |b| {
        b.iter(|| {
            let parent = Arc::new(std::sync::Mutex::new(ProgressFactory::create(false)));

            {
                let mut parent = parent.lock().unwrap();
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
                parent.start(parent_config);
            }

            let handles: Vec<_> = (0..3)
                .map(|child_idx| {
                    let parent = parent.clone();
                    thread::spawn(move || {
                        let mut child = {
                            let mut parent = parent.lock().unwrap();
                            parent.create_child()
                        };

                        let child_config =
                            ProgressConfig::new(ProgressStyle::Bytes).with_total(50_000_000);
                        child.start(child_config);

                        // Simulate 50 updates per child
                        for i in 0..50 {
                            child.update(i * 1_000_000, None);
                            child.set_message(format!("Child {child_idx} - {i} MB"));
                            // Small sleep to simulate real work
                            thread::sleep(Duration::from_micros(10));
                        }

                        child.complete(None);

                        let mut parent = parent.lock().unwrap();
                        parent.update(child_idx + 1, None);
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            let mut parent = parent.lock().unwrap();
            parent.complete(None);
        });
    });

    group.finish();
}

fn benchmark_update_frequency(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_frequency");

    group.bench_function("high_frequency_updates", |b| {
        b.iter_batched(
            || {
                let mut parent = ProgressFactory::create(false);
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
                parent.start(parent_config);

                let child = parent.create_child();
                (parent, child)
            },
            |(mut parent, mut child)| {
                let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(10_000_000);
                child.start(child_config);

                // Simulate very frequent updates (every 10KB)
                for i in 0..1000 {
                    child.update(i * 10_000, None);
                    if i % 100 == 0 {
                        parent.update(i / 10, None);
                    }
                }

                child.complete(None);
                parent.complete(None);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("throttled_updates", |b| {
        b.iter_batched(
            || {
                let mut parent = ProgressFactory::create(false);
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
                parent.start(parent_config);

                let child = parent.create_child();
                (parent, child)
            },
            |(mut parent, mut child)| {
                let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(10_000_000);
                child.start(child_config);

                // Simulate throttled updates (every 100KB)
                for i in 0..100 {
                    child.update(i * 100_000, None);
                    if i % 10 == 0 {
                        parent.update(i, None);
                    }
                }

                child.complete(None);
                parent.complete(None);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn benchmark_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");

    group.bench_function("create_destroy_children", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut parent| {
                let parent_config = ProgressConfig::new(ProgressStyle::Count);
                parent.start(parent_config);

                // Create and destroy many children
                for _ in 0..10 {
                    let mut child = parent.create_child();
                    let child_config = ProgressConfig::new(ProgressStyle::Count);
                    child.start(child_config);
                    child.complete(None);
                }

                parent.complete(None);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("arc_reference_counting", |b| {
        b.iter(|| {
            let mut parent = ProgressFactory::create(false);
            let parent_config = ProgressConfig::new(ProgressStyle::Count);
            parent.start(parent_config);

            // Create multiple children sharing the same Arc<MultiProgress>
            let children: Vec<_> = (0..5).map(|_| parent.create_child()).collect();

            // Drop all children
            drop(children);

            parent.complete(None);
        });
    });

    group.finish();
}

fn benchmark_real_world_download_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_download");

    group.bench_function("jdk_download_simulation", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut parent| {
                // Parent: Installing temurin@21
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
                parent.start(parent_config);
                parent.set_message("Resolving version".to_string());

                parent.update(1, None);
                parent.set_message("Checking cache".to_string());

                parent.update(2, None);
                parent.set_message("Downloading".to_string());

                // Child: Download progress (195MB JDK)
                let mut child = parent.create_child();
                let child_config =
                    ProgressConfig::new(ProgressStyle::Bytes).with_total(195_000_000);
                child.start(child_config);

                // Simulate download in 1MB chunks
                for mb in 0..195 {
                    child.update(mb * 1_000_000, None);
                    child.set_message(format!("{mb} MB / 195 MB"));

                    // Simulate varying download speeds
                    if mb % 20 == 0 {
                        child.set_message(format!("{mb} MB / 195 MB (2.3 MB/s)"));
                    }
                }

                child.complete(Some("Download complete".to_string()));

                parent.update(3, None);
                parent.set_message("Extracting".to_string());

                parent.update(4, None);
                parent.set_message("Installing".to_string());

                parent.update(5, None);
                parent.set_message("Creating shims".to_string());

                parent.update(6, None);
                parent.set_message("Updating PATH".to_string());

                parent.update(7, None);
                parent.set_message("Verifying installation".to_string());

                parent.update(8, None);
                parent.complete(Some("Installation complete".to_string()));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("cache_refresh_simulation", |b| {
        b.iter_batched(
            || ProgressFactory::create(false),
            |mut parent| {
                // Parent: Refreshing cache
                let parent_config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
                parent.start(parent_config);
                parent.set_message("Refreshing metadata sources".to_string());

                // Child 1: Foojay API
                parent.update(1, None);
                parent.set_message("Fetching from Foojay".to_string());

                let mut child1 = parent.create_child();
                let child_config = ProgressConfig::new(ProgressStyle::Count);
                child1.start(child_config);
                child1.set_message("Fetching package list".to_string());

                for _ in 0..10 {
                    child1.set_message("Processing packages...".to_string());
                }

                child1.complete(Some("2,845 packages fetched".to_string()));

                // Child 2: HTTP metadata
                parent.update(2, None);
                parent.set_message("Fetching from HTTP source".to_string());

                let mut child2 = parent.create_child();
                let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(15_000_000);
                child2.start(child_config);

                for mb in 0..15 {
                    child2.update(mb * 1_000_000, None);
                    child2.set_message(format!("{mb} MB / 15 MB"));
                }

                child2.complete(Some("Metadata downloaded".to_string()));

                parent.update(3, None);
                parent.complete(Some("Cache refresh complete".to_string()));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_vs_multi_progress,
    benchmark_multiple_children,
    benchmark_update_frequency,
    benchmark_memory_overhead,
    benchmark_real_world_download_scenario
);

criterion_main!(benches);
