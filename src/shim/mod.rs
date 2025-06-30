//! Shim system for transparent JDK version switching.
//!
//! This module implements the core shim binary that intercepts Java tool invocations
//! and routes them to the appropriate JDK version based on project configuration.

use crate::error::{KopiError, Result};
use crate::models::jdk::{Distribution, VersionRequest};
use crate::storage::JdkRepository;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod executor;
pub mod version_resolver;

use executor::ShimExecutor;
use version_resolver::VersionResolver;

/// Main entry point for the shim binary.
///
/// This function:
/// 1. Detects the tool name from argv[0]
/// 2. Resolves the appropriate JDK version
/// 3. Finds the JDK installation path
/// 4. Executes the actual Java tool with all arguments
pub fn run_shim() -> Result<()> {
    let start = std::time::Instant::now();

    // Get tool name from argv[0]
    let tool_name = get_tool_name()?;
    log::debug!("Shim invoked as: {}", tool_name);

    // Resolve JDK version
    let resolver = VersionResolver::new();
    let version_request = resolver.resolve_version()?;
    log::debug!("Resolved version: {:?}", version_request);

    // Find JDK installation
    let repository = JdkRepository::new()?;
    let jdk_path = find_jdk_installation(&repository, &version_request)?;
    log::debug!("JDK path: {:?}", jdk_path);

    // Build tool path
    let tool_path = build_tool_path(&jdk_path, &tool_name)?;
    log::debug!("Tool path: {:?}", tool_path);

    // Collect arguments (skip argv[0])
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    // Log performance
    let elapsed = start.elapsed();
    log::debug!("Shim resolution completed in {:?}", elapsed);

    // Execute the tool
    ShimExecutor::exec(tool_path, args)?;

    // This should not be reached on Unix (exec replaces process)
    Ok(())
}

/// Extract tool name from argv[0].
///
/// Handles both direct invocation (e.g., "java") and path invocation
/// (e.g., "/home/user/.kopi/shims/java").
fn get_tool_name() -> Result<String> {
    let arg0 = env::args_os()
        .next()
        .ok_or_else(|| KopiError::SystemError("No argv[0] found".to_string()))?;

    let path = PathBuf::from(arg0);
    let tool_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| KopiError::SystemError("Invalid tool name in argv[0]".to_string()))?;

    Ok(tool_name.to_string())
}

/// Find the JDK installation path for the given version request.
fn find_jdk_installation(
    repository: &JdkRepository,
    version_request: &VersionRequest,
) -> Result<PathBuf> {
    // Parse distribution from version request
    let distribution = if let Some(dist_name) = &version_request.distribution {
        Distribution::from_str(dist_name)?
    } else {
        // Use default distribution from config or fall back to temurin
        Distribution::Temurin
    };

    // List installed JDKs
    let installed_jdks = repository.list_installed_jdks()?;

    // Find matching JDK
    for jdk in installed_jdks {
        if jdk.distribution.to_lowercase() == distribution.id()
            && version_matches(&jdk.version, &version_request.version_pattern)
        {
            return Ok(jdk.path);
        }
    }

    // No matching JDK found
    Err(KopiError::JdkNotInstalled(format!(
        "{}@{}",
        distribution.id(),
        version_request.version_pattern
    )))
}

/// Check if an installed JDK version matches the requested pattern.
fn version_matches(installed_version: &str, pattern: &str) -> bool {
    // Parse both versions
    if let (Ok(installed), Ok(_requested)) = (
        crate::models::jdk::Version::from_str(installed_version),
        crate::models::jdk::Version::from_str(pattern),
    ) {
        installed.matches_pattern(pattern)
    } else {
        // Fallback to string comparison if parsing fails
        installed_version == pattern
    }
}

/// Build the full path to the Java tool executable.
fn build_tool_path(jdk_path: &Path, tool_name: &str) -> Result<PathBuf> {
    let bin_dir = jdk_path.join("bin");

    #[cfg(target_os = "windows")]
    let tool_path = bin_dir.join(format!("{}.exe", tool_name));

    #[cfg(not(target_os = "windows"))]
    let tool_path = bin_dir.join(tool_name);

    // Verify the tool exists
    if !tool_path.exists() {
        return Err(KopiError::SystemError(format!(
            "Tool '{}' not found in JDK at {:?}",
            tool_name, jdk_path
        )));
    }

    Ok(tool_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_tool_name() {
        // We can't easily test get_tool_name() since it reads from env::args_os()
        // This would require integration tests
    }

    #[test]
    fn test_build_tool_path_unix() {
        #[cfg(not(target_os = "windows"))]
        {
            let temp_dir = TempDir::new().unwrap();
            let jdk_path = temp_dir.path();
            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();

            let java_path = bin_dir.join("java");
            fs::write(&java_path, "").unwrap();

            let result = build_tool_path(jdk_path, "java").unwrap();
            assert_eq!(result, java_path);
        }
    }

    #[test]
    fn test_build_tool_path_windows() {
        #[cfg(target_os = "windows")]
        {
            let temp_dir = TempDir::new().unwrap();
            let jdk_path = temp_dir.path();
            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();

            let java_path = bin_dir.join("java.exe");
            fs::write(&java_path, "").unwrap();

            let result = build_tool_path(jdk_path, "java").unwrap();
            assert_eq!(result, java_path);
        }
    }

    #[test]
    fn test_build_tool_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let result = build_tool_path(jdk_path, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_jdk_installation_found() {
        let temp_dir = TempDir::new().unwrap();
        let _repository = JdkRepository::with_home(temp_dir.path().to_path_buf());

        // Create a mock installed JDK structure
        let jdk_path = temp_dir.path().join("jdks").join("temurin-21.0.1");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create version request
        let _version_request =
            VersionRequest::new("21".to_string()).with_distribution("temurin".to_string());

        // Since we can't easily mock list_installed_jdks, we test with actual filesystem
        // This demonstrates the need for better abstraction in future phases
    }

    #[test]
    fn test_find_jdk_installation_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repository = JdkRepository::with_home(temp_dir.path().to_path_buf());

        let version_request =
            VersionRequest::new("99".to_string()).with_distribution("nonexistent".to_string());

        let result = find_jdk_installation(&repository, &version_request);
        assert!(result.is_err());
        assert!(matches!(result, Err(KopiError::JdkNotInstalled(_))));
    }
}
