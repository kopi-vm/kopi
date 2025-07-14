use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution;
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use std::env;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod discovery;
pub mod installer;
pub mod security;
pub mod tools;
use crate::error::format_error_with_color;
use crate::installation::AutoInstaller;
use crate::version::resolver::VersionResolver;
use security::SecurityValidator;

/// Run the shim with the provided arguments
/// Returns the exit code
pub fn run(_args: Vec<String>) -> Result<i32> {
    // The shim implementation doesn't need the args vector since it reads from env::args_os()
    run_shim()?;
    Ok(0)
}

pub fn run_shim() -> Result<()> {
    let start = std::time::Instant::now();

    // Load configuration once
    let config = new_kopi_config()?;
    let security_validator = SecurityValidator::new(&config);

    // Get tool name from argv[0]
    let tool_name = get_tool_name()?;
    log::debug!("Shim invoked as: {tool_name}");

    // Validate tool name
    security_validator.validate_tool(&tool_name)?;

    // Resolve JDK version
    let resolver = VersionResolver::new();
    let (version_request, version_source) = match resolver.resolve_version() {
        Ok((req, source)) => (req, source),
        Err(e @ KopiError::NoLocalVersion { .. }) => {
            eprintln!(
                "{}",
                format_error_with_color(&e, std::io::stderr().is_terminal())
            );
            std::process::exit(crate::error::get_exit_code(&e));
        }
        Err(e) => return Err(e),
    };
    log::debug!("Resolved version: {version_request:?} from {version_source:?}");

    // Validate version string
    security_validator.validate_version(&version_request.version.to_string())?;
    if let Some(dist) = &version_request.distribution {
        security_validator.validate_version(dist)?;
    }

    // Find JDK installation
    let repository = JdkRepository::new(&config);
    let jdk_path = match find_jdk_installation(&repository, &version_request) {
        Ok(path) => path,
        Err(mut err) => {
            if let KopiError::JdkNotInstalled {
                jdk_spec,
                auto_install_enabled: enabled,
                ..
            } = &mut err
            {
                // Check if auto-install is enabled
                let auto_installer = AutoInstaller::new(&config);
                let auto_install_enabled = auto_installer.should_auto_install();
                *enabled = auto_install_enabled;

                if auto_install_enabled {
                    // Check if we should prompt the user
                    let version_spec = if let Some(dist) = &version_request.distribution {
                        format!("{}@{}", dist, version_request.version)
                    } else {
                        version_request.version.to_string()
                    };

                    let should_install = match auto_installer.prompt_user(&version_spec) {
                        Ok(approved) => approved,
                        Err(e) => {
                            eprintln!(
                                "{}",
                                format_error_with_color(&e, std::io::stderr().is_terminal())
                            );
                            false
                        }
                    };

                    if should_install {
                        // Try to install the JDK
                        match auto_installer.install_jdk(&version_request) {
                            Ok(()) => {
                                // Retry finding the JDK after installation
                                match find_jdk_installation(&repository, &version_request) {
                                    Ok(path) => path,
                                    Err(_) => {
                                        // Still not found after installation attempt
                                        let error = KopiError::JdkNotInstalled {
                                            jdk_spec: jdk_spec.clone(),
                                            version: Some(version_request.version.to_string()),
                                            distribution: version_request.distribution.clone(),
                                            auto_install_enabled,
                                            auto_install_failed: Some(
                                                "Installation succeeded but JDK still not found"
                                                    .to_string(),
                                            ),
                                            user_declined: false,
                                            install_in_progress: false,
                                        };
                                        eprintln!(
                                            "{}",
                                            format_error_with_color(
                                                &error,
                                                std::io::stderr().is_terminal()
                                            )
                                        );
                                        std::process::exit(crate::error::get_exit_code(&error));
                                    }
                                }
                            }
                            Err(e) => {
                                // Check if it's specifically a kopi not found error
                                if let KopiError::KopiNotFound { .. } = &e {
                                    eprintln!(
                                        "{}",
                                        format_error_with_color(
                                            &e,
                                            std::io::stderr().is_terminal()
                                        )
                                    );
                                    std::process::exit(crate::error::get_exit_code(&e));
                                }

                                // Auto-install failed for other reasons
                                let error = KopiError::JdkNotInstalled {
                                    jdk_spec: jdk_spec.clone(),
                                    version: Some(version_request.version.to_string()),
                                    distribution: version_request.distribution.clone(),
                                    auto_install_enabled,
                                    auto_install_failed: Some(e.to_string()),
                                    user_declined: false,
                                    install_in_progress: false,
                                };
                                eprintln!(
                                    "{}",
                                    format_error_with_color(
                                        &error,
                                        std::io::stderr().is_terminal()
                                    )
                                );
                                std::process::exit(crate::error::get_exit_code(&error));
                            }
                        }
                    } else {
                        // User declined installation
                        let error = KopiError::JdkNotInstalled {
                            jdk_spec: jdk_spec.clone(),
                            version: Some(version_request.version.to_string()),
                            distribution: version_request.distribution.clone(),
                            auto_install_enabled,
                            auto_install_failed: None,
                            user_declined: true,
                            install_in_progress: false,
                        };
                        eprintln!(
                            "{}",
                            format_error_with_color(&error, std::io::stderr().is_terminal())
                        );
                        std::process::exit(crate::error::get_exit_code(&error));
                    }
                } else {
                    eprintln!(
                        "{}",
                        format_error_with_color(&err, std::io::stderr().is_terminal())
                    );
                    std::process::exit(crate::error::get_exit_code(&err));
                }
            } else {
                return Err(err);
            }
        }
    };
    log::debug!("JDK path: {jdk_path:?}");

    // Build tool path
    let tool_path = build_tool_path(&jdk_path, &tool_name)?;
    log::debug!("Tool path: {tool_path:?}");

    // Collect arguments (skip argv[0])
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    // Validate tool path and permissions before execution
    security_validator.validate_path(&tool_path)?;
    security_validator.check_permissions(&tool_path)?;

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
            && version_matches(&jdk.version, &version_request.version.to_string())
        {
            return Ok(jdk.path);
        }
    }

    // No matching JDK found
    Err(KopiError::JdkNotInstalled {
        jdk_spec: format!("{}@{}", distribution.id(), version_request.version),
        version: Some(version_request.version.to_string()),
        distribution: Some(distribution.id().to_string()),
        auto_install_enabled: false, // Will be updated by caller
        auto_install_failed: None,
        user_declined: false,
        install_in_progress: false,
    })
}

fn version_matches(installed_version: &str, pattern: &str) -> bool {
    // Parse both versions
    if let (Ok(installed), Ok(_requested)) = (
        crate::version::Version::from_str(installed_version),
        crate::version::Version::from_str(pattern),
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
            // List available tools in the JDK bin directory
            let mut available_tools = Vec::new();

            if bin_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&bin_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            // Remove .exe extension on Windows
                            let tool_name_clean = if cfg!(windows) && name.ends_with(".exe") {
                                &name[..name.len() - 4]
                            } else {
                                name
                            };

                            // Only include executable files
                            if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                                available_tools.push(tool_name_clean.to_string());
                            }
                        }
                    }
                }
            }

            available_tools.sort();

            let error = KopiError::ToolNotFound {
                tool: tool_name.to_string(),
                jdk_path: jdk_path.to_str().unwrap_or("<invalid path>").to_string(),
                available_tools,
            };
            eprintln!(
                "{}",
                format_error_with_color(&error, std::io::stderr().is_terminal())
            );
            std::process::exit(crate::error::get_exit_code(&error));
        }

        #[cfg(test)]
        return Err(KopiError::SystemError(format!(
            "Tool '{tool_name}' not found in JDK at {jdk_path:?}"
        )));
    }

    Ok(tool_path)
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
        let _version_request = VersionRequest::new("21".to_string())
            .unwrap()
            .with_distribution("temurin".to_string());

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

        let version_request = VersionRequest::new("99".to_string())
            .unwrap()
            .with_distribution("nonexistent".to_string());

        let result = find_jdk_installation(&repository, &version_request);
        assert!(result.is_err());
        assert!(matches!(result, Err(KopiError::JdkNotInstalled { .. })));
    }
}
