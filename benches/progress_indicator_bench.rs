use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use kopi::indicator::factory::ProgressFactory;
use kopi::indicator::status::StatusReporter;
use kopi::indicator::{ProgressConfig, ProgressStyle};

fn benchmark_progress_factory_creation(c: &mut Criterion) {
    c.bench_function("progress_factory_create_silent", |b| {
        b.iter(|| black_box(ProgressFactory::create(true)));
    });

    c.bench_function("progress_factory_create_normal", |b| {
        b.iter(|| black_box(ProgressFactory::create(false)));
    });
}

fn benchmark_progress_operations(c: &mut Criterion) {
    c.bench_function("progress_start_complete_silent", |b| {
        let mut progress = ProgressFactory::create(true);
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(1000);

        b.iter(|| {
            progress.start(config.clone());
            progress.complete(None);
        });
    });

    c.bench_function("progress_update_silent", |b| {
        let mut progress = ProgressFactory::create(true);
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(1000);

        progress.start(config);

        b.iter(|| {
            progress.update(black_box(500), None);
        });

        progress.complete(None);
    });

    c.bench_function("progress_message_update_silent", |b| {
        let mut progress = ProgressFactory::create(true);
        let config = ProgressConfig::new(ProgressStyle::Count);

        progress.start(config);

        b.iter(|| {
            progress.set_message(black_box("Test message".to_string()));
        });

        progress.complete(None);
    });

    c.bench_function("progress_update_with_total_silent", |b| {
        let mut progress = ProgressFactory::create(true);
        let config = ProgressConfig::new(ProgressStyle::Count);

        progress.start(config);

        b.iter(|| {
            progress.update(black_box(500), black_box(Some(1000)));
        });

        progress.complete(None);
    });
}

fn benchmark_progress_styles(c: &mut Criterion) {
    c.bench_function("progress_bytes_style", |b| {
        let mut progress = ProgressFactory::create(true);

        b.iter_batched(
            || ProgressConfig::new(ProgressStyle::Bytes).with_total(100_000_000),
            |config| {
                progress.start(config);
                for i in 0..10 {
                    progress.update(i * 10_000_000, None);
                }
                progress.complete(None);
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("progress_count_style", |b| {
        let mut progress = ProgressFactory::create(true);

        b.iter_batched(
            || ProgressConfig::new(ProgressStyle::Count).with_total(1000),
            |config| {
                progress.start(config);
                for i in 0..10 {
                    progress.update(i * 100, None);
                }
                progress.complete(None);
            },
            BatchSize::SmallInput,
        );
    });
}

fn benchmark_status_reporter(c: &mut Criterion) {
    c.bench_function("status_reporter_silent", |b| {
        let reporter = StatusReporter::new(true);

        b.iter(|| {
            reporter.operation(black_box("Operation"), black_box("context"));
            reporter.step(black_box("Step"));
            reporter.success(black_box("Success"));
        });
    });

    c.bench_function("status_reporter_normal", |b| {
        let reporter = StatusReporter::new(false);

        b.iter(|| {
            reporter.operation(black_box("Operation"), black_box("context"));
            reporter.step(black_box("Step"));
            reporter.success(black_box("Success"));
        });
    });

    c.bench_function("status_reporter_error", |b| {
        let reporter = StatusReporter::new(true);

        b.iter(|| {
            reporter.error(black_box("Error message"));
        });
    });
}

fn benchmark_progress_large_operations(c: &mut Criterion) {
    c.bench_function("progress_1m_updates", |b| {
        let mut progress = ProgressFactory::create(true);
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(1_000_000);

        b.iter(|| {
            progress.start(config.clone());

            // Update every 10,000 items (100 updates total)
            for i in 0..100 {
                progress.update(i * 10_000, None);
            }

            progress.complete(None);
        });
    });

    c.bench_function("progress_concurrent_updates", |b| {
        use std::thread;

        b.iter(|| {
            let mut handles = vec![];

            for _ in 0..4 {
                let handle = thread::spawn(move || {
                    let mut progress = ProgressFactory::create(true);
                    let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);

                    progress.start(config);
                    for i in 0..100 {
                        progress.update(i, None);
                    }
                    progress.complete(None);
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

fn benchmark_progress_memory_allocation(c: &mut Criterion) {
    c.bench_function("progress_config_creation", |b| {
        b.iter(|| black_box(ProgressConfig::new(ProgressStyle::Count).with_total(1000)));
    });

    c.bench_function("progress_factory_allocation", |b| {
        b.iter(|| black_box(ProgressFactory));
    });

    c.bench_function("status_reporter_allocation", |b| {
        b.iter(|| black_box(StatusReporter::new(false)));
    });
}

fn benchmark_real_world_scenarios(c: &mut Criterion) {
    c.bench_function("scenario_file_download", |b| {
        let mut progress = ProgressFactory::create(true);

        b.iter(|| {
            let config = ProgressConfig::new(ProgressStyle::Bytes).with_total(195_000_000); // ~195MB

            progress.start(config);

            // Simulate download in 1MB chunks
            let chunk_size = 1_000_000;
            let chunks = 195;

            for i in 0..chunks {
                progress.update(i * chunk_size, None);
            }

            progress.complete(None);
        });
    });

    c.bench_function("scenario_batch_uninstall", |b| {
        let mut progress = ProgressFactory::create(true);

        b.iter(|| {
            let config = ProgressConfig::new(ProgressStyle::Count).with_total(10);

            progress.start(config);

            for i in 0..10 {
                progress.set_message(format!("Removing JDK {}/10", i + 1));
                progress.update(i as u64 + 1, None);
            }

            progress.complete(None);
        });
    });

    c.bench_function("scenario_cache_refresh", |b| {
        let mut progress = ProgressFactory::create(true);

        b.iter(|| {
            let config = ProgressConfig::new(ProgressStyle::Count);

            progress.start(config);

            // Simulate spinner with message updates
            for _ in 0..20 {
                progress.set_message("Fetching...".to_string());
            }

            progress.set_message("Processing distributions...".to_string());

            for _ in 0..10 {
                progress.set_message("Finalizing...".to_string());
            }

            progress.complete(None);
        });
    });
}

criterion_group!(
    benches,
    benchmark_progress_factory_creation,
    benchmark_progress_operations,
    benchmark_progress_styles,
    benchmark_status_reporter,
    benchmark_progress_large_operations,
    benchmark_progress_memory_allocation,
    benchmark_real_world_scenarios
);

criterion_main!(benches);
