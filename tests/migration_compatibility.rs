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
use assert_cmd::Command;
use common::TestHomeGuard;
use kopi::paths::install;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::Path;

/// Helper to create a mock JDK installation without metadata
fn create_legacy_jdk_installation(kopi_home: &Path, distribution: &str, version: &str) {
    let jdk_dir = kopi_home
        .join("jdks")
        .join(format!("{distribution}-{version}"));
    fs::create_dir_all(&jdk_dir).unwrap();

    // Create standard JDK structure
    #[cfg(target_os = "macos")]
    {
        // Create bundle structure for macOS
        if distribution == "temurin" {
            let bundle_home = install::bundle_java_home(&jdk_dir);
            let bundle_bin = install::bin_directory(&bundle_home);
            fs::create_dir_all(&bundle_bin).unwrap();
            fs::write(bundle_bin.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(bundle_bin.join("java")).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(bundle_bin.join("java"), perms).unwrap();
            }
        } else {
            // Direct structure
            let bin_dir = jdk_dir.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(bin_dir.join("java"), perms).unwrap();
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(bin_dir.join("java"), perms).unwrap();
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Direct structure for non-macOS
        let bin_dir = jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    // Create lib directory
    let lib_dir = if cfg!(target_os = "macos") && distribution == "temurin" {
        install::bundle_java_home(&jdk_dir).join("lib")
    } else {
        jdk_dir.join("lib")
    };
    fs::create_dir_all(&lib_dir).unwrap();

    // Create release file
    let release_path = if cfg!(target_os = "macos") && distribution == "temurin" {
        install::bundle_java_home(&jdk_dir).join("release")
    } else {
        jdk_dir.join("release")
    };
    fs::write(
        &release_path,
        format!("JAVA_VERSION=\"{version}\"\nIMPLEMENTOR=\"Mock\""),
    )
    .unwrap();
}

/// Test that existing JDK installations without metadata continue to work
#[test]
#[serial]
fn test_existing_installations_work_without_metadata() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create several legacy JDK installations without metadata
    create_legacy_jdk_installation(&kopi_home, "temurin", "21.0.1");
    create_legacy_jdk_installation(&kopi_home, "liberica", "17.0.9");
    create_legacy_jdk_installation(&kopi_home, "zulu", "11.0.21");

    // List installed JDKs using CLI
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin@21.0.1"))
        .stdout(predicate::str::contains("liberica@17.0.9"))
        .stdout(predicate::str::contains("zulu@11.0.21"));

    // Test that we can use each JDK
    for (dist, ver) in [
        ("temurin", "21.0.1"),
        ("liberica", "17.0.9"),
        ("zulu", "11.0.21"),
    ] {
        // Create a .kopi-version file
        fs::write(
            test_home.path().join(".kopi-version"),
            format!("{dist}@{ver}"),
        )
        .unwrap();

        // Test 'kopi env' command - this tests path resolution without checking for java binary
        let mut cmd = Command::cargo_bin("kopi").unwrap();
        cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
            .env("HOME", test_home.path())
            .current_dir(test_home.path())
            .arg("env")
            .assert()
            .success()
            .stdout(predicate::str::contains("JAVA_HOME"))
            .stdout(predicate::str::contains(format!("{dist}-{ver}")));
    }
}

/// Test mixed environment with both old (no metadata) and new (with metadata) installations
#[test]
#[serial]
fn test_mixed_environment_old_and_new_installations() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create legacy installation without metadata
    create_legacy_jdk_installation(&kopi_home, "liberica", "17.0.9");

    // Create new installation with metadata
    let new_jdk_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&new_jdk_dir).unwrap();

    #[cfg(target_os = "macos")]
    {
        let bundle_bin = install::bin_directory(&install::bundle_java_home(&new_jdk_dir));
        fs::create_dir_all(&bundle_bin).unwrap();
        fs::write(bundle_bin.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let bin_dir = new_jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    // Create metadata file for new installation
    let metadata_content = r#"{
        "id": "test-id",
        "archive_type": "tar.gz",
        "distribution": "temurin",
        "major_version": 21,
        "java_version": "21.0.1",
        "distribution_version": "21.0.1+35.1",
        "jdk_version": 21,
        "directly_downloadable": true,
        "filename": "test.tar.gz",
        "links": {
            "pkg_download_redirect": "https://example.com",
            "pkg_info_uri": null
        },
        "free_use_in_production": true,
        "tck_tested": "yes",
        "tck_cert_uri": "https://example.com/cert",
        "aqavit_certified": "yes",
        "aqavit_cert_uri": "https://example.com/aqavit",
        "size": 100000000,
        "feature": [],
        "installation_metadata": {
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos",
            "metadata_version": 1
        }
    }"#;

    let metadata_path = kopi_home.join("jdks").join("temurin-21.0.1.meta.json");
    fs::write(&metadata_path, metadata_content).unwrap();

    // List installed JDKs - should show both
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("liberica@17.0.9"))
        .stdout(predicate::str::contains("temurin@21.0.1"));

    // Test both JDKs work correctly
    // Test liberica (no metadata)
    fs::write(test_home.path().join(".kopi-version"), "liberica@17.0.9").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("liberica-17.0.9"));

    // Test temurin (with metadata)
    fs::write(test_home.path().join(".kopi-version"), "temurin@21.0.1").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-21.0.1"));

    // Both should work despite one having metadata and one not
}

/// Test upgrading from old version (no metadata) to new version (with metadata)
#[test]
#[serial]
fn test_upgrade_scenario_from_old_to_new() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Step 1: Create legacy installation
    create_legacy_jdk_installation(&kopi_home, "temurin", "17.0.1");

    // Verify it works without metadata
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin@17.0.1"));

    // Test it works
    fs::write(test_home.path().join(".kopi-version"), "temurin@17.0.1").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-17.0.1"));

    // Step 2: Simulate upgrade by installing a new version with metadata
    let new_jdk_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&new_jdk_dir).unwrap();

    #[cfg(target_os = "macos")]
    {
        let bundle_bin = install::bin_directory(&install::bundle_java_home(&new_jdk_dir));
        fs::create_dir_all(&bundle_bin).unwrap();
        fs::write(bundle_bin.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let bin_dir = new_jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    // Create metadata for new installation
    let metadata_content = r#"{
        "id": "new-test-id",
        "archive_type": "tar.gz",
        "distribution": "temurin",
        "major_version": 21,
        "java_version": "21.0.1",
        "distribution_version": "21.0.1+35.1",
        "jdk_version": 21,
        "directly_downloadable": true,
        "filename": "test.tar.gz",
        "links": {
            "pkg_download_redirect": "https://example.com",
            "pkg_info_uri": null
        },
        "free_use_in_production": true,
        "tck_tested": "yes",
        "tck_cert_uri": "https://example.com/cert",
        "aqavit_certified": "yes",
        "aqavit_cert_uri": "https://example.com/aqavit",
        "size": 100000000,
        "feature": [],
        "installation_metadata": {
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos",
            "metadata_version": 1
        }
    }"#;

    let metadata_path = kopi_home.join("jdks").join("temurin-21.0.1.meta.json");
    fs::write(&metadata_path, metadata_content).unwrap();

    // Step 3: Verify both old and new installations work
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin@17.0.1"))
        .stdout(predicate::str::contains("temurin@21.0.1"));

    // Both versions should work
    fs::write(test_home.path().join(".kopi-version"), "temurin@17.0.1").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-17.0.1"));

    fs::write(test_home.path().join(".kopi-version"), "temurin@21.0.1").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-21.0.1"));
}

/// Test rollback scenario - newer version fails, fallback to older version
#[test]
#[serial]
fn test_rollback_scenario() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create an old installation without metadata
    create_legacy_jdk_installation(&kopi_home, "corretto", "17.0.9");

    // Create a new installation with corrupted metadata
    let new_jdk_dir = kopi_home.join("jdks").join("corretto-21.0.1");
    fs::create_dir_all(&new_jdk_dir).unwrap();

    #[cfg(target_os = "macos")]
    {
        // Intentionally create wrong structure to test fallback
        let bin_dir = new_jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let bin_dir = new_jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    // Create corrupted metadata that doesn't match actual structure
    let bad_metadata_content = r#"{
        "id": "test-id",
        "installation_metadata": {
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos",
            "metadata_version": 1
        }
    }"#;

    let metadata_path = kopi_home.join("jdks").join("corretto-21.0.1.meta.json");
    fs::write(&metadata_path, bad_metadata_content).unwrap();

    // List installed JDKs - should show both
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("corretto@17.0.9"))
        .stdout(predicate::str::contains("corretto@21.0.1"));

    // Test rollback - both should work even with bad metadata
    // Test old JDK (no metadata)
    fs::write(test_home.path().join(".kopi-version"), "corretto@17.0.9").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("corretto-17.0.9"));

    // Test new JDK (corrupted metadata) - should still work via fallback
    fs::write(test_home.path().join(".kopi-version"), "corretto@21.0.1").unwrap();
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("corretto-21.0.1"));
}

/// Test that shim works with legacy installations
#[test]
#[serial]
fn test_shim_with_legacy_installation() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create legacy installation
    create_legacy_jdk_installation(&kopi_home, "temurin", "21.0.1");

    // Create a .kopi-version file to select this JDK
    let project_dir = test_home.path().join("test-project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join(".kopi-version"), "temurin@21.0.1").unwrap();

    // Test that the shim works by using 'kopi env' in the project directory
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(&project_dir)
        .arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("JAVA_HOME"))
        .stdout(predicate::str::contains("temurin-21.0.1"));

    // Test that 'kopi current' detects the version correctly
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(&project_dir)
        .arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains("temurin@21.0.1"));
}

/// Test performance of legacy installations vs new installations with metadata
#[test]
#[serial]
fn test_performance_comparison() {
    use std::time::Instant;

    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create legacy installation
    create_legacy_jdk_installation(&kopi_home, "liberica", "17.0.9");

    // Create new installation with metadata
    let new_jdk_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&new_jdk_dir).unwrap();

    #[cfg(target_os = "macos")]
    {
        let bundle_bin = install::bin_directory(&install::bundle_java_home(&new_jdk_dir));
        fs::create_dir_all(&bundle_bin).unwrap();
        fs::write(bundle_bin.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let bin_dir = new_jdk_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "#!/bin/sh\necho 'mock java'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(bin_dir.join("java")).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(bin_dir.join("java"), perms).unwrap();
        }
    }

    // Create metadata
    let metadata_content = r#"{
        "id": "test-id",
        "archive_type": "tar.gz",
        "distribution": "temurin",
        "major_version": 21,
        "java_version": "21.0.1",
        "distribution_version": "21.0.1+35.1",
        "jdk_version": 21,
        "directly_downloadable": true,
        "filename": "test.tar.gz",
        "links": {
            "pkg_download_redirect": "https://example.com",
            "pkg_info_uri": null
        },
        "free_use_in_production": true,
        "tck_tested": "yes",
        "tck_cert_uri": "https://example.com/cert",
        "aqavit_certified": "yes",
        "aqavit_cert_uri": "https://example.com/aqavit",
        "size": 100000000,
        "feature": [],
        "installation_metadata": {
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos",
            "metadata_version": 1
        }
    }"#;

    let metadata_path = kopi_home.join("jdks").join("temurin-21.0.1.meta.json");
    fs::write(&metadata_path, metadata_content).unwrap();

    // Measure performance using 'kopi which' command
    // Legacy JDK (runtime detection)
    fs::write(test_home.path().join(".kopi-version"), "liberica@17.0.9").unwrap();
    let start = Instant::now();
    for _ in 0..10 {
        let mut cmd = Command::cargo_bin("kopi").unwrap();
        cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
            .env("HOME", test_home.path())
            .current_dir(test_home.path())
            .arg("env")
            .assert()
            .success();
    }
    let legacy_time = start.elapsed();

    // New JDK (with metadata)
    fs::write(test_home.path().join(".kopi-version"), "temurin@21.0.1").unwrap();
    let start = Instant::now();
    for _ in 0..10 {
        let mut cmd = Command::cargo_bin("kopi").unwrap();
        cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
            .env("HOME", test_home.path())
            .current_dir(test_home.path())
            .arg("env")
            .assert()
            .success();
    }
    let new_time = start.elapsed();

    println!("Legacy (runtime detection): {legacy_time:?} for 10 calls");
    println!("New (with metadata cache): {new_time:?} for 10 calls");

    // Note: Performance comparison at the integration test level may vary
    // The real performance gain is at the internal path resolution level
}

/// Test that env command works with legacy installations
#[test]
#[serial]
fn test_env_command_with_legacy_installation() {
    use assert_cmd::Command;

    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create legacy installation
    create_legacy_jdk_installation(&kopi_home, "zulu", "11.0.21");

    // Create .kopi-version file
    fs::write(test_home.path().join(".kopi-version"), "zulu@11.0.21").unwrap();

    // Run env command
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .current_dir(test_home.path())
        .arg("env")
        .assert()
        .success();

    // The env command should work with legacy installations
}

/// Test uninstall command works with legacy installations
#[test]
#[serial]
fn test_uninstall_legacy_installation() {
    use assert_cmd::Command;

    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create legacy installation
    create_legacy_jdk_installation(&kopi_home, "liberica", "17.0.9");

    // Verify it exists
    let jdk_dir = kopi_home.join("jdks").join("liberica-17.0.9");
    assert!(jdk_dir.exists(), "Legacy JDK should exist");

    // Run uninstall command
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap())
        .env("HOME", test_home.path())
        .arg("uninstall")
        .arg("liberica@17.0.9")
        .arg("--force") // Skip confirmation
        .assert()
        .success();

    // Verify it's gone
    assert!(!jdk_dir.exists(), "Legacy JDK should be uninstalled");
}
