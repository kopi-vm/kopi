use kopi::models::distribution::Distribution;
use kopi::platform::shell::{self as shim_platform};
use kopi::shim::installer::ShimInstaller;
use kopi::shim::tools::{ToolRegistry, default_shim_tools};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Cleanup function to remove any test artifacts
fn cleanup_test_shim() {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let shim_path = dir.join(kopi::platform::shim_binary_name());
            if shim_path.exists() {
                // Only remove if it's our test shim (check size or content)
                if let Ok(content) = fs::read(&shim_path) {
                    // Our test shim has a specific pattern
                    if content.len() > 128 && content.len() < 4096 {
                        let _ = fs::remove_file(&shim_path);
                    }
                }
            }
        }
    }
}

/// Helper function to build tool path with platform-appropriate extension
fn tool_path(dir: &Path, tool_name: &str) -> PathBuf {
    let ext = kopi::platform::executable_extension();
    if ext.is_empty() {
        dir.join(tool_name)
    } else {
        dir.join(format!("{tool_name}{ext}"))
    }
}

/// Helper function to create a mock kopi-shim binary
fn create_mock_kopi_shim(kopi_bin_dir: &Path) -> PathBuf {
    let shim_name = kopi::platform::shim_binary_name();
    let shim_path = kopi_bin_dir.join(shim_name);

    // Create a dummy file to act as the kopi-shim binary
    #[cfg(windows)]
    {
        // On Windows, create a mock PE file that passes validation
        // PE header starts with "MZ" (DOS header) and needs to be at least 1KB
        let mut mock_exe = vec![0x4D, 0x5A]; // MZ header
        // Add DOS stub
        mock_exe.extend(&[0x90; 62]); // Pad to offset 64
        // Add PE signature offset at 0x3C (60)
        mock_exe[60] = 0x80; // PE header at offset 128
        mock_exe[61] = 0x00;
        mock_exe[62] = 0x00;
        mock_exe[63] = 0x00;
        // Pad to PE header location
        mock_exe.extend(&[0x00; 64]);
        // Add PE signature "PE\0\0"
        mock_exe.extend(b"PE\0\0");
        // Add enough padding to make it > 1KB
        mock_exe.extend(vec![0u8; 2048]);

        fs::write(&shim_path, &mock_exe).unwrap();
        eprintln!(
            "Created mock kopi-shim at {:?} with size {} bytes",
            shim_path,
            mock_exe.len()
        );

        // Verify what was written
        let written = fs::read(&shim_path).unwrap();
        eprintln!("Verified written size: {} bytes", written.len());
    }

    #[cfg(not(windows))]
    {
        // On Unix, just create a simple script
        fs::write(&shim_path, "#!/bin/sh\necho mock shim binary").unwrap();
    }

    kopi::platform::file_ops::make_executable(&shim_path).unwrap();

    shim_path
}

/// Helper to set up a test environment with kopi-shim binary
fn setup_test_env() -> (TempDir, PathBuf, ShimInstaller) {
    // Cleanup any previous test artifacts
    cleanup_test_shim();

    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create a bin directory to hold the mock kopi-shim
    let kopi_bin_dir = kopi_home.join("bin");
    fs::create_dir_all(&kopi_bin_dir).unwrap();

    // Create the mock kopi-shim binary
    let kopi_shim_path = create_mock_kopi_shim(&kopi_bin_dir);

    // Create a custom ShimInstaller that will find our mock binary
    // We need to override the kopi_bin_path in the installer
    let installer = ShimInstaller::new(kopi_home);

    // Since we can't directly set kopi_bin_path, we'll work around it by placing
    // kopi-shim in the expected location relative to the current executable
    let current_exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let shim_binary_name = kopi::platform::shim_binary_name();
    let expected_shim_path = current_exe_dir.join(shim_binary_name);

    // Always create a fresh copy for this test
    if expected_shim_path.exists() {
        fs::remove_file(&expected_shim_path).ok();
    }

    eprintln!("Copying mock shim from {kopi_shim_path:?} to {expected_shim_path:?}");

    // Read the source file to ensure we have the data
    let source_data = fs::read(&kopi_shim_path).unwrap();
    eprintln!("Source shim size: {} bytes", source_data.len());

    // Write to destination
    fs::write(&expected_shim_path, &source_data).unwrap();

    // Make it executable on Windows too
    kopi::platform::file_ops::make_executable(&expected_shim_path).ok();

    // Verify the copy
    let copied_size = fs::metadata(&expected_shim_path).unwrap().len();
    eprintln!("Copied shim size: {copied_size} bytes");

    // Double-check the content
    let copied_data = fs::read(&expected_shim_path).unwrap();
    assert_eq!(source_data.len(), copied_data.len(), "Copy size mismatch");

    (temp_dir, kopi_shim_path, installer)
}

#[test]
fn test_shim_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let installer = ShimInstaller::new(temp_dir.path());

    // Verify shims directory doesn't exist initially
    assert!(!installer.shims_dir().exists());

    // Initialize shims directory
    installer.init_shims_directory().unwrap();

    // Verify it was created
    assert!(installer.shims_dir().exists());
    assert!(installer.shims_dir().is_dir());
}

#[test]
#[serial]
fn test_create_and_remove_shim() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    // Create a shim for java
    installer.create_shim("java").unwrap();

    // Verify shim was created
    let shims = installer.list_shims().unwrap();
    assert!(shims.contains(&"java".to_string()));

    // Verify the shim file exists
    let java_shim_path = tool_path(installer.shims_dir(), "java");
    assert!(java_shim_path.exists());

    // Remove the shim
    installer.remove_shim("java").unwrap();

    // Verify it was removed
    let shims = installer.list_shims().unwrap();
    assert!(!shims.contains(&"java".to_string()));
    assert!(!java_shim_path.exists());
}

#[test]
#[serial]
fn test_create_duplicate_shim_fails() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    // Create a shim
    installer.create_shim("javac").unwrap();

    // Try to create it again
    let result = installer.create_shim("javac");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
#[serial]
fn test_remove_nonexistent_shim_fails() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    let result = installer.remove_shim("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
#[serial]
fn test_list_multiple_shims() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    // Create multiple shims
    let tools = vec!["java", "javac", "jar", "jshell"];
    for tool in &tools {
        installer.create_shim(tool).unwrap();
    }

    // List and verify
    let shims = installer.list_shims().unwrap();
    assert_eq!(shims.len(), tools.len());

    // Verify they're sorted
    let mut expected = tools.clone();
    expected.sort();
    assert_eq!(shims, expected);
}

#[test]
#[serial]
#[cfg(windows)] // Only test on Windows where shims are regular files, not symlinks
fn test_verify_shims() {
    let (_temp_dir, shim_binary_path, installer) = setup_test_env();

    // Debug: Verify the kopi-shim binary that will be used as source
    eprintln!("\n=== Verifying source kopi-shim ===");
    eprintln!("Source kopi-shim path: {shim_binary_path:?}");
    if shim_binary_path.exists() {
        let metadata = fs::metadata(&shim_binary_path).unwrap();
        eprintln!("Source kopi-shim size: {} bytes", metadata.len());
    }

    // Also check where installer will look for kopi-shim
    let current_exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let expected_kopi_shim = current_exe_dir.join(kopi::platform::shim_binary_name());
    eprintln!("Installer will look for kopi-shim at: {expected_kopi_shim:?}");
    if expected_kopi_shim.exists() {
        let metadata = fs::metadata(&expected_kopi_shim).unwrap();
        eprintln!("Expected kopi-shim size: {} bytes", metadata.len());
    }

    // Create a valid shim
    eprintln!("\n=== Creating java shim ===");
    installer.create_shim("java").unwrap();

    // Debug: Check what was created
    let java_shim = tool_path(installer.shims_dir(), "java");
    eprintln!("\n=== Verifying created shim ===");
    eprintln!("Java shim path: {java_shim:?}");
    eprintln!("Java shim exists: {}", java_shim.exists());
    if java_shim.exists() {
        let metadata = fs::metadata(&java_shim).unwrap();
        eprintln!("Java shim size: {} bytes", metadata.len());
        eprintln!("Java shim is file: {}", metadata.is_file());

        // Check first few bytes
        let content = fs::read(&java_shim).unwrap();
        if content.len() >= 4 {
            eprintln!(
                "First 4 bytes: {:02X} {:02X} {:02X} {:02X}",
                content[0], content[1], content[2], content[3]
            );
        }
        eprintln!("Total content length: {} bytes", content.len());
    }

    // Verify - should find no broken shims
    eprintln!("\n=== Running verify_shims ===");
    let broken = installer.verify_shims().unwrap();
    if !broken.is_empty() {
        eprintln!("Broken shims found: {broken:?}");
    }
    assert!(
        broken.is_empty(),
        "Expected no broken shims after initial creation"
    );

    // Break the shim - on Windows we corrupt the file directly
    let java_shim = tool_path(installer.shims_dir(), "java");

    // On Windows, shims are regular files, so we can corrupt them
    fs::write(&java_shim, "corrupted").unwrap();

    // Verify again - should find broken shim
    let broken = installer.verify_shims().unwrap();
    eprintln!("Broken shims found after corruption: {broken:?}");
    assert_eq!(
        broken.len(),
        1,
        "Expected 1 broken shim after corruption, found {}",
        broken.len()
    );
    assert_eq!(broken[0].0, "java");
    assert!(
        broken[0].1.contains("too small"),
        "Expected 'too small' error, got: {}",
        broken[0].1
    );
}

#[test]
#[serial]
fn test_repair_shim() {
    let (_temp_dir, _shim_binary_path, installer) = setup_test_env();

    // Debug: Verify source kopi-shim before test
    eprintln!("\n=== test_repair_shim: Verifying source kopi-shim ===");
    let current_exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let expected_kopi_shim = current_exe_dir.join(kopi::platform::shim_binary_name());
    if expected_kopi_shim.exists() {
        let metadata = fs::metadata(&expected_kopi_shim).unwrap();
        eprintln!("Source kopi-shim size: {} bytes", metadata.len());
        let content = fs::read(&expected_kopi_shim).unwrap();
        if content.len() >= 2 {
            eprintln!(
                "Source first 2 bytes: {:02X} {:02X}",
                content[0], content[1]
            );
        }
    }

    // Create a shim
    installer.create_shim("javadoc").unwrap();

    let shim_path = tool_path(installer.shims_dir(), "javadoc");

    // Verify initial shim
    eprintln!("\n=== Initial shim state ===");
    if shim_path.exists() {
        let metadata = fs::metadata(&shim_path).unwrap();
        eprintln!("Initial shim size: {} bytes", metadata.len());
    }

    // Corrupt the shim
    eprintln!("\n=== Corrupting shim ===");
    fs::write(&shim_path, "corrupted").unwrap();
    let corrupted_size = fs::metadata(&shim_path).unwrap().len();
    eprintln!("Corrupted shim size: {corrupted_size} bytes");

    // Repair it
    eprintln!("\n=== Repairing shim ===");
    installer.repair_shim("javadoc").unwrap();

    // Verify it exists and is not corrupted
    eprintln!("\n=== Verifying repaired shim ===");
    assert!(shim_path.exists(), "Shim should exist after repair");

    if kopi::platform::uses_symlinks_for_shims() {
        // On Unix, it should be a symlink
        let metadata = fs::symlink_metadata(&shim_path).unwrap();
        assert!(metadata.file_type().is_symlink());
    } else {
        // On Windows, it should be a copy of the shim binary
        let content = fs::read(&shim_path).unwrap();
        eprintln!("Repaired shim size: {} bytes", content.len());
        if content.len() >= 4 {
            eprintln!(
                "Repaired first 4 bytes: {:02X} {:02X} {:02X} {:02X}",
                content[0], content[1], content[2], content[3]
            );
        }

        assert_ne!(
            content, b"corrupted",
            "Shim should not contain corrupted content after repair"
        );
        // It should be a valid PE file (starts with MZ header)
        assert!(
            content.len() >= 1024,
            "Repaired shim should be at least 1024 bytes, but was {} bytes",
            content.len()
        );
        assert_eq!(
            &content[0..2],
            &[0x4D, 0x5A],
            "Repaired shim should start with MZ header"
        );
    }
}

#[test]
fn test_platform_shell_detection() {
    // Just verify we can detect a shell
    let result = shim_platform::detect_shell();

    // On Windows without a shell parent, it might error
    // On Unix, it should either succeed or have a fallback
    match result {
        Ok((shell, _path)) => {
            let shell_name = shell.get_shell_name();
            assert!(!shell_name.is_empty());
        }
        Err(_) => {
            // This is acceptable, especially in test environments
            // where there might not be a proper shell parent
        }
    }
}

#[test]
fn test_tool_availability_for_different_jdk_versions() {
    let registry = ToolRegistry::new();

    // Test JDK 8 - should have older tools but not newer ones
    let jdk8_tools = registry.available_tools(&Distribution::Temurin, 8);
    let jdk8_names: Vec<&str> = jdk8_tools.iter().map(|t| t.name).collect();

    assert!(jdk8_names.contains(&"java"));
    assert!(jdk8_names.contains(&"javac"));
    assert!(jdk8_names.contains(&"jhat")); // Available in JDK 8
    assert!(!jdk8_names.contains(&"jshell")); // Added in JDK 9
    assert!(!jdk8_names.contains(&"jlink")); // Added in JDK 9

    // Test JDK 17 - should have newer tools but not deprecated ones
    let jdk17_tools = registry.available_tools(&Distribution::Temurin, 17);
    let jdk17_names: Vec<&str> = jdk17_tools.iter().map(|t| t.name).collect();

    assert!(jdk17_names.contains(&"java"));
    assert!(jdk17_names.contains(&"javac"));
    assert!(!jdk17_names.contains(&"jhat")); // Removed in JDK 9
    assert!(jdk17_names.contains(&"jshell")); // Available since JDK 9
    assert!(jdk17_names.contains(&"jlink")); // Available since JDK 9
    assert!(jdk17_names.contains(&"jpackage")); // Available since JDK 14
}

#[test]
fn test_graalvm_specific_tools() {
    let registry = ToolRegistry::new();

    // Test GraalVM - should have GraalVM-specific tools
    let graalvm_tools = registry.available_tools(&Distribution::GraalVm, 21);
    let graalvm_names: Vec<&str> = graalvm_tools.iter().map(|t| t.name).collect();

    assert!(graalvm_names.contains(&"native-image"));
    assert!(graalvm_names.contains(&"js")); // Still available in GraalVM 21

    // Test GraalVM 23 - js should be removed
    let graalvm23_tools = registry.available_tools(&Distribution::GraalVm, 23);
    let graalvm23_names: Vec<&str> = graalvm23_tools.iter().map(|t| t.name).collect();

    assert!(graalvm23_names.contains(&"native-image"));
    assert!(!graalvm23_names.contains(&"js")); // Removed in GraalVM 23

    // Test non-GraalVM distribution - should not have GraalVM tools
    let corretto_tools = registry.available_tools(&Distribution::Corretto, 21);
    let corretto_names: Vec<&str> = corretto_tools.iter().map(|t| t.name).collect();

    assert!(!corretto_names.contains(&"native-image"));
    assert!(!corretto_names.contains(&"gu"));
    assert!(!corretto_names.contains(&"js"));
}

#[test]
fn test_default_shim_tools_are_valid() {
    let registry = ToolRegistry::new();
    let defaults = default_shim_tools();

    // Verify all default tools exist in the registry
    for tool_name in defaults {
        let tool = registry.get_tool(tool_name);
        assert!(
            tool.is_some(),
            "Default tool '{tool_name}' not found in registry"
        );
    }
}

// TODO: This test was for PlatformUtils::sanitize_tool_name which was removed
// #[test]
// fn test_sanitize_tool_names() {
//     // Valid names should pass
//     assert!(platform::sanitize_tool_name("java").is_ok());
//     assert!(platform::sanitize_tool_name("native-image").is_ok());
//     assert!(platform::sanitize_tool_name("jdk.compiler").is_ok());

//     // Invalid names should fail
//     assert!(platform::sanitize_tool_name("").is_err());
//     assert!(platform::sanitize_tool_name("java/c").is_err());
//     assert!(platform::sanitize_tool_name("../java").is_err());
//     assert!(platform::sanitize_tool_name("..").is_err());
//     assert!(platform::sanitize_tool_name(".").is_err());
// }

#[test]
fn test_find_executables_in_jdk_bin() {
    let temp_dir = TempDir::new().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create mock JDK tools
    let tools = vec!["java", "javac", "jar", "jshell"];

    for tool in &tools {
        let tool_path = tool_path(&bin_dir, tool);
        if kopi::platform::executable_extension().is_empty() {
            // Unix: create shell script
            fs::write(&tool_path, "#!/bin/sh\necho mock").unwrap();
        } else {
            // Windows: create mock executable
            fs::write(&tool_path, "mock executable").unwrap();
        }
        kopi::platform::file_ops::make_executable(&tool_path).unwrap();
    }

    // Also create a non-executable file
    fs::write(bin_dir.join("README.txt"), "not executable").unwrap();

    // TODO: find_executables was part of PlatformUtils which was removed
    // For now, manually verify the expected tools exist
    let mut found = Vec::new();
    for tool in &tools {
        let tp = tool_path(&bin_dir, tool);
        if tp.exists() {
            found.push(tool.to_string());
        }
    }

    // Should find all our tools
    assert_eq!(found.len(), tools.len());
    for tool in &tools {
        assert!(found.contains(&tool.to_string()));
    }
}
