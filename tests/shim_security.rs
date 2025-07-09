use kopi::config::KopiConfig;
use kopi::error::KopiError;
use kopi::shim::security::SecurityValidator;
use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};

fn create_test_validator_with_dir(temp_dir: &TempDir) -> SecurityValidator {
    std::fs::create_dir_all(temp_dir.path()).unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
    SecurityValidator::new(&config)
}

#[test]
fn test_path_traversal_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Create a safe directory inside kopi_home
    let safe_dir = temp_dir.path().join("jdks").join("java-11");
    fs::create_dir_all(&safe_dir).unwrap();

    // Test various path traversal attempts
    let malicious_paths = vec![
        "../../../etc/passwd",
        "jdks/../../../../../../etc/passwd",
        "jdks/../../../root/.ssh/id_rsa",
        "/etc/passwd",
        "jdks/java-11/../../../../etc/shadow",
    ];

    for path in malicious_paths {
        let test_path = temp_dir.path().join(path);
        let result = validator.validate_path(&test_path);
        assert!(result.is_err(), "Path '{path}' should be rejected");
        assert!(
            matches!(
                result,
                Err(KopiError::SecurityError(_)) | Err(KopiError::SystemError(_))
            ),
            "Expected SecurityError or SystemError for path '{path}'"
        );
    }

    // Test that valid paths within kopi_home are accepted
    assert!(validator.validate_path(&safe_dir).is_ok());
}

#[test]
#[cfg(unix)]
fn test_symlink_target_validation() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Create directories
    let safe_target = temp_dir.path().join("safe_target");
    let unsafe_target = Path::new("/etc/passwd");
    let link_dir = temp_dir.path().join("links");
    fs::create_dir_all(&link_dir).unwrap();
    File::create(&safe_target).unwrap();

    // Test safe symlink (pointing inside kopi_home)
    let safe_link = link_dir.join("safe_link");
    symlink(&safe_target, &safe_link).unwrap();
    assert!(
        validator.validate_symlink(&safe_link).is_ok(),
        "Safe symlink should be accepted"
    );

    // Test unsafe symlink (pointing outside kopi_home)
    let unsafe_link = link_dir.join("unsafe_link");
    symlink(unsafe_target, &unsafe_link).unwrap();
    assert!(
        validator.validate_symlink(&unsafe_link).is_err(),
        "Unsafe symlink should be rejected"
    );

    // Test relative symlink that escapes kopi_home
    let escape_link = link_dir.join("escape_link");
    symlink("../../../../../etc/passwd", &escape_link).unwrap();
    assert!(
        validator.validate_symlink(&escape_link).is_err(),
        "Escaping symlink should be rejected"
    );

    // Test non-symlink file (should pass)
    assert!(
        validator.validate_symlink(&safe_target).is_ok(),
        "Regular file should pass symlink validation"
    );
}

#[test]
fn test_version_string_validation() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Valid version strings
    let valid_versions = vec![
        "21",
        "11.0.2",
        "temurin@21.0.1",
        "graalvm-ce@22.3.0",
        "openjdk@17_35",
        "corretto@11.0.21.9.1",
        "zulu@8.0.372+07",
        "17+35",
        "11.0.2_9",
    ];

    for version in valid_versions {
        assert!(
            validator.validate_version(version).is_ok(),
            "Version '{version}' should be valid"
        );
    }

    // Invalid version strings
    let long_version = "x".repeat(101);
    let invalid_versions = vec![
        "",                     // Empty
        "../../../etc/passwd",  // Path traversal
        "java; rm -rf /",       // Command injection
        "java\necho hacked",    // Newline injection
        "java|cat /etc/passwd", // Pipe injection
        "java$(whoami)",        // Command substitution
        "java`id`",             // Backtick injection
        "java && echo pwned",   // Command chaining
        &long_version,          // Too long
        "java//bin//sh",        // Double slashes
        "java..version",        // Double dots
        "java\0null",           // Null byte
        "java\x1b[31mred",      // ANSI escape
        "java%00null",          // URL encoded null
        "java${PATH}",          // Environment variable
    ];

    for version in invalid_versions {
        let result = validator.validate_version(version);
        assert!(
            result.is_err(),
            "Version '{}' should be invalid",
            version.replace('\0', "\\0").replace('\n', "\\n")
        );
        assert!(
            matches!(
                result,
                Err(KopiError::ValidationError(_)) | Err(KopiError::SecurityError(_))
            ),
            "Expected ValidationError or SecurityError for version '{}'",
            version.replace('\0', "\\0").replace('\n', "\\n")
        );
    }
}

#[test]
#[cfg(unix)]
fn test_permission_verification() {
    use std::fs::Permissions;

    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Test executable file
    let exec_file = temp_dir.path().join("java");
    File::create(&exec_file).unwrap();
    fs::set_permissions(&exec_file, Permissions::from_mode(0o755)).unwrap();
    assert!(
        validator.check_permissions(&exec_file).is_ok(),
        "Executable file with proper permissions should pass"
    );

    // Test non-executable file
    let non_exec_file = temp_dir.path().join("data.txt");
    File::create(&non_exec_file).unwrap();
    fs::set_permissions(&non_exec_file, Permissions::from_mode(0o644)).unwrap();
    assert!(
        validator.check_permissions(&non_exec_file).is_err(),
        "Non-executable file should fail"
    );

    // Test world-writable executable
    let world_writable = temp_dir.path().join("vulnerable");
    File::create(&world_writable).unwrap();
    fs::set_permissions(&world_writable, Permissions::from_mode(0o777)).unwrap();
    let result = validator.check_permissions(&world_writable);
    assert!(result.is_err(), "World-writable file should fail");
    assert!(
        result.unwrap_err().to_string().contains("world-writable"),
        "Error should mention world-writable"
    );

    // Test various permission combinations
    let test_cases = vec![
        (0o700, true, "Owner executable only"),
        (0o750, true, "Owner and group executable"),
        (0o755, true, "Everyone can execute, but not write"),
        (0o775, true, "Group writable but not world writable"),
        (0o777, false, "World writable should fail"),
        (0o666, false, "Read/write but not executable"),
        (0o444, false, "Read-only should fail"),
    ];

    for (mode, should_pass, description) in test_cases {
        let test_file = temp_dir.path().join(format!("test_{:o}", mode));
        File::create(&test_file).unwrap();
        fs::set_permissions(&test_file, Permissions::from_mode(mode)).unwrap();

        let result = validator.check_permissions(&test_file);
        if should_pass {
            assert!(result.is_ok(), "{description} (mode {mode:o}) should pass");
        } else {
            assert!(result.is_err(), "{description} (mode {mode:o}) should fail");
        }
    }
}

#[test]
#[cfg(windows)]
fn test_windows_executable_extension() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Test .exe file
    let exe_file = temp_dir.path().join("java.exe");
    File::create(&exe_file).unwrap();
    assert!(
        validator.check_permissions(&exe_file).is_ok(),
        "Windows .exe file should pass"
    );

    // Test non-.exe file
    let non_exe_file = temp_dir.path().join("java");
    File::create(&non_exe_file).unwrap();
    assert!(
        validator.check_permissions(&non_exe_file).is_err(),
        "Windows file without .exe extension should fail"
    );
}

#[test]
fn test_directory_validation() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Create a directory
    let dir_path = temp_dir.path().join("not_a_file");
    fs::create_dir(&dir_path).unwrap();

    // Check permissions should fail for directories
    assert!(
        validator.check_permissions(&dir_path).is_err(),
        "Directories should not pass permission check"
    );
}

#[test]
fn test_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    let nonexistent = temp_dir.path().join("does_not_exist");
    assert!(
        validator.check_permissions(&nonexistent).is_err(),
        "Nonexistent files should fail permission check"
    );
}

#[test]
fn test_complex_path_validation() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Create nested structure
    let nested_path = temp_dir
        .path()
        .join("jdks")
        .join("vendor")
        .join("version")
        .join("bin");
    fs::create_dir_all(&nested_path).unwrap();

    // Valid nested path
    let valid_nested = nested_path.join("java");
    File::create(&valid_nested).unwrap();
    assert!(validator.validate_path(&valid_nested).is_ok());

    // Path with special characters (but still valid)
    let special_chars_dir = temp_dir.path().join("jdk-11.0.2+9");
    fs::create_dir(&special_chars_dir).unwrap();
    assert!(validator.validate_path(&special_chars_dir).is_ok());
}

#[test]
fn test_tool_validation() {
    let temp_dir = TempDir::new().unwrap();
    let validator = create_test_validator_with_dir(&temp_dir);

    // Standard JDK tools that should be recognized
    let valid_tools = vec![
        "java",
        "javac",
        "javap",
        "javadoc",
        "jar",
        "jshell",
        "keytool",
        "jarsigner",
        "native-image",
    ];

    for tool in valid_tools {
        assert!(
            validator.validate_tool(tool).is_ok(),
            "Tool '{tool}' should be recognized"
        );
    }

    // Non-JDK tools that should be rejected
    let invalid_tools = vec![
        "rm",
        "ls",
        "cat",
        "echo",
        "sh",
        "bash",
        "cmd",
        "powershell",
        "python",
        "node",
        "curl",
        "wget",
        "unknown-tool",
    ];

    for tool in invalid_tools {
        assert!(
            validator.validate_tool(tool).is_err(),
            "Tool '{tool}' should not be recognized"
        );
    }
}
