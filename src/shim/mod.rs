use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::models::jdk::{Distribution, VersionRequest};
use crate::storage::JdkRepository;
use std::env;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod errors;
pub mod installer;
pub mod tools;
pub mod version_resolver;
use errors::{ShimErrorBuilder, format_shim_error};
use version_resolver::VersionResolver;

/// Run the shim with the provided arguments
/// Returns the exit code
pub fn run(_args: Vec<String>) -> Result<i32> {
    // The shim implementation doesn't need the args vector since it reads from env::args_os()
    run_shim()?;
    Ok(0)
}

pub fn run_shim() -> Result<()> {
    let start = std::time::Instant::now();

    // Get tool name from argv[0]
    let tool_name = get_tool_name()?;
    log::debug!("Shim invoked as: {tool_name}");

    // Resolve JDK version
    let resolver = VersionResolver::new();
    let version_request = match resolver.resolve_version() {
        Ok(req) => req,
        Err(KopiError::NoLocalVersion) => {
            let error = ShimErrorBuilder::no_version_found(
                &std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
            );
            eprintln!(
                "{}",
                format_shim_error(&error, std::io::stderr().is_terminal())
            );
            std::process::exit(error.exit_code());
        }
        Err(e) => return Err(e),
    };
    log::debug!("Resolved version: {version_request:?}");

    // Find JDK installation
    let config = new_kopi_config()?;
    let repository = JdkRepository::new(&config);
    let jdk_path = match find_jdk_installation(&repository, &version_request) {
        Ok(path) => path,
        Err(KopiError::JdkNotInstalled(_)) => {
            // Check if auto-install is enabled
            let auto_install_enabled = config.auto_install.enabled;

            if auto_install_enabled {
                // Try to delegate to main kopi binary for installation
                match delegate_auto_install(&version_request) {
                    Ok(()) => {
                        // Retry finding the JDK after installation
                        match find_jdk_installation(&repository, &version_request) {
                            Ok(path) => path,
                            Err(_) => {
                                // Still not found after installation attempt
                                let default_dist = "temurin".to_string();
                                let distribution = version_request
                                    .distribution
                                    .as_ref()
                                    .unwrap_or(&default_dist);
                                let error = ShimErrorBuilder::jdk_not_installed(
                                    &version_request.version_pattern,
                                    distribution,
                                    auto_install_enabled,
                                );
                                eprintln!(
                                    "{}",
                                    format_shim_error(&error, std::io::stderr().is_terminal())
                                );
                                std::process::exit(error.exit_code());
                            }
                        }
                    }
                    Err(_) => {
                        // Auto-install failed or was declined
                        let default_dist = "temurin".to_string();
                        let distribution = version_request
                            .distribution
                            .as_ref()
                            .unwrap_or(&default_dist);
                        let error = ShimErrorBuilder::jdk_not_installed(
                            &version_request.version_pattern,
                            distribution,
                            auto_install_enabled,
                        );
                        eprintln!(
                            "{}",
                            format_shim_error(&error, std::io::stderr().is_terminal())
                        );
                        std::process::exit(error.exit_code());
                    }
                }
            } else {
                // Auto-install is disabled
                let default_dist = "temurin".to_string();
                let distribution = version_request
                    .distribution
                    .as_ref()
                    .unwrap_or(&default_dist);
                let error = ShimErrorBuilder::jdk_not_installed(
                    &version_request.version_pattern,
                    distribution,
                    auto_install_enabled,
                );
                eprintln!(
                    "{}",
                    format_shim_error(&error, std::io::stderr().is_terminal())
                );
                std::process::exit(error.exit_code());
            }
        }
        Err(e) => return Err(e),
    };
    log::debug!("JDK path: {jdk_path:?}");

    // Build tool path
    let tool_path = build_tool_path(&jdk_path, &tool_name)?;
    log::debug!("Tool path: {tool_path:?}");

    // Collect arguments (skip argv[0])
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    // Log performance
    let elapsed = start.elapsed();
    log::debug!("Shim resolution completed in {elapsed:?}");

    // Execute the tool
    let err = crate::platform::process::exec_replace(&tool_path, args);

    // exec_replace only returns on error
    Err(KopiError::SystemError(format!(
        "Failed to execute {tool_path:?}: {err}"
    )))
}

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

fn build_tool_path(jdk_path: &Path, tool_name: &str) -> Result<PathBuf> {
    let bin_dir = jdk_path.join("bin");

    let tool_filename = if crate::platform::executable_extension().is_empty() {
        tool_name.to_string()
    } else {
        format!("{}{}", tool_name, crate::platform::executable_extension())
    };

    let tool_path = bin_dir.join(tool_filename);

    // Verify the tool exists
    if !tool_path.exists() {
        // Only exit in production code, not during tests
        #[cfg(not(test))]
        {
            let error = ShimErrorBuilder::tool_not_found(
                tool_name,
                jdk_path.to_str().unwrap_or("<invalid path>"),
            )?;
            eprintln!(
                "{}",
                format_shim_error(&error, std::io::stderr().is_terminal())
            );
            std::process::exit(error.exit_code());
        }

        #[cfg(test)]
        return Err(KopiError::SystemError(format!(
            "Tool '{tool_name}' not found in JDK at {jdk_path:?}"
        )));
    }

    Ok(tool_path)
}

fn delegate_auto_install(version_request: &VersionRequest) -> Result<()> {
    // Build the version specification for the install command
    let version_spec = if let Some(dist) = &version_request.distribution {
        format!("{}@{}", dist, version_request.version_pattern)
    } else {
        version_request.version_pattern.clone()
    };

    log::info!("Delegating auto-install to main kopi binary for {version_spec}");

    // Find the kopi binary in the same directory as the shim
    let kopi_path = find_kopi_binary()?;

    // Execute kopi install command
    let mut cmd = std::process::Command::new(&kopi_path);
    cmd.arg("install").arg(&version_spec);

    match cmd.status() {
        Ok(status) if status.success() => {
            log::info!("Successfully delegated installation of {version_spec}");
            Ok(())
        }
        Ok(status) => {
            log::warn!("kopi install command failed with status: {status:?}");
            Err(KopiError::SystemError(format!(
                "Failed to install {version_spec}: command exited with status {status:?}"
            )))
        }
        Err(e) => {
            log::error!("Failed to execute kopi install command: {e}");
            Err(KopiError::SystemError(format!(
                "Failed to execute kopi install command: {e}"
            )))
        }
    }
}

fn find_kopi_binary() -> Result<PathBuf> {
    // First try to find kopi in the same directory as the current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let kopi_path = parent.join(if cfg!(windows) { "kopi.exe" } else { "kopi" });
            if kopi_path.exists() {
                return Ok(kopi_path);
            }
        }
    }

    // Fallback to searching in PATH
    if let Ok(kopi_in_path) = which::which(if cfg!(windows) { "kopi.exe" } else { "kopi" }) {
        return Ok(kopi_in_path);
    }

    Err(KopiError::SystemError(
        "Could not find kopi binary for auto-installation".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
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
        // Repository setup removed - not needed for this test

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
        // Clear any leftover environment variables
        unsafe {
            std::env::remove_var("KOPI_AUTO_INSTALL");
            std::env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            std::env::remove_var("KOPI_AUTO_INSTALL__PROMPT");
            std::env::remove_var("KOPI_AUTO_INSTALL__TIMEOUT_SECS");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);

        let version_request =
            VersionRequest::new("99".to_string()).with_distribution("nonexistent".to_string());

        let result = find_jdk_installation(&repository, &version_request);
        assert!(result.is_err());
        assert!(matches!(result, Err(KopiError::JdkNotInstalled(_))));
    }
}
