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

mod common;

use common::TestHomeGuard;
use kopi::config::KopiConfig;
use kopi::doctor::{CheckCategory, CheckStatus, DiagnosticEngine};
use serial_test::serial;
use std::env;
use std::fs;

#[test]
fn test_shell_checks_full_suite() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Ensure shims directory exists for some tests
    let shims_dir = config.kopi_home().join("shims");
    fs::create_dir_all(&shims_dir).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    // Should have exactly 4 shell checks
    assert_eq!(results.len(), 4);

    // Check that all results are from Shell category
    for result in &results {
        assert_eq!(result.category, CheckCategory::Shell);
    }

    // Verify check names
    let check_names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(check_names.contains(&"Shell Detection"));
    assert!(check_names.contains(&"PATH Configuration"));
    assert!(check_names.contains(&"Shell Configuration"));
    assert!(check_names.contains(&"Shim Functionality"));
}

#[test]
#[serial]
fn test_path_configuration_with_shims_in_path() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create shims directory
    let shims_dir = config.kopi_home().join("shims");
    fs::create_dir_all(&shims_dir).unwrap();

    // Add shims to PATH
    let original_path = env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ";" } else { ":" };
    let new_path = format!("{}{}{}", shims_dir.display(), separator, original_path);
    unsafe {
        env::set_var("PATH", &new_path);
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    // Find PATH configuration check
    let path_check = results
        .iter()
        .find(|r| r.name == "PATH Configuration")
        .expect("PATH Configuration check not found");

    assert_eq!(path_check.status, CheckStatus::Pass);
    assert!(path_check.message.contains("correctly configured"));

    // Restore PATH
    unsafe {
        env::set_var("PATH", original_path);
    }
}

#[test]
#[serial]
fn test_path_configuration_missing_shims() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Ensure PATH doesn't contain our shims directory
    let original_path = env::var("PATH").unwrap_or_default();
    let shims_path = config
        .kopi_home()
        .join("shims")
        .to_string_lossy()
        .to_string();
    let separator = if cfg!(windows) { ";" } else { ":" };
    let cleaned_path: Vec<&str> = original_path
        .split(separator)
        .filter(|p| !p.contains(&shims_path))
        .collect();
    unsafe {
        env::set_var("PATH", cleaned_path.join(separator));
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    let path_check = results
        .iter()
        .find(|r| r.name == "PATH Configuration")
        .expect("PATH Configuration check not found");

    assert_eq!(path_check.status, CheckStatus::Fail);
    assert!(path_check.message.contains("not found in PATH"));
    assert!(path_check.suggestion.is_some());

    // Restore PATH
    unsafe {
        env::set_var("PATH", original_path);
    }
}

#[test]
fn test_shell_detection() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    let shell_check = results
        .iter()
        .find(|r| r.name == "Shell Detection")
        .expect("Shell Detection check not found");

    // Shell detection should either pass or warn, never fail
    assert!(matches!(
        shell_check.status,
        CheckStatus::Pass | CheckStatus::Warning
    ));
}

#[test]
fn test_shim_functionality_no_directory() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Ensure shims directory doesn't exist
    let shims_dir = config.kopi_home().join("shims");
    if shims_dir.exists() {
        fs::remove_dir_all(&shims_dir).ok();
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    let shim_check = results
        .iter()
        .find(|r| r.name == "Shim Functionality")
        .expect("Shim Functionality check not found");

    assert_eq!(shim_check.status, CheckStatus::Fail);
    assert!(shim_check.message.contains("does not exist"));
}

#[test]
fn test_shim_functionality_with_shims() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Create shims directory and mock shim files
    let shims_dir = config.kopi_home().join("shims");
    fs::create_dir_all(&shims_dir).unwrap();

    // Create executable shim files
    for shim_name in &["java", "javac", "jar"] {
        let shim_path = shims_dir.join(shim_name);
        #[cfg(windows)]
        let shim_path = shim_path.with_extension("exe");

        fs::write(&shim_path, "#!/bin/sh\necho mock shim").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&shim_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&shim_path, perms).unwrap();
        }
    }

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    let shim_check = results
        .iter()
        .find(|r| r.name == "Shim Functionality")
        .expect("Shim Functionality check not found");

    assert_eq!(shim_check.status, CheckStatus::Pass);
    assert!(shim_check.message.contains("executable shims"));
}

#[test]
fn test_shell_configuration_check() {
    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    let engine = DiagnosticEngine::new(&config);
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);

    let config_check = results
        .iter()
        .find(|r| r.name == "Shell Configuration")
        .expect("Shell Configuration check not found");

    // This check can have various statuses depending on the environment
    // Just ensure it doesn't panic and has appropriate status
    assert!(matches!(
        config_check.status,
        CheckStatus::Pass | CheckStatus::Warning | CheckStatus::Skip
    ));
}

#[cfg_attr(not(feature = "perf_tests"), ignore)]
#[test]
fn test_performance_shell_checks() {
    use std::time::Instant;

    let guard = TestHomeGuard::new();
    guard.setup_kopi_structure();
    let config = KopiConfig::new(guard.kopi_home()).unwrap();

    // Ensure shims directory exists
    let shims_dir = config.kopi_home().join("shims");
    fs::create_dir_all(&shims_dir).unwrap();

    let engine = DiagnosticEngine::new(&config);

    let start = Instant::now();
    let results = engine.run_checks(Some(vec![CheckCategory::Shell]), false);
    let total_duration = start.elapsed();

    // All shell checks should complete quickly
    for result in &results {
        assert!(
            result.duration.as_millis() < 200,
            "{} took {}ms, expected < 200ms",
            result.name,
            result.duration.as_millis()
        );
    }

    // Total time should be reasonable
    assert!(
        total_duration.as_millis() < 500,
        "Shell checks took {}ms total, expected < 500ms",
        total_duration.as_millis()
    );
}
