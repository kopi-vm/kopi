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

//! Benchmarks for the env command performance
//!
//! Run with: cargo bench --bench env_command

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use kopi::commands::env::EnvCommand;
use kopi::config::new_kopi_config;
use std::env;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

/// Create a test configuration with a temporary home directory and set KOPI_HOME
fn setup_test_env() -> (TempDir, impl Drop) {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Set KOPI_HOME environment variable
    unsafe {
        env::set_var("KOPI_HOME", kopi_home);
    }

    // Create necessary directories
    fs::create_dir_all(kopi_home.join("jdks")).unwrap();
    fs::create_dir_all(kopi_home.join("cache")).unwrap();

    // Return a guard that will restore the environment when dropped
    struct EnvGuard {
        original: Option<String>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(val) = &self.original {
                    env::set_var("KOPI_HOME", val);
                } else {
                    env::remove_var("KOPI_HOME");
                }
            }
        }
    }

    let guard = EnvGuard {
        original: env::var("KOPI_HOME").ok(),
    };

    (temp_dir, guard)
}

/// Install a real JDK using kopi install command
fn install_jdk(kopi_home: &std::path::Path, version: &str, distribution: &str) {
    use std::process::Command;

    // Build the kopi path - use the release binary from this project
    let kopi_binary = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("release")
        .join("kopi");

    // Run kopi install command with KOPI_HOME set
    let output = Command::new(&kopi_binary)
        .env("KOPI_HOME", kopi_home)
        .arg("install")
        .arg(format!("{distribution}@{version}"))
        .output()
        .expect("Failed to execute kopi install");

    if !output.status.success() {
        panic!(
            "Failed to install JDK {}@{}: {}\n{}",
            distribution,
            version,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Benchmark env command with global configuration
fn benchmark_env_global(c: &mut Criterion) {
    c.bench_function("env_global_config", |b| {
        // Setup once before iterations
        let (_temp_dir, _guard) = setup_test_env();
        let kopi_home = env::var("KOPI_HOME").unwrap();
        let kopi_home_path = std::path::Path::new(&kopi_home);

        // Install a real JDK
        install_jdk(kopi_home_path, "21", "temurin");

        // Set global version in ~/.kopi/version
        let version_file = kopi_home_path.join("version");
        fs::write(&version_file, "temurin@21").unwrap();

        b.iter(|| {
            // Create config and run command
            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            // Use export=true to include export statements during benchmarking
            let _ = black_box(cmd.execute(None, None, true));
        });
    });
}

/// Benchmark env command with project-specific version file
fn benchmark_env_project(c: &mut Criterion) {
    c.bench_function("env_project_version", |b| {
        // Setup once before iterations
        let (_temp_dir, _guard) = setup_test_env();
        let kopi_home = env::var("KOPI_HOME").unwrap();
        let kopi_home_path = std::path::Path::new(&kopi_home);

        // Install a real JDK
        install_jdk(kopi_home_path, "17", "corretto");

        // Create project directory with .kopi-version
        let project_dir = kopi_home_path.join("project");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(".kopi-version"), "corretto@17").unwrap();

        // Change to project directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_dir).unwrap();

        b.iter(|| {
            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, None, true));
        });

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    });
}

/// Benchmark env command with explicit version
fn benchmark_env_explicit(c: &mut Criterion) {
    c.bench_function("env_explicit_version", |b| {
        // Setup once before iterations
        let (_temp_dir, _guard) = setup_test_env();
        let kopi_home = env::var("KOPI_HOME").unwrap();
        let kopi_home_path = std::path::Path::new(&kopi_home);

        // Install multiple real JDKs
        install_jdk(kopi_home_path, "11", "temurin");
        install_jdk(kopi_home_path, "17", "temurin");
        install_jdk(kopi_home_path, "21", "temurin");

        b.iter(|| {
            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(Some("temurin@17"), None, true));
        });
    });
}

/// Benchmark env command with deep directory hierarchy
fn benchmark_env_deep_hierarchy(c: &mut Criterion) {
    c.bench_function("env_deep_hierarchy", |b| {
        // Setup once before iterations
        let (_temp_dir, _guard) = setup_test_env();
        let kopi_home = env::var("KOPI_HOME").unwrap();
        let kopi_home_path = std::path::Path::new(&kopi_home);

        // Install a real JDK
        install_jdk(kopi_home_path, "21", "zulu");

        // Create deep directory structure
        let mut current = kopi_home_path.join("project");
        for i in 0..10 {
            current = current.join(format!("level{i}"));
            fs::create_dir_all(&current).unwrap();
        }

        // Put .kopi-version at the project root
        fs::write(
            kopi_home_path.join("project").join(".kopi-version"),
            "zulu@21",
        )
        .unwrap();

        // Change to deepest directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&current).unwrap();

        b.iter(|| {
            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, None, true));
        });

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    });
}

/// Benchmark different shell formatters
fn benchmark_env_shells(c: &mut Criterion) {
    let shells = vec!["bash", "zsh", "fish", "powershell", "cmd"];

    for shell in shells {
        c.bench_function(&format!("env_shell_{shell}"), |b| {
            // Setup once before iterations
            let (_temp_dir, _guard) = setup_test_env();
            let kopi_home = env::var("KOPI_HOME").unwrap();
            let kopi_home_path = std::path::Path::new(&kopi_home);

            // Install a real JDK
            install_jdk(kopi_home_path, "21", "temurin");

            // Set global version
            let version_file = kopi_home_path.join("version");
            fs::write(&version_file, "temurin@21").unwrap();

            b.iter(|| {
                let config = new_kopi_config().unwrap();
                let cmd = EnvCommand::new(&config).unwrap();
                let _ = black_box(cmd.execute(None, Some(shell), true));
            });
        });
    }
}

/// Benchmark env command error cases
fn benchmark_env_errors(c: &mut Criterion) {
    c.bench_function("env_no_jdk_error", |b| {
        // Setup once before iterations
        let (_temp_dir, _guard) = setup_test_env();

        // No JDK installed - this will error
        b.iter(|| {
            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(Some("temurin@99"), None, true));
        });
    });
}

/// Benchmark cold start performance
fn benchmark_env_cold_start(c: &mut Criterion) {
    c.bench_function("env_cold_start", |b| {
        b.iter(|| {
            // Create new config each time to simulate cold start
            let (_temp_dir, _guard) = setup_test_env();
            let kopi_home = env::var("KOPI_HOME").unwrap();
            let kopi_home_path = std::path::Path::new(&kopi_home);

            install_jdk(kopi_home_path, "21", "temurin");
            let version_file = kopi_home_path.join("version");
            fs::write(&version_file, "temurin@21").unwrap();

            let config = new_kopi_config().unwrap();
            let cmd = EnvCommand::new(&config).unwrap();
            let _ = black_box(cmd.execute(None, None, true));
        });
    });
}

criterion_group! {
    name = env_benchmarks;
    config = Criterion::default()
        .sample_size(10)  // Minimum sample size
        .measurement_time(Duration::from_secs(2))  // Shorter measurement time
        .warm_up_time(Duration::from_millis(500));  // Shorter warm-up time
    targets = benchmark_env_global,
        benchmark_env_project,
        benchmark_env_explicit,
        benchmark_env_deep_hierarchy,
        benchmark_env_shells,
        benchmark_env_errors,
        benchmark_env_cold_start
}

criterion_main!(env_benchmarks);

/// Additional module for microbenchmarks
#[cfg(test)]
mod microbenchmarks {
    // Move imports here to avoid unused import warnings
    #[allow(unused_imports)]
    use kopi::config::new_kopi_config;
    #[allow(unused_imports)]
    use kopi::platform::shell::{detect_shell, parse_shell_name};
    #[allow(unused_imports)]
    use kopi::version::resolver::VersionResolver;
    #[allow(unused_imports)]
    use std::env;
    #[allow(unused_imports)]
    use std::fs;
    #[allow(unused_imports)]
    use std::time::Instant;
    #[allow(unused_imports)]
    use tempfile::TempDir;

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
        let original_home = env::var("KOPI_HOME").ok();

        // Set KOPI_HOME to temp directory
        unsafe {
            env::set_var("KOPI_HOME", temp.path());
        }

        // Create nested directories
        let mut path = temp.path().to_path_buf();
        for i in 0..20 {
            path = path.join(format!("dir{}", i));
            fs::create_dir(&path).unwrap();
        }

        // Put version file at root
        fs::write(temp.path().join(".kopi-version"), "temurin@21").unwrap();

        // Measure from deepest directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&path).unwrap();

        let iterations = 100;
        let start = Instant::now();

        for _ in 0..iterations {
            let config = new_kopi_config().unwrap();
            let resolver = VersionResolver::new(&config);
            let _ = resolver.resolve_version();
        }

        let elapsed = start.elapsed();
        let per_call = elapsed / iterations;

        println!(
            "Version resolution (20 levels deep): {:?} per call",
            per_call
        );

        // Restore original state
        env::set_current_dir(original_dir).unwrap();
        unsafe {
            if let Some(home) = original_home {
                env::set_var("KOPI_HOME", home);
            } else {
                env::remove_var("KOPI_HOME");
            }
        }
    }
}
