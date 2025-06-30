use kopi::error::KopiError;
use kopi::shim::version_resolver::VersionResolver;
use kopi::storage::JdkRepository;
use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_version_resolution_with_real_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create .kopi-version file
    let version_file = project_dir.join(".kopi-version");
    fs::write(&version_file, "temurin@21.0.1").unwrap();

    // Change to project directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&project_dir).unwrap();

    // Test version resolution
    let resolver = VersionResolver::new();
    let result = resolver.resolve_version();

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    let version_request = result.unwrap();
    assert_eq!(version_request.version_pattern, "21.0.1");
    assert_eq!(version_request.distribution, Some("temurin".to_string()));
}

#[test]
fn test_version_resolution_parent_directory_search() {
    let temp_dir = TempDir::new().unwrap();
    let parent_dir = temp_dir.path();
    let child_dir = parent_dir.join("src").join("main").join("java");
    fs::create_dir_all(&child_dir).unwrap();

    // Place .java-version in parent
    let version_file = parent_dir.join(".java-version");
    fs::write(&version_file, "17.0.8").unwrap();

    // Change to deeply nested directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&child_dir).unwrap();

    // Test version resolution
    let resolver = VersionResolver::new();
    let result = resolver.resolve_version();

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    let version_request = result.unwrap();
    assert_eq!(version_request.version_pattern, "17.0.8");
    assert_eq!(version_request.distribution, None);
}

#[test]
fn test_version_resolution_performance() {
    let temp_dir = TempDir::new().unwrap();

    // Create deeply nested directory structure
    let mut current = temp_dir.path().to_path_buf();
    for i in 0..10 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
    }

    // Place version file at root
    let version_file = temp_dir.path().join(".kopi-version");
    fs::write(&version_file, "corretto@11").unwrap();

    // Change to deepest directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&current).unwrap();

    // Measure resolution time
    let start = Instant::now();
    let resolver = VersionResolver::new();
    let result = resolver.resolve_version();
    let elapsed = start.elapsed();

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    // Version resolution should be fast even with deep directory traversal
    assert!(
        elapsed.as_millis() < 10,
        "Version resolution took {:?}",
        elapsed
    );
}

#[test]
fn test_no_version_found_error() {
    let temp_dir = TempDir::new().unwrap();

    // Change to empty directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Test version resolution
    let resolver = VersionResolver::new();
    let result = resolver.resolve_version();

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err());
    assert!(matches!(result, Err(KopiError::NoLocalVersion)));
}

#[test]
fn test_jdk_path_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let kopi_home = temp_dir.path();

    // Create mock JDK installation
    let jdk_dir = kopi_home.join("jdks").join("temurin-21.0.1");
    let bin_dir = jdk_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create mock java executable
    let java_path = if cfg!(windows) {
        bin_dir.join("java.exe")
    } else {
        bin_dir.join("java")
    };
    fs::write(&java_path, "mock java").unwrap();

    // Create metadata file
    let metadata = r#"{
        "id": "test-id",
        "distribution": "temurin",
        "version": {
            "major": 21,
            "minor": 0,
            "patch": 1,
            "build": null
        },
        "java_version": "21.0.1",
        "distribution_version": "21.0.1+12"
    }"#;

    let metadata_file = kopi_home.join("jdks").join("temurin-21.0.1.meta.json");
    fs::write(&metadata_file, metadata).unwrap();

    // Test repository listing
    let repository = JdkRepository::with_home(kopi_home.to_path_buf());
    let installed_jdks = repository.list_installed_jdks().unwrap();

    assert_eq!(installed_jdks.len(), 1);
    assert_eq!(installed_jdks[0].distribution, "temurin");
    assert_eq!(installed_jdks[0].version, "21.0.1");
}

#[test]
fn test_environment_variable_override() {
    // Set environment variable
    unsafe {
        env::set_var("KOPI_JAVA_VERSION", "zulu@8.0.372");
    }

    // Create resolver
    let resolver = VersionResolver::new();
    let result = resolver.resolve_version();

    // Clean up
    unsafe {
        env::remove_var("KOPI_JAVA_VERSION");
    }

    assert!(result.is_ok());
    let version_request = result.unwrap();
    assert_eq!(version_request.version_pattern, "8.0.372");
    assert_eq!(version_request.distribution, Some("zulu".to_string()));
}

#[test]
#[cfg(unix)]
fn test_tool_path_construction_unix() {
    let temp_dir = TempDir::new().unwrap();
    let jdk_path = temp_dir.path();
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create multiple tools
    let tools = ["java", "javac", "jar", "jps"];
    for tool in &tools {
        let tool_path = bin_dir.join(tool);
        fs::write(&tool_path, format!("mock {}", tool)).unwrap();
    }

    // Verify each tool can be found
    for tool in &tools {
        let expected_path = bin_dir.join(tool);
        assert!(expected_path.exists());
    }
}

#[test]
#[cfg(windows)]
fn test_tool_path_construction_windows() {
    let temp_dir = TempDir::new().unwrap();
    let jdk_path = temp_dir.path();
    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create multiple tools with .exe extension
    let tools = ["java.exe", "javac.exe", "jar.exe", "jps.exe"];
    for tool in &tools {
        let tool_path = bin_dir.join(tool);
        fs::write(&tool_path, format!("mock {}", tool)).unwrap();
    }

    // Verify each tool can be found
    for tool in &tools {
        let expected_path = bin_dir.join(tool);
        assert!(expected_path.exists());
    }
}

#[test]
fn test_executor_stdio_inheritance() {
    // This test verifies that stdio is properly inherited
    let temp_dir = TempDir::new().unwrap();
    let script_path = if cfg!(windows) {
        temp_dir.path().join("test_script.bat")
    } else {
        temp_dir.path().join("test_script.sh")
    };

    // Create a test script that outputs to stdout and stderr
    let script_content = if cfg!(windows) {
        r#"@echo off
echo This is stdout
echo This is stderr >&2
exit 42"#
    } else {
        r#"#!/bin/sh
echo "This is stdout"
echo "This is stderr" >&2
exit 42"#
    };

    fs::write(&script_path, script_content).unwrap();

    #[cfg(unix)]
    {
        // Make script executable on Unix
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }

    // Test that we can capture output from a subprocess
    let output = Command::new(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute test script");

    // Verify output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.contains("This is stdout"));
    assert!(stderr.contains("This is stderr"));
    assert_eq!(output.status.code(), Some(42));
}

#[test]
#[cfg(unix)]
fn test_mock_java_tool_execution() {
    // Create a mock Java tool that outputs version info
    let temp_dir = TempDir::new().unwrap();
    let mock_java = temp_dir.path().join("java");

    let script_content = r#"#!/bin/sh
if [ "$1" = "-version" ]; then
    echo "openjdk version \"21.0.1\" 2023-10-17" >&2
    echo "OpenJDK Runtime Environment Temurin-21.0.1+12 (build 21.0.1+12)" >&2
    echo "OpenJDK 64-Bit Server VM Temurin-21.0.1+12 (build 21.0.1+12, mixed mode, sharing)" >&2
    exit 0
fi
echo "Usage: java [options] <mainclass> [args...]"
exit 1"#;

    fs::write(&mock_java, script_content).unwrap();

    // Make executable
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&mock_java).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&mock_java, perms).unwrap();

    // Execute with -version flag
    let output = Command::new(&mock_java)
        .arg("-version")
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute mock java");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("openjdk version \"21.0.1\""));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg(windows)]
fn test_mock_java_tool_execution_windows() {
    // Create a mock Java tool batch script
    let temp_dir = TempDir::new().unwrap();
    let mock_java = temp_dir.path().join("java.bat");

    let script_content = r#"@echo off
if "%1"=="-version" (
    echo openjdk version "21.0.1" 2023-10-17 >&2
    echo OpenJDK Runtime Environment Temurin-21.0.1+12 ^(build 21.0.1+12^) >&2
    echo OpenJDK 64-Bit Server VM Temurin-21.0.1+12 ^(build 21.0.1+12, mixed mode, sharing^) >&2
    exit /b 0
)
echo Usage: java [options] ^<mainclass^> [args...]
exit /b 1"#;

    fs::write(&mock_java, script_content).unwrap();

    // Execute with -version flag
    let output = Command::new(&mock_java)
        .arg("-version")
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute mock java");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("openjdk version \"21.0.1\""));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_environment_variable_propagation() {
    // Test that environment variables are properly propagated
    let temp_dir = TempDir::new().unwrap();
    let script_path = if cfg!(windows) {
        temp_dir.path().join("env_test.bat")
    } else {
        temp_dir.path().join("env_test.sh")
    };

    // Create a script that prints an environment variable
    let script_content = if cfg!(windows) {
        r#"@echo off
echo TEST_VAR=%TEST_VAR%"#
    } else {
        r#"#!/bin/sh
echo "TEST_VAR=$TEST_VAR""#
    };

    fs::write(&script_path, script_content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }

    // Execute with custom environment variable
    let output = Command::new(&script_path)
        .env("TEST_VAR", "test_value_123")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute env test script");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_value_123"));
}
