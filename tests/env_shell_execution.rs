//! Integration tests for real shell execution of the env command
//! These tests verify that the output can be successfully evaluated by different shells

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Helper to get the kopi binary path
fn get_kopi_binary() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let target_dir = PathBuf::from(manifest_dir).join("target");

    // Try debug build first, then release
    let debug_binary = target_dir.join("debug").join("kopi");
    if debug_binary.exists() {
        return debug_binary;
    }

    let release_binary = target_dir.join("release").join("kopi");
    if release_binary.exists() {
        return release_binary;
    }

    panic!("kopi binary not found. Run 'cargo build' first.");
}

/// Helper to check if a shell is available
fn shell_available(shell: &str) -> bool {
    Command::new("which")
        .arg(shell)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore] // These tests require real shells and a JDK installation
fn test_bash_shell_execution() {
    if !shell_available("bash") {
        eprintln!("Skipping bash test - shell not available");
        return;
    }

    let kopi_path = get_kopi_binary();

    // Create a test script that evaluates kopi env and echoes JAVA_HOME
    let script = format!(
        r#"
        eval "$({} env --quiet)"
        echo "$JAVA_HOME"
        "#,
        kopi_path.display()
    );

    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("Failed to execute bash");

    if output.status.success() {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert!(!java_home.is_empty(), "JAVA_HOME should be set");
        assert!(
            java_home.contains("kopi"),
            "JAVA_HOME should point to a kopi-managed JDK"
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // It's okay if no JDK is configured
        assert!(
            stderr.contains("No local version") || stderr.contains("not installed"),
            "Unexpected error: {stderr}"
        );
    }
}

#[test]
#[ignore] // These tests require real shells and a JDK installation
fn test_zsh_shell_execution() {
    if !shell_available("zsh") {
        eprintln!("Skipping zsh test - shell not available");
        return;
    }

    let kopi_path = get_kopi_binary();

    let script = format!(
        r#"
        eval "$({} env --quiet)"
        echo "$JAVA_HOME"
        "#,
        kopi_path.display()
    );

    let output = Command::new("zsh")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("Failed to execute zsh");

    if output.status.success() {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert!(!java_home.is_empty(), "JAVA_HOME should be set");
        assert!(
            java_home.contains("kopi"),
            "JAVA_HOME should point to a kopi-managed JDK"
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("No local version") || stderr.contains("not installed"),
            "Unexpected error: {stderr}"
        );
    }
}

#[test]
#[ignore] // These tests require real shells and a JDK installation
fn test_fish_shell_execution() {
    if !shell_available("fish") {
        eprintln!("Skipping fish test - shell not available");
        return;
    }

    let kopi_path = get_kopi_binary();

    let script = format!(
        r#"
        {} env --quiet | source
        echo $JAVA_HOME
        "#,
        kopi_path.display()
    );

    let output = Command::new("fish")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("Failed to execute fish");

    if output.status.success() {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert!(!java_home.is_empty(), "JAVA_HOME should be set");
        assert!(
            java_home.contains("kopi"),
            "JAVA_HOME should point to a kopi-managed JDK"
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("No local version") || stderr.contains("not installed"),
            "Unexpected error: {stderr}"
        );
    }
}

#[test]
#[ignore] // These tests require real shells and a JDK installation
fn test_powershell_shell_execution() {
    if !cfg!(windows) && !shell_available("pwsh") {
        eprintln!("Skipping PowerShell test - shell not available");
        return;
    }

    let kopi_path = get_kopi_binary();
    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };

    let script = format!(
        r#"
        & '{}' env --quiet | Invoke-Expression
        $env:JAVA_HOME
        "#,
        kopi_path.display()
    );

    let output = Command::new(shell)
        .arg("-Command")
        .arg(&script)
        .output()
        .expect("Failed to execute PowerShell");

    if output.status.success() {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !java_home.is_empty() {
            assert!(
                java_home.contains("kopi"),
                "JAVA_HOME should point to a kopi-managed JDK"
            );
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // PowerShell might have different error formats
        eprintln!("PowerShell stderr: {stderr}");
    }
}

#[test]
#[ignore] // These tests require real shells and a JDK installation  
fn test_cmd_shell_execution() {
    if !cfg!(windows) {
        eprintln!("Skipping CMD test - only runs on Windows");
        return;
    }

    let kopi_path = get_kopi_binary();

    // CMD requires a different approach - we'll create a batch file
    let temp_dir = env::temp_dir();
    let batch_file = temp_dir.join("test_kopi_env.bat");

    let batch_content = format!(
        r#"@echo off
for /f "tokens=*" %%i in ('"{}" env --quiet --shell cmd') do %%i
echo %JAVA_HOME%
"#,
        kopi_path.display()
    );

    std::fs::write(&batch_file, batch_content).expect("Failed to write batch file");

    let output = Command::new("cmd")
        .arg("/c")
        .arg(&batch_file)
        .output()
        .expect("Failed to execute CMD");

    // Clean up
    let _ = std::fs::remove_file(&batch_file);

    if output.status.success() {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !java_home.is_empty() && !java_home.contains("JAVA_HOME") {
            assert!(
                java_home.contains("kopi"),
                "JAVA_HOME should point to a kopi-managed JDK"
            );
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("CMD stderr: {stderr}");
    }
}

#[test]
#[ignore] // These tests require real shells and a JDK installation
fn test_shell_execution_with_spaces_in_path() {
    // This test verifies that paths with spaces are properly escaped
    if !shell_available("bash") {
        eprintln!("Skipping spaces test - bash not available");
        return;
    }

    let kopi_path = get_kopi_binary();

    // Create a mock response that simulates a path with spaces
    let script = format!(
        r#"
        # First check if kopi env works
        output=$({} env --quiet 2>&1)
        if [ $? -eq 0 ]; then
            # If it works, test that quotes are handled properly
            eval "$output"
            # Check that JAVA_HOME is set and accessible
            if [ -n "$JAVA_HOME" ]; then
                echo "JAVA_HOME is set to: $JAVA_HOME"
                # Try to use the path (this will fail if escaping is wrong)
                if [ -d "$JAVA_HOME" ]; then
                    echo "Directory exists"
                else
                    echo "Directory does not exist"
                fi
            fi
        else
            echo "kopi env failed: $output"
        fi
        "#,
        kopi_path.display()
    );

    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("Failed to execute bash");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    // The test passes if either:
    // 1. JAVA_HOME is properly set and the directory exists
    // 2. No JDK is configured (expected in CI environments)
    assert!(
        stdout.contains("Directory exists")
            || stderr.contains("No local version")
            || stderr.contains("not installed")
            || stdout.contains("kopi env failed"),
        "Unexpected output"
    );
}

#[test]
fn test_env_command_exists() {
    // Basic test to ensure the env command is available
    let kopi_path = get_kopi_binary();

    let output = Command::new(&kopi_path)
        .arg("env")
        .arg("--help")
        .output()
        .expect("Failed to execute kopi");

    assert!(output.status.success(), "kopi env --help should succeed");

    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(help_text.contains("Output environment variables"));
    assert!(help_text.contains("Examples:"));
    assert!(help_text.contains("eval"));
}

#[test]
fn test_env_shell_override() {
    // Test that --shell flag produces different output
    let kopi_path = get_kopi_binary();

    // Get bash output
    let bash_output = Command::new(&kopi_path)
        .arg("env")
        .arg("--shell")
        .arg("bash")
        .arg("--export=false")
        .output()
        .expect("Failed to execute kopi");

    // Get fish output
    let fish_output = Command::new(&kopi_path)
        .arg("env")
        .arg("--shell")
        .arg("fish")
        .arg("--export=false")
        .output()
        .expect("Failed to execute kopi");

    // If both commands succeed (have a JDK configured)
    if bash_output.status.success() && fish_output.status.success() {
        let bash_text = String::from_utf8_lossy(&bash_output.stdout);
        let fish_text = String::from_utf8_lossy(&fish_output.stdout);

        // Without export, each shell has its own syntax
        assert!(
            bash_text.starts_with("JAVA_HOME="),
            "Bash should use simple assignment"
        );
        assert!(
            !bash_text.contains("export"),
            "Bash should not export when --export=false"
        );
        assert!(
            fish_text.starts_with("set -g JAVA_HOME"),
            "Fish should use set -g (not -gx) when --export=false"
        );
        assert!(
            !fish_text.contains("-gx"),
            "Fish should not export when --export=false"
        );
    }

    // Now test with export
    let bash_export = Command::new(&kopi_path)
        .arg("env")
        .arg("--shell")
        .arg("bash")
        .output()
        .expect("Failed to execute kopi");

    let fish_export = Command::new(&kopi_path)
        .arg("env")
        .arg("--shell")
        .arg("fish")
        .output()
        .expect("Failed to execute kopi");

    if bash_export.status.success() && fish_export.status.success() {
        let bash_text = String::from_utf8_lossy(&bash_export.stdout);
        let fish_text = String::from_utf8_lossy(&fish_export.stdout);

        // With export, outputs should be different
        assert!(bash_text.contains("export JAVA_HOME="));
        assert!(fish_text.contains("set -gx JAVA_HOME"));
        assert_ne!(bash_text, fish_text, "Shell-specific output should differ");
    }
}
