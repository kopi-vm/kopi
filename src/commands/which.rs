use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::storage::{InstalledJdk, JdkRepository};
use crate::version::VersionRequest;
use crate::version::resolver::{VersionResolver, VersionSource};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Serialize)]
struct WhichOutput {
    distribution: String,
    version: String,
    tool: String,
    tool_path: String,
    jdk_home: String,
    source: String,
}

pub struct WhichCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> WhichCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, version: Option<&str>, tool: &str, home: bool, json: bool) -> Result<()> {
        let repo = JdkRepository::new(self.config);

        // Resolve JDK spec
        let (version_request, source) = if let Some(version) = version {
            // Parse specified version
            let request = VersionRequest::from_str(version)?;
            (request, "specified".to_string())
        } else {
            // Use current version resolution
            let resolver = VersionResolver::new(self.config);
            let (version_request, version_source) = resolver.resolve_version()?;
            let source = format_source(&version_source);
            (version_request, source)
        };

        // Find installed JDK
        let matching_jdks = repo.find_matching_jdks(&version_request)?;
        let installation = if matching_jdks.is_empty() {
            return Err(KopiError::JdkNotInstalled {
                jdk_spec: version_request.to_string(),
                version: Some(version_request.version_pattern.clone()),
                distribution: version_request.distribution.clone(),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            });
        } else if matching_jdks.len() == 1 {
            matching_jdks.into_iter().next().unwrap()
        } else {
            // Multiple matches - need disambiguation
            return Err(KopiError::ValidationError(format!(
                "Multiple JDKs match version '{}'\n\nFound:\n  {}\n\nPlease specify the full \
                 version or distribution",
                version_request.version_pattern,
                matching_jdks
                    .iter()
                    .map(|jdk| format!("{}@{}", jdk.distribution, jdk.version))
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )));
        };

        // Determine output path
        let output_path = if home {
            installation.path.clone()
        } else {
            get_tool_path(&installation, tool)?
        };

        // Output result
        if json {
            output_json(&installation, tool, &output_path, &source)?;
        } else {
            println!("{}", output_path.display());
        }

        Ok(())
    }
}

fn format_source(source: &VersionSource) -> String {
    match source {
        VersionSource::Environment(_) => "environment".to_string(),
        VersionSource::ProjectFile(path) => {
            format!("project file: {}", path.display())
        }
        VersionSource::GlobalDefault(_) => "global default".to_string(),
    }
}

fn get_tool_path(installation: &InstalledJdk, tool: &str) -> Result<PathBuf> {
    let tool_name = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    };

    let tool_path = installation.path.join("bin").join(&tool_name);

    if !tool_path.exists() {
        // Get list of available tools in the bin directory
        let bin_dir = installation.path.join("bin");
        let mut available_tools = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() || file_type.is_symlink() {
                        if let Some(name) = entry.file_name().to_str() {
                            // Remove .exe extension on Windows for cleaner listing
                            let tool_name = if cfg!(windows) && name.ends_with(".exe") {
                                &name[..name.len() - 4]
                            } else {
                                name
                            };
                            available_tools.push(tool_name.to_string());
                        }
                    }
                }
            }
        }

        available_tools.sort();

        return Err(KopiError::ToolNotFound {
            tool: tool.to_string(),
            jdk_path: installation.path.display().to_string(),
            available_tools,
        });
    }

    Ok(tool_path)
}

fn output_json(
    installation: &InstalledJdk,
    tool: &str,
    tool_path: &Path,
    source: &str,
) -> Result<()> {
    let output = WhichOutput {
        distribution: installation.distribution.clone(),
        version: installation.version.to_string(),
        tool: tool.to_string(),
        tool_path: tool_path.display().to_string(),
        jdk_home: installation.path.display().to_string(),
        source: source.to_string(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use std::fs;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn create_test_jdk(temp_dir: &TempDir, distribution: &str, version: &str) -> PathBuf {
        let jdk_path = temp_dir
            .path()
            .join("jdks")
            .join(format!("{distribution}-{version}"));

        let bin_dir = jdk_path.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Create test tools
        for tool in &["java", "javac", "jar", "jshell"] {
            let tool_path = if cfg!(windows) {
                bin_dir.join(format!("{tool}.exe"))
            } else {
                bin_dir.join(tool)
            };
            fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();

            #[cfg(unix)]
            {
                let metadata = fs::metadata(&tool_path).unwrap();
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&tool_path, perms).unwrap();
            }
        }

        jdk_path
    }

    fn setup_test_environment(temp_dir: &TempDir, distribution: &str, version: &str) -> KopiConfig {
        let jdk_path = create_test_jdk(temp_dir, distribution, version);

        // Create version metadata file
        let metadata = serde_json::json!({
            "distribution": distribution,
            "version": version,
        });
        let metadata_path = jdk_path.join("kopi-metadata.json");
        fs::write(&metadata_path, serde_json::to_string(&metadata).unwrap()).unwrap();

        KopiConfig::new(temp_dir.path().to_path_buf()).unwrap()
    }

    #[test]
    fn test_version_request_from_str() {
        // Test simple version
        let request = VersionRequest::from_str("21").unwrap();
        assert_eq!(request.version_pattern, "21");
        assert_eq!(request.distribution, None);

        // Test distribution@version format
        let request = VersionRequest::from_str("temurin@21.0.5").unwrap();
        assert_eq!(request.version_pattern, "21.0.5");
        assert_eq!(request.distribution, Some("temurin".to_string()));

        // Test package_type@version@distribution format (3 parts)
        let request = VersionRequest::from_str("jre@21@temurin").unwrap();
        assert_eq!(request.version_pattern, "21");
        assert_eq!(request.distribution, Some("temurin".to_string()));
        assert_eq!(
            request.package_type,
            Some(crate::models::package::PackageType::Jre)
        );
    }

    #[test]
    fn test_which_specific_version() {
        let temp_dir = TempDir::new().unwrap();
        let config = setup_test_environment(&temp_dir, "temurin", "21.0.5+11");

        let command = WhichCommand::new(&config).unwrap();
        let result = command.execute(Some("temurin@21"), "java", false, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_which_tool_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = setup_test_environment(&temp_dir, "temurin", "21.0.5+11");

        let command = WhichCommand::new(&config).unwrap();
        let result = command.execute(Some("temurin@21"), "nonexistent-tool", false, false);

        match result {
            Err(KopiError::ToolNotFound { tool, .. }) => {
                assert_eq!(tool, "nonexistent-tool");
            }
            _ => panic!("Expected ToolNotFound error"),
        }
    }

    #[test]
    fn test_which_home_option() {
        let temp_dir = TempDir::new().unwrap();
        let config = setup_test_environment(&temp_dir, "temurin", "21.0.5+11");

        let command = WhichCommand::new(&config).unwrap();
        // Home option should return JDK home directory
        let result = command.execute(Some("temurin@21"), "java", true, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_which_json_output() {
        let temp_dir = TempDir::new().unwrap();
        let config = setup_test_environment(&temp_dir, "temurin", "21.0.5+11");

        let command = WhichCommand::new(&config).unwrap();

        // Capture stdout for JSON output test
        let result = std::panic::catch_unwind(|| {
            command
                .execute(Some("temurin@21"), "javac", false, true)
                .unwrap();
        });

        // JSON output would be printed to stdout
        assert!(result.is_ok());
    }

    #[test]
    fn test_ambiguous_version() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple JDKs with same major version
        let _jdk1 = create_test_jdk(&temp_dir, "temurin", "21.0.5+11");
        let _jdk2 = create_test_jdk(&temp_dir, "corretto", "21.0.7.6.1");

        // Create metadata for both
        let metadata1 = serde_json::json!({
            "distribution": "temurin",
            "version": "21.0.5+11",
        });
        let metadata2 = serde_json::json!({
            "distribution": "corretto",
            "version": "21.0.7.6.1",
        });

        fs::write(
            _jdk1.join("kopi-metadata.json"),
            serde_json::to_string(&metadata1).unwrap(),
        )
        .unwrap();
        fs::write(
            _jdk2.join("kopi-metadata.json"),
            serde_json::to_string(&metadata2).unwrap(),
        )
        .unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let command = WhichCommand::new(&config).unwrap();
        let result = command.execute(Some("21"), "java", false, false);

        match result {
            Err(KopiError::ValidationError(msg)) => {
                assert!(msg.contains("Multiple JDKs match"));
                assert!(msg.contains("temurin@21"));
                assert!(msg.contains("corretto@21"));
            }
            _ => panic!("Expected ValidationError for ambiguous version"),
        }
    }

    #[test]
    fn test_version_request_display() {
        let request = VersionRequest::new("21".to_string()).unwrap();
        assert_eq!(request.to_string(), "21");

        let request = VersionRequest::new("21".to_string())
            .unwrap()
            .with_distribution("temurin".to_string());
        assert_eq!(request.to_string(), "temurin@21");
    }

    #[test]
    fn test_get_tool_path() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = create_test_jdk(&temp_dir, "temurin", "21.0.5");

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: crate::version::Version::from_str("21.0.5").unwrap(),
            path: jdk_path,
        };

        // Test existing tool
        let java_path = get_tool_path(&jdk, "java").unwrap();
        assert!(java_path.exists());
        assert!(java_path.ends_with(if cfg!(windows) { "java.exe" } else { "java" }));

        // Test non-existent tool
        let result = get_tool_path(&jdk, "nonexistent");
        assert!(result.is_err());
        if let Err(KopiError::ToolNotFound {
            tool,
            available_tools,
            ..
        }) = result
        {
            assert_eq!(tool, "nonexistent");
            assert!(available_tools.contains(&"java".to_string()));
            assert!(available_tools.contains(&"javac".to_string()));
        } else {
            panic!("Expected ToolNotFound error");
        }
    }

    #[test]
    fn test_get_tool_path_various_tools() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = create_test_jdk(&temp_dir, "temurin", "21.0.5");

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::from_str("21.0.5").unwrap(),
            path: jdk_path,
        };

        // Test various JDK tools
        for tool_name in &["javac", "jar", "jshell", "jps", "jstack", "jmap"] {
            let tool_path = get_tool_path(&jdk, tool_name).unwrap();
            assert!(tool_path.exists());
            let expected_suffix = if cfg!(windows) {
                format!("{tool_name}.exe")
            } else {
                tool_name.to_string()
            };
            assert!(tool_path.ends_with(&expected_suffix));
        }
    }

    #[test]
    fn test_format_source() {
        assert_eq!(
            format_source(&VersionSource::Environment("temurin@21".to_string())),
            "environment"
        );

        let path = PathBuf::from("/project/.kopi-version");
        assert_eq!(
            format_source(&VersionSource::ProjectFile(path.clone())),
            format!("project file: {}", path.display())
        );

        let path = PathBuf::from("/home/user/.kopi/version");
        assert_eq!(
            format_source(&VersionSource::GlobalDefault(path)),
            "global default"
        );
    }

    #[test]
    fn test_which_not_installed() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let command = WhichCommand::new(&config).unwrap();

        let result = command.execute(Some("temurin@22"), "java", false, false);

        match result {
            Err(KopiError::JdkNotInstalled { jdk_spec, .. }) => {
                assert_eq!(jdk_spec, "temurin@22");
            }
            _ => panic!("Expected JdkNotInstalled error"),
        }
    }
}
