//! Benchmarks for the env command performance
//!
//! Run with: cargo bench --bench env_command

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use kopi::commands::env::EnvCommand;
use kopi::config::KopiConfig;
use std::fs;
use tempfile::TempDir;

/// Create a test configuration with a temporary home directory
fn setup_test_config() -> (KopiConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create config - this will create necessary directories
    let config = KopiConfig::new(kopi_home.to_path_buf()).unwrap();
    
    // Force directory creation
    let _ = config.jdks_dir();
    let _ = config.cache_dir();

    (config, temp_dir)
}

/// Create a mock JDK installation for testing
fn create_mock_jdk(config: &KopiConfig, version: &str, distribution: &str) {
    let jdks_dir = config.jdks_dir().unwrap();
    let jdk_dir = jdks_dir.join(format!("{}-{}", distribution, version));
    fs::create_dir_all(&jdk_dir).unwrap();

    // Create bin directory
    let bin_dir = jdk_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create java executable (empty file for testing)
    #[cfg(unix)]
    {
        fs::write(bin_dir.join("java"), "").unwrap();
    }
    #[cfg(windows)]
    {
        fs::write(bin_dir.join("java.exe"), "").unwrap();
    }
    
    // Create a simple marker file for testing
    fs::write(jdk_dir.join(".installed"), "").unwrap();
}

/// Benchmark env command with global configuration
fn benchmark_env_global(c: &mut Criterion) {
    let (config, _temp) = setup_test_config();

    // Create a JDK installation
    create_mock_jdk(&config, "21", "temurin");

    // Set global version
    fs::write(
        config.config_path(),
        "[global]\ndefault_version = \"temurin@21\"",
    )
    .unwrap();

    c.bench_function("env_global_config", |b| {
        b.iter(|| {
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, Some("bash"), true, true));
        });
    });
}

/// Benchmark env command with project-specific version file
fn benchmark_env_project(c: &mut Criterion) {
    let (config, temp) = setup_test_config();

    // Create a JDK installation
    create_mock_jdk(&config, "17", "corretto");

    // Create project directory with .kopi-version
    let project_dir = temp.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join(".kopi-version"), "corretto@17").unwrap();

    // Change to project directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project_dir).unwrap();

    c.bench_function("env_project_version", |b| {
        b.iter(|| {
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, Some("bash"), true, true));
        });
    });

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

/// Benchmark env command with explicit version
fn benchmark_env_explicit(c: &mut Criterion) {
    let (config, _temp) = setup_test_config();

    // Create multiple JDK installations
    create_mock_jdk(&config, "11", "temurin");
    create_mock_jdk(&config, "17", "temurin");
    create_mock_jdk(&config, "21", "temurin");

    c.bench_function("env_explicit_version", |b| {
        b.iter(|| {
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(Some("temurin@17"), Some("bash"), true, true));
        });
    });
}

/// Benchmark env command with deep directory hierarchy
fn benchmark_env_deep_hierarchy(c: &mut Criterion) {
    let (config, temp) = setup_test_config();

    // Create a JDK installation
    create_mock_jdk(&config, "21", "zulu");

    // Create deep directory structure
    let mut current = temp.path().to_path_buf();
    for i in 0..10 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
    }

    // Put .kopi-version at the root
    fs::write(temp.path().join(".kopi-version"), "zulu@21").unwrap();

    // Change to deepest directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&current).unwrap();

    c.bench_function("env_deep_hierarchy", |b| {
        b.iter(|| {
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, Some("bash"), true, true));
        });
    });

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

/// Benchmark different shell formatters
fn benchmark_env_shells(c: &mut Criterion) {
    let (config, _temp) = setup_test_config();

    // Create a JDK installation
    create_mock_jdk(&config, "21", "temurin");

    // Set global version
    fs::write(
        config.config_path(),
        "[global]\ndefault_version = \"temurin@21\"",
    )
    .unwrap();

    let shells = vec!["bash", "zsh", "fish", "powershell", "cmd"];

    for shell in shells {
        c.bench_function(&format!("env_shell_{}", shell), |b| {
            b.iter(|| {
                let cmd = EnvCommand::new(&config).unwrap();
                let _ = black_box(cmd.execute(None, Some(shell), true, true));
            });
        });
    }
}

/// Benchmark env command error cases
fn benchmark_env_errors(c: &mut Criterion) {
    let (config, _temp) = setup_test_config();

    // No JDK installed - this will error
    c.bench_function("env_no_jdk_error", |b| {
        b.iter(|| {
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(Some("temurin@99"), Some("bash"), true, true));
        });
    });
}

/// Benchmark cold start performance
fn benchmark_env_cold_start(c: &mut Criterion) {
    c.bench_function("env_cold_start", |b| {
        b.iter(|| {
            // Create new config each time to simulate cold start
            let (config, _temp) = setup_test_config();
            create_mock_jdk(&config, "21", "temurin");
            fs::write(
                config.config_path(),
                "[global]\ndefault_version = \"temurin@21\"",
            )
            .unwrap();

            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, Some("bash"), true, true));
        });
    });
}

criterion_group!(
    env_benchmarks,
    benchmark_env_global,
    benchmark_env_project,
    benchmark_env_explicit,
    benchmark_env_deep_hierarchy,
    benchmark_env_shells,
    benchmark_env_errors,
    benchmark_env_cold_start
);

criterion_main!(env_benchmarks);

/// Additional module for microbenchmarks
#[cfg(test)]
mod microbenchmarks {
    #[allow(unused_imports)]
    use super::*;
    use kopi::platform::shell::{detect_shell, parse_shell_name};
    use kopi::version::resolver::VersionResolver;
    use std::time::Instant;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    #[ignore]
    fn measure_shell_detection() {
        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = detect_shell();
        }

        let elapsed = start.elapsed();
        let per_call = elapsed / iterations;

        println!("Shell detection: {:?} per call", per_call);
        println!("Total for {} iterations: {:?}", iterations, elapsed);
    }

    #[test]
    #[ignore]
    fn measure_shell_parsing() {
        let shells = vec!["bash", "zsh", "fish", "powershell", "cmd", "unknown"];
        let iterations = 10000;

        for shell in shells {
            let start = Instant::now();

            for _ in 0..iterations {
                let _ = parse_shell_name(shell);
            }

            let elapsed = start.elapsed();
            let per_call = elapsed / iterations;

            println!("Parse '{}': {:?} per call", shell, per_call);
        }
    }

    #[test]
    #[ignore]
    fn measure_version_file_lookup() {
        let temp = TempDir::new().unwrap();

        // Create nested directories
        let mut path = temp.path().to_path_buf();
        for i in 0..20 {
            path = path.join(format!("dir{}", i));
            fs::create_dir(&path).unwrap();
        }

        // Put version file at root
        fs::write(temp.path().join(".kopi-version"), "temurin@21").unwrap();

        // Measure from deepest directory
        std::env::set_current_dir(&path).unwrap();

        let iterations = 100;
        let start = Instant::now();

        for _ in 0..iterations {
            let resolver = VersionResolver::new();
            let _ = resolver.resolve_version();
        }

        let elapsed = start.elapsed();
        let per_call = elapsed / iterations;

        println!(
            "Version resolution (20 levels deep): {:?} per call",
            per_call
        );
    }
}
