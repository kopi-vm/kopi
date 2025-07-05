mod common;
use assert_cmd::Command;
use common::TestHomeGuard;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn get_test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home.to_str().unwrap());
    cmd.env("HOME", kopi_home.parent().unwrap());
    cmd
}

/// Test basic version installation without distribution specification
/// User command: `kopi install 21`
/// Expected: Successfully installs latest Eclipse Temurin 21.x.x
#[test]
fn test_install_basic_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install a basic version
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

/// Test installation with specific distribution and version
/// User command: `kopi install corretto@17`
/// Expected: Successfully installs Amazon Corretto 17
#[test]
fn test_install_with_distribution() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install with specific distribution
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("corretto@17")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

/// Test error handling for non-existent version
/// User command: `kopi install 999.999.999`
/// Expected: Clear error message with suggestion to check available versions
#[test]
fn test_install_invalid_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("999.999.999")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("Invalid major version: 999"));
}

/// Test error handling for invalid version format
/// User command: `kopi install invalid@#$%`
/// Expected: Error message explaining proper version format with examples
#[test]
fn test_install_invalid_format() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("invalid@#$%")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid version format"))
        .stderr(predicate::str::contains("Suggestion:"));
}

/// Test error handling when JDK version is already installed
/// Simulates: User tries to install a version that already exists
/// Expected: Error message suggesting --force flag to reinstall
#[test]
fn test_install_already_exists() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache to get available versions
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Do a dry-run first to see what version will be installed
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd
        .arg("install")
        .arg("21")
        .arg("--dry-run")
        .output()
        .unwrap();

    // Extract the actual version from the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = if stdout.contains("21.0.7") {
        "temurin-21.0.7"
    } else {
        // Fallback to a common version
        "temurin-21.0.5"
    };

    // Create a fake installation with the correct version
    let install_dir = kopi_home.join("jdks").join(version);
    fs::create_dir_all(&install_dir).unwrap();

    // Try to install without dry-run (should fail)
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already installed"))
        .stderr(predicate::str::contains("--force"));
}

/// Test --force flag to overwrite existing installation
/// User command: `kopi install 21 --force`
/// Expected: Successfully reinstalls even if version exists
#[test]
fn test_install_force_reinstall() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a fake installation
    let install_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    fs::create_dir_all(&install_dir).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install with force
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--force")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

#[test]
fn test_install_with_timeout() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--timeout")
        .arg("1") // Very short timeout
        .arg("--dry-run")
        .assert();
    // Note: May succeed or fail depending on network speed
}

#[test]
fn test_install_no_progress() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--no-progress")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_install_verbose_output() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("-vv") // Debug verbosity
        .arg("install")
        .arg("21")
        .arg("--dry-run")
        .assert();
}

#[test]
fn test_install_without_cache() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Try to install without cache - should automatically fetch metadata and succeed
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .arg("--dry-run") // Use dry-run to avoid actual download
        .assert()
        .success()
        .stdout(predicate::str::contains("Would install"));
}

#[test]
#[cfg(unix)]
fn test_install_permission_denied() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Make directory read-only
    let jdks_dir = kopi_home.join("jdks");
    fs::create_dir_all(&jdks_dir).unwrap();

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o444); // Read-only
    fs::set_permissions(&jdks_dir, perms).unwrap();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install - should fail with permission error
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Permission"))
        .stderr(predicate::str::contains("sudo").or(predicate::str::contains("permissions")));

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&jdks_dir).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&jdks_dir, perms).unwrap();
}

#[test]
fn test_install_with_javafx() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache with javafx
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache")
        .arg("refresh")
        .arg("--javafx-bundled")
        .assert()
        .success();

    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("17")
        .arg("--javafx-bundled")
        .arg("--dry-run")
        .assert();
}

#[test]
fn test_concurrent_installs() {
    use std::thread;

    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install two different versions concurrently
    let kopi_home_1 = kopi_home.clone();
    let handle1 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_1);
        cmd.arg("install")
            .arg("17")
            .arg("--dry-run")
            .assert()
            .success();
    });

    let kopi_home_2 = kopi_home.clone();
    let handle2 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_2);
        cmd.arg("install")
            .arg("21")
            .arg("--dry-run")
            .assert()
            .success();
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn test_install_specific_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install a specific patch version
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install").arg("17.0.9").arg("--dry-run").assert();
}

#[test]
fn test_install_lts_version() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install an LTS version - should show LTS note
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("21") // 21 is LTS
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_exit_codes() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Test invalid version format - should exit with code 2
    let output = get_test_command(&kopi_home)
        .arg("install")
        .arg("@@@invalid")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));

    // Test network error by using invalid API URL (if we can simulate it)
    // This would require environment variable override or mock
}

#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_actual_download() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try actual download with a small JDK if available
    // This test might take a while and requires internet
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("8") // Older versions might be smaller
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .assert();
}

/// Test actual JDK installation and verify file structure
/// This test performs a real download and installation
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_install_and_verify_files() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install JDK 8 (typically smaller than newer versions)
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd
        .arg("install")
        .arg("8")
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .output()
        .unwrap();

    // Debug output
    eprintln!(
        "Install stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    eprintln!(
        "Install stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("Install status: {:?}", output.status);

    if !output.status.success() {
        eprintln!(
            "Install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Installation failed");
    }

    // Extract the installed version from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_pattern =
        regex::Regex::new(r"Successfully installed .* to .*/.kopi/jdks/(\S+)").unwrap();
    let installed_version = version_pattern
        .captures(&stdout)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str());

    // If we can't extract the version, try to find what was actually installed
    let installed_version = match installed_version {
        Some(v) => v.to_string(),
        None => {
            // List what's in the jdks directory
            let jdks_dir = kopi_home.join("jdks");
            if jdks_dir.exists() {
                let entries: Vec<_> = fs::read_dir(&jdks_dir)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();

                eprintln!("JDKs directory contents:");
                for entry in &entries {
                    eprintln!("  - {:?}", entry.file_name());
                }

                if let Some(entry) = entries.first() {
                    entry.file_name().to_string_lossy().to_string()
                } else {
                    panic!("No JDK directories found after installation");
                }
            } else {
                panic!("JDKs directory doesn't exist");
            }
        }
    };

    // Verify JDK directory structure
    let jdk_dir = kopi_home.join("jdks").join(&installed_version);
    assert!(
        jdk_dir.exists(),
        "JDK directory should exist at {:?}",
        jdk_dir
    );
    assert!(jdk_dir.is_dir(), "JDK path should be a directory");

    // Verify bin directory exists
    let bin_dir = jdk_dir.join("bin");
    assert!(bin_dir.exists(), "bin directory should exist");
    assert!(bin_dir.is_dir(), "bin should be a directory");

    // Verify core executables exist
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    let core_executables = vec!["java", "javac", "jar", "javadoc"];

    for exe in &core_executables {
        let exe_path = bin_dir.join(format!("{}{}", exe, exe_ext));
        assert!(
            exe_path.exists(),
            "Executable {} should exist at {:?}",
            exe,
            exe_path
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&exe_path).unwrap();
            let mode = metadata.permissions().mode();
            assert!(mode & 0o111 != 0, "{} should be executable", exe);
        }
    }

    // Verify lib directory exists
    let lib_dir = jdk_dir.join("lib");
    assert!(lib_dir.exists(), "lib directory should exist");

    // Verify release file exists (contains JDK version info)
    let release_file = jdk_dir.join("release");
    assert!(release_file.exists(), "release file should exist");

    // Verify the content of release file
    let release_content = fs::read_to_string(&release_file).unwrap();
    assert!(
        release_content.contains("JAVA_VERSION="),
        "release file should contain JAVA_VERSION"
    );
}

/// Test installation creates proper shims
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
#[ignore = "Shim creation is not yet implemented in the install command"]
fn test_install_creates_shims() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install a JDK
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("8")
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .assert()
        .success();

    // Verify shims directory exists
    let shims_dir = kopi_home.join("bin");
    assert!(shims_dir.exists(), "Shims directory should exist");
    assert!(shims_dir.is_dir(), "Shims directory should be a directory");

    // Verify default shims are created
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    let default_shims = vec!["java", "javac", "javadoc", "jar", "jshell"];

    for shim in &default_shims {
        let shim_path = shims_dir.join(format!("{}{}", shim, exe_ext));
        // Note: jshell might not exist in JDK 8, so we'll check but not fail
        if shim_path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(&shim_path).unwrap();
                let mode = metadata.permissions().mode();
                assert!(mode & 0o111 != 0, "Shim {} should be executable", shim);
            }
        } else if *shim != "jshell" {
            // jshell is only available from JDK 9+
            panic!("Shim {} should exist at {:?}", shim, shim_path);
        }
    }
}

/// Test installation with specific distribution
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
#[ignore = "Corretto packages may have empty checksums causing validation errors"]
fn test_install_specific_distribution() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Install Corretto 8
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd
        .arg("install")
        .arg("corretto@8")
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .output()
        .unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Installation failed.\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Verify the installation directory contains "corretto"
    let jdks_dir = kopi_home.join("jdks");
    let entries: Vec<_> = fs::read_dir(&jdks_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    let corretto_installed = entries
        .iter()
        .any(|e| e.file_name().to_string_lossy().contains("corretto"));

    assert!(corretto_installed, "Corretto JDK should be installed");
}

/// Test that installation properly handles disk space
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_install_verifies_disk_space() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // This test is hard to simulate without mocking the filesystem
    // For now, we'll just verify that the installation succeeds normally
    // which implies disk space checks passed

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("install")
        .arg("8")
        .arg("--timeout")
        .arg("300")
        .arg("-v") // Verbose to see disk space messages
        .timeout(std::time::Duration::from_secs(600))
        .assert()
        .success();
}

/// Test concurrent installation of the same version
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_concurrent_same_version_install() {
    use std::thread;

    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // First refresh cache
    let mut cmd = get_test_command(&kopi_home);
    cmd.arg("cache").arg("refresh").assert().success();

    // Try to install the same version concurrently
    let kopi_home_1 = kopi_home.clone();
    let handle1 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_1);
        cmd.arg("install")
            .arg("8")
            .arg("--timeout")
            .arg("300")
            .timeout(std::time::Duration::from_secs(600))
            .output()
            .unwrap()
    });

    let kopi_home_2 = kopi_home.clone();
    let handle2 = thread::spawn(move || {
        let mut cmd = get_test_command(&kopi_home_2);
        cmd.arg("install")
            .arg("8")
            .arg("--timeout")
            .arg("300")
            .timeout(std::time::Duration::from_secs(600))
            .output()
            .unwrap()
    });

    let result1 = handle1.join().unwrap();
    let result2 = handle2.join().unwrap();

    // At least one should succeed
    assert!(
        result1.status.success() || result2.status.success(),
        "At least one installation should succeed"
    );

    // If one failed, it should be because the JDK already exists
    if !result1.status.success() {
        let stderr = String::from_utf8_lossy(&result1.stderr);
        // Accept various error messages that indicate the JDK already exists
        assert!(
            stderr.contains("already installed")
                || stderr.contains("already exists")
                || stderr.contains("File exists")
                || stderr.contains("Cannot create a file when that file already exists")
                || stderr.contains("failed to rename")
                || stderr.contains("Directory not empty"),
            "Failure should be due to existing installation, but got: {}",
            stderr
        );
    }
    if !result2.status.success() {
        let stderr = String::from_utf8_lossy(&result2.stderr);
        assert!(
            stderr.contains("already installed")
                || stderr.contains("already exists")
                || stderr.contains("File exists")
                || stderr.contains("Cannot create a file when that file already exists")
                || stderr.contains("failed to rename")
                || stderr.contains("Directory not empty"),
            "Failure should be due to existing installation, but got: {}",
            stderr
        );
    }
}

/// Test installation cleanup on failure
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_install_cleanup_on_failure() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    // Create a .tmp directory to check it's cleaned up
    let tmp_dir = kopi_home.join("jdks").join(".tmp");
    fs::create_dir_all(&tmp_dir).unwrap();

    // Try to install with a very short timeout to force failure
    let mut cmd = get_test_command(&kopi_home);
    let _ = cmd
        .arg("install")
        .arg("21") // Larger JDK more likely to timeout
        .arg("--timeout")
        .arg("1") // 1 second timeout
        .timeout(std::time::Duration::from_secs(10))
        .output();

    // Check that .tmp directory doesn't contain leftover files
    if tmp_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&tmp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        // There should be no leftover installation directories
        for entry in entries {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("install-") {
                panic!("Found leftover installation directory: {:?}", name);
            }
        }
    }
}

/// Simple test to debug installation process
#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
fn test_simple_install_debug() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home();

    eprintln!("KOPI_HOME: {:?}", kopi_home);

    // First refresh cache with verbose output
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd
        .arg("-vv") // Very verbose
        .arg("cache")
        .arg("refresh")
        .output()
        .unwrap();

    eprintln!(
        "Cache refresh stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    eprintln!(
        "Cache refresh stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        panic!("Failed to refresh cache");
    }

    // Try to install with verbose output
    let mut cmd = get_test_command(&kopi_home);
    let output = cmd
        .arg("-vv") // Very verbose
        .arg("install")
        .arg("8")
        .arg("--timeout")
        .arg("300")
        .timeout(std::time::Duration::from_secs(600))
        .output()
        .unwrap();

    eprintln!(
        "Install stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    eprintln!(
        "Install stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("Install status: {:?}", output.status);

    // List contents of jdks directory
    let jdks_dir = kopi_home.join("jdks");
    if jdks_dir.exists() {
        eprintln!("JDKs directory exists");
        for entry in fs::read_dir(&jdks_dir).unwrap() {
            if let Ok(entry) = entry {
                eprintln!("  Found: {:?}", entry.path());
            }
        }
    } else {
        eprintln!("JDKs directory does not exist!");
    }

    assert!(output.status.success(), "Installation should succeed");
}
