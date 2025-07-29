//! Benchmarks for the which command performance
//!
//! Run with: cargo bench --bench which_bench

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use kopi::commands::which::WhichCommand;
use kopi::config::KopiConfig;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test JDK installation for benchmarking
fn setup_test_jdk(kopi_home: &PathBuf, distribution: &str, version: &str) {
    let jdk_path = kopi_home
        .join("jdks")
        .join(format!("{distribution}-{version}"));

    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create mock executables
    for tool in &["java", "javac", "jar", "jshell"] {
        let tool_path = if cfg!(windows) {
            bin_dir.join(format!("{tool}.exe"))
        } else {
            bin_dir.join(tool)
        };

        fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&tool_path).unwrap();
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tool_path, perms).unwrap();
        }
    }

    // Create metadata file
    let metadata = serde_json::json!({
        "distribution": distribution,
        "version": version,
    });
    let metadata_path = jdk_path.join("kopi-metadata.json");
    fs::write(&metadata_path, serde_json::to_string(&metadata).unwrap()).unwrap();
}

/// Setup test environment with multiple JDKs
fn setup_test_environment() -> (TempDir, KopiConfig) {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path().to_path_buf();

    // Create directory structure
    fs::create_dir_all(kopi_home.join("jdks")).unwrap();
    fs::create_dir_all(kopi_home.join("cache")).unwrap();
    fs::create_dir_all(kopi_home.join("shims")).unwrap();

    // Create test JDKs
    setup_test_jdk(&kopi_home, "temurin", "21.0.5+11");
    setup_test_jdk(&kopi_home, "temurin", "17.0.13+11");
    setup_test_jdk(&kopi_home, "corretto", "21.0.5.11.1");
    setup_test_jdk(&kopi_home, "zulu", "11.76.21");

    // Set global default
    fs::write(kopi_home.join("version"), "temurin@21.0.5+11").unwrap();

    let config = KopiConfig::new(kopi_home).unwrap();
    (temp_dir, config)
}

fn bench_which_current(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_current", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            let _ = command.execute(None, black_box("java"), false, false);
        });
    });
}

fn bench_which_specific(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_specific_version", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            let _ = command.execute(
                Some(black_box("temurin@21")),
                black_box("java"),
                false,
                false,
            );
        });
    });
}

fn bench_which_home_option(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_home_option", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            let _ = command.execute(
                Some(black_box("temurin@21")),
                black_box("java"),
                true,
                false,
            );
        });
    });
}

fn bench_which_different_tools(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_different_tools", |b| {
        let tools = ["java", "javac", "jar", "jshell"];
        let mut tool_index = 0;

        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            let tool = tools[tool_index % tools.len()];
            let _ = command.execute(Some(black_box("temurin@21")), black_box(tool), false, false);
            tool_index += 1;
        });
    });
}

fn bench_which_json_output(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_json_output", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            let _ = command.execute(
                Some(black_box("corretto@21")),
                black_box("java"),
                false,
                true,
            );
        });
    });
}

fn bench_which_ambiguous_version(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_ambiguous_version", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            // This should fail with multiple matches (temurin@21 and corretto@21)
            let _ = command.execute(Some(black_box("21")), black_box("java"), false, false);
        });
    });
}

fn bench_which_not_found(c: &mut Criterion) {
    let (_temp_dir, config) = setup_test_environment();

    c.bench_function("which_not_found", |b| {
        b.iter(|| {
            let command = WhichCommand::new(&config).unwrap();
            // This should fail as JDK is not installed
            let _ = command.execute(
                Some(black_box("liberica@22")),
                black_box("java"),
                false,
                false,
            );
        });
    });
}

criterion_group!(
    benches,
    bench_which_current,
    bench_which_specific,
    bench_which_home_option,
    bench_which_different_tools,
    bench_which_json_output,
    bench_which_ambiguous_version,
    bench_which_not_found
);
criterion_main!(benches);
