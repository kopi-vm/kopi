// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::storage::JdkRepository;
use crate::version::resolver::{VersionResolver, VersionSource};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct CurrentOutput {
    version: Option<String>,
    source: String,
    source_path: Option<String>,
    installed: bool,
    installation_path: Option<String>,
    distribution: Option<String>,
}

pub struct CurrentCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> CurrentCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, quiet: bool, json: bool) -> Result<()> {
        // Create version resolver
        let resolver = VersionResolver::new(self.config);

        // Resolve version with source tracking
        let (version_request, source) = match resolver.resolve_version() {
            Ok(result) => result,
            Err(KopiError::NoLocalVersion { searched_paths }) => {
                if json {
                    let output = serde_json::json!({
                        "error": "no_version_configured",
                        "message": "No JDK version configured",
                        "searched_paths": searched_paths,
                        "hints": [
                            "Use 'kopi local <version>' to set a project version",
                            "Use 'kopi global <version>' to set a default"
                        ]
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else if quiet {
                    // In quiet mode, output nothing on error
                    return Err(KopiError::NoLocalVersion { searched_paths });
                } else {
                    eprintln!("No JDK version configured");
                    eprintln!("Hint: Use 'kopi local <version>' to set a project version");
                    eprintln!("      or 'kopi global <version>' to set a default");
                }
                return Err(KopiError::NoLocalVersion { searched_paths });
            }
            Err(e) => return Err(e),
        };

        // Check if the version is actually installed
        let repository = JdkRepository::new(self.config);

        // Try to find matching installations using find_matching_jdks
        let mut install_path = None;
        let mut is_installed = false;

        // Get matching JDKs and use the last one from the results
        if let Ok(matching_jdks) = repository.find_matching_jdks(&version_request)
            && let Some(jdk) = matching_jdks.last()
        {
            install_path = Some(jdk.path.clone());
            is_installed = true;
        }

        // Format and display output
        if json {
            print_json_output(&version_request, &source, is_installed, &install_path)?;
        } else if quiet {
            println!("{}", version_request.version_pattern);
        } else {
            print_standard_output(&version_request, &source, is_installed)?;
        }

        Ok(())
    }
}

fn print_json_output(
    version_request: &crate::version::VersionRequest,
    source: &VersionSource,
    is_installed: bool,
    install_path: &Option<PathBuf>,
) -> Result<()> {
    let (source_name, source_path) = match source {
        VersionSource::Environment(value) => ("KOPI_JAVA_VERSION".to_string(), Some(value.clone())),
        VersionSource::ProjectFile(path) => {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "version file".to_string());
            (file_name, Some(path.display().to_string()))
        }
        VersionSource::GlobalDefault(path) => (
            "global default".to_string(),
            Some(path.display().to_string()),
        ),
    };

    let output = CurrentOutput {
        version: Some(version_request.version_pattern.clone()),
        source: source_name,
        source_path,
        installed: is_installed,
        installation_path: install_path.as_ref().map(|p| p.display().to_string()),
        distribution: version_request.distribution.clone(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_standard_output(
    version_request: &crate::version::VersionRequest,
    source: &VersionSource,
    is_installed: bool,
) -> Result<()> {
    let source_display = match source {
        VersionSource::Environment(_) => "set by KOPI_JAVA_VERSION".to_string(),
        VersionSource::ProjectFile(path) => {
            // Try to make the path relative to current directory for better readability
            let display_path = if let Ok(current_dir) = std::env::current_dir() {
                path.strip_prefix(&current_dir)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string())
            } else {
                path.display().to_string()
            };
            format!("set by {display_path}")
        }
        VersionSource::GlobalDefault(_) => "set by global default".to_string(),
    };

    let version_display = if let Some(dist) = &version_request.distribution {
        format!("{dist}@{}", version_request.version_pattern)
    } else {
        version_request.version_pattern.clone()
    };

    if is_installed {
        println!("{version_display} ({source_display})");
    } else {
        println!("{version_display} ({source_display}) [NOT INSTALLED]");
        eprintln!("Warning: JDK version {version_display} is configured but not installed");
        eprintln!("Hint: Run 'kopi install {version_display}' to install this version");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_current_with_env_var() {
        // Save and set environment variable
        let original = env::var("KOPI_JAVA_VERSION").ok();
        unsafe {
            env::set_var("KOPI_JAVA_VERSION", "21");
        }

        let temp_dir = TempDir::new().unwrap();
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let _command = CurrentCommand::new(&config).unwrap();
        // This would need mocking to properly test without side effects

        // Restore environment
        unsafe {
            if let Some(val) = original {
                env::set_var("KOPI_JAVA_VERSION", val);
            } else {
                env::remove_var("KOPI_JAVA_VERSION");
            }
        }
    }

    #[test]
    fn test_current_with_project_file() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join(".kopi-version");
        fs::write(&version_file, "temurin@17").unwrap();

        // Would need to mock the current directory and resolver to test properly
    }
}
