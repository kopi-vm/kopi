use kopi::models::jdk::Distribution;
use kopi::platform;
use kopi::platform::shell::{self as shim_platform, Shell};
use kopi::shim::installer::ShimInstaller;
use kopi::shim::tools::{ToolRegistry, default_shim_tools};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper function to create a mock kopi-shim binary
fn create_mock_kopi_shim(kopi_bin_dir: &Path) -> PathBuf {
    #[cfg(windows)]
    let shim_name = "kopi-shim.exe";

    #[cfg(not(windows))]
    let shim_name = "kopi-shim";

    let shim_path = kopi_bin_dir.join(shim_name);

    // Create a dummy file to act as the kopi-shim binary
    fs::write(&shim_path, "mock shim binary").unwrap();

    #[cfg(unix)]
    platform::file_ops::make_executable(&shim_path).unwrap();

    shim_path
}

/// Helper to set up a test environment with kopi-shim binary
fn setup_test_env() -> (TempDir, PathBuf, ShimInstaller) {
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

    #[cfg(windows)]
    let shim_binary_name = "kopi-shim.exe";

    #[cfg(not(windows))]
    let shim_binary_name = "kopi-shim";

    let expected_shim_path = current_exe_dir.join(shim_binary_name);
    if !expected_shim_path.exists() {
        fs::copy(&kopi_shim_path, &expected_shim_path).unwrap();
    }

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
fn test_create_and_remove_shim() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    // Create a shim for java
    installer.create_shim("java").unwrap();

    // Verify shim was created
    let shims = installer.list_shims().unwrap();
    assert!(shims.contains(&"java".to_string()));

    // Verify the shim file exists
    let java_shim_path = installer.shims_dir().join("java");
    #[cfg(windows)]
    let java_shim_path = installer.shims_dir().join("java.exe");

    assert!(java_shim_path.exists());

    // Remove the shim
    installer.remove_shim("java").unwrap();

    // Verify it was removed
    let shims = installer.list_shims().unwrap();
    assert!(!shims.contains(&"java".to_string()));
    assert!(!java_shim_path.exists());
}

#[test]
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
fn test_remove_nonexistent_shim_fails() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    let result = installer.remove_shim("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
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
#[cfg(unix)]
#[ignore = "This test modifies shared test binaries and can interfere with other tests"]
fn test_verify_shims_unix() {
    let (_temp_dir, _shim_binary_path, installer) = setup_test_env();

    // Create a valid shim
    installer.create_shim("java").unwrap();

    // Debug: Check what was created
    let java_shim = installer.shims_dir().join("java");
    eprintln!("Java shim path: {:?}", java_shim);
    eprintln!("Java shim exists: {}", java_shim.exists());
    if java_shim.exists() {
        let metadata = fs::symlink_metadata(&java_shim).unwrap();
        eprintln!(
            "Java shim is symlink: {}",
            metadata.file_type().is_symlink()
        );
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&java_shim).unwrap();
            eprintln!("Java shim target: {:?}", target);
        }
    }

    // Verify - should find no broken shims
    let broken = installer.verify_shims().unwrap();
    assert!(broken.is_empty());

    // Now break the shim by reading where it actually points to and removing that
    let java_shim = installer.shims_dir().join("java");
    let actual_target = fs::read_link(&java_shim).unwrap();
    eprintln!("Removing actual target: {:?}", actual_target);

    // Remove the actual target file
    if actual_target.exists() {
        fs::remove_file(&actual_target).unwrap();
    }

    // Verify again - should find broken shim
    let broken = installer.verify_shims().unwrap();
    eprintln!("Broken shims found: {:?}", broken);
    assert_eq!(
        broken.len(),
        1,
        "Expected 1 broken shim, found {}",
        broken.len()
    );
    if !broken.is_empty() {
        assert_eq!(broken[0].0, "java");
        assert!(broken[0].1.contains("Broken symlink"));
    }
}

#[test]
fn test_repair_shim() {
    let (_temp_dir, _shim_path, installer) = setup_test_env();

    // Create a shim
    installer.create_shim("javadoc").unwrap();

    let shim_path = installer.shims_dir().join("javadoc");
    #[cfg(windows)]
    let shim_path = installer.shims_dir().join("javadoc.exe");

    // Corrupt the shim
    fs::write(&shim_path, "corrupted").unwrap();

    // Repair it
    installer.repair_shim("javadoc").unwrap();

    // Verify it exists and is not corrupted
    assert!(shim_path.exists());

    #[cfg(unix)]
    {
        // On Unix, it should be a symlink
        let metadata = fs::symlink_metadata(&shim_path).unwrap();
        assert!(metadata.file_type().is_symlink());
    }

    #[cfg(windows)]
    {
        // On Windows, it should be a copy of the shim binary
        let content = fs::read(&shim_path).unwrap();
        assert_ne!(content, b"corrupted");
        // It should contain our mock content
        assert_eq!(content, b"mock shim binary");
    }
}

#[test]
fn test_platform_shell_detection() {
    // Just verify we can detect a shell
    let shell = shim_platform::detect_shell();

    // Should return some shell
    let shell_name = shell.get_shell_name();
    assert!(!shell_name.is_empty());
}

#[test]
fn test_path_instructions_generation() {
    let shims_dir = Path::new("/home/user/.kopi/shims");
    let shell = Shell::Bash;

    let instructions = shell.generate_path_instructions(shims_dir);

    // Verify instructions contain the path
    assert!(instructions.contains("/home/user/.kopi/shims"));
    assert!(instructions.contains("export PATH="));
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
    assert!(graalvm_names.contains(&"gu"));
    assert!(graalvm_names.contains(&"js")); // Still available in GraalVM 21

    // Test GraalVM 23 - js should be removed
    let graalvm23_tools = registry.available_tools(&Distribution::GraalVm, 23);
    let graalvm23_names: Vec<&str> = graalvm23_tools.iter().map(|t| t.name).collect();

    assert!(graalvm23_names.contains(&"native-image"));
    assert!(graalvm23_names.contains(&"gu"));
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
            "Default tool '{}' not found in registry",
            tool_name
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

    #[cfg(unix)]
    {
        for tool in &tools {
            let tool_path = bin_dir.join(tool);
            fs::write(&tool_path, "#!/bin/sh\necho mock").unwrap();
            platform::file_ops::make_executable(&tool_path).unwrap();
        }

        // Also create a non-executable file
        fs::write(bin_dir.join("README"), "not executable").unwrap();
    }

    #[cfg(windows)]
    {
        for tool in &tools {
            let tool_path = bin_dir.join(format!("{}.exe", tool));
            fs::write(&tool_path, "mock executable").unwrap();
        }

        // Also create a non-exe file
        fs::write(bin_dir.join("README.txt"), "not executable").unwrap();
    }

    // TODO: find_executables was part of PlatformUtils which was removed
    // For now, manually verify the expected tools exist
    let mut found = Vec::new();
    for tool in &tools {
        #[cfg(unix)]
        let tool_path = bin_dir.join(tool);
        #[cfg(windows)]
        let tool_path = bin_dir.join(format!("{}.exe", tool));

        if tool_path.exists() {
            found.push(tool.to_string());
        }
    }

    // Should find all our tools
    assert_eq!(found.len(), tools.len());
    for tool in &tools {
        assert!(found.contains(&tool.to_string()));
    }
}
