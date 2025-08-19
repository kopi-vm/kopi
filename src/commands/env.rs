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
use crate::platform::shell::{Shell, detect_shell, parse_shell_name};
use crate::storage::JdkRepository;
use crate::version::VersionRequest;
use crate::version::resolver::{VersionResolver, VersionSource};
use std::io::Write;
use std::path::Path;

pub struct EnvCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> EnvCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, version: Option<&str>, shell: Option<&str>, export: bool) -> Result<()> {
        // Resolve version
        let (version_request, _source) = if let Some(ver) = version {
            // Version explicitly provided
            let request = ver.parse::<VersionRequest>()?;
            (request, VersionSource::Environment(ver.to_string()))
        } else {
            // Use version resolver
            let resolver = VersionResolver::new(self.config);
            resolver.resolve_version()?
        };

        // Verify JDK is installed
        let repository = JdkRepository::new(self.config);
        let matching_jdks = repository.find_matching_jdks(&version_request)?;

        let jdk = matching_jdks.last().ok_or_else(|| {
            let version_display = if let Some(dist) = &version_request.distribution {
                format!("{dist}@{}", version_request.version_pattern)
            } else {
                version_request.version_pattern.clone()
            };
            KopiError::JdkNotInstalled {
                jdk_spec: version_display,
                version: Some(version_request.version_pattern.clone()),
                distribution: version_request.distribution.clone(),
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            }
        })?;

        // Detect or parse shell
        let shell_type = if let Some(shell_name) = shell {
            parse_shell_name(shell_name)?
        } else {
            let (shell, _path) = detect_shell()?;
            shell
        };

        // Format environment variables
        let formatter = EnvFormatter::new(shell_type, export);
        let java_home = jdk.resolve_java_home();
        let output = formatter.format_env(&java_home)?;

        // Output to stdout
        let mut stdout = std::io::stdout();
        stdout.write_all(output.as_bytes())?;
        stdout.flush()?;

        Ok(())
    }
}

struct EnvFormatter {
    shell_type: Shell,
    export: bool,
}

impl EnvFormatter {
    fn new(shell_type: Shell, export: bool) -> Self {
        Self { shell_type, export }
    }

    fn format_env(&self, jdk_path: &Path) -> Result<String> {
        let java_home = jdk_path.to_string_lossy();

        match self.shell_type {
            Shell::Bash | Shell::Zsh => {
                // Escape double quotes and backslashes in the path
                let escaped_path = java_home.replace('\\', "\\\\").replace('"', "\\\"");
                if self.export {
                    Ok(format!("export JAVA_HOME=\"{escaped_path}\"\n"))
                } else {
                    Ok(format!("JAVA_HOME=\"{escaped_path}\"\n"))
                }
            }
            Shell::Fish => {
                // Fish also needs quote escaping
                let escaped_path = java_home.replace('\\', "\\\\").replace('"', "\\\"");
                if self.export {
                    Ok(format!("set -gx JAVA_HOME \"{escaped_path}\"\n"))
                } else {
                    Ok(format!("set -g JAVA_HOME \"{escaped_path}\"\n"))
                }
            }
            Shell::PowerShell => {
                // PowerShell uses backtick for escaping
                let escaped_path = java_home.replace('"', "`\"");
                Ok(format!("$env:JAVA_HOME = \"{escaped_path}\"\n"))
            }
            Shell::Cmd => {
                // CMD is more complex - spaces and special chars need quotes
                if java_home.contains(' ')
                    || java_home.contains('&')
                    || java_home.contains('(')
                    || java_home.contains(')')
                {
                    // Use quotes and escape internal quotes
                    let escaped_path = java_home.replace('"', "\"\"");
                    Ok(format!("set JAVA_HOME=\"{escaped_path}\"\n"))
                } else {
                    Ok(format!("set JAVA_HOME={java_home}\n"))
                }
            }
            Shell::Unknown(_) => {
                // Default to bash-style export with escaping
                let escaped_path = java_home.replace('\\', "\\\\").replace('"', "\\\"");
                if self.export {
                    Ok(format!("export JAVA_HOME=\"{escaped_path}\"\n"))
                } else {
                    Ok(format!("JAVA_HOME=\"{escaped_path}\"\n"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InstalledJdk;
    use crate::version::Version;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[test]
    fn test_bash_formatter() {
        let formatter = EnvFormatter::new(Shell::Bash, true);
        let path = PathBuf::from("/home/user/.kopi/jdks/temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "export JAVA_HOME=\"/home/user/.kopi/jdks/temurin-21\"\n"
        );
    }

    #[test]
    fn test_bash_formatter_no_export() {
        let formatter = EnvFormatter::new(Shell::Bash, false);
        let path = PathBuf::from("/home/user/.kopi/jdks/temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(output, "JAVA_HOME=\"/home/user/.kopi/jdks/temurin-21\"\n");
    }

    #[test]
    fn test_fish_formatter() {
        let formatter = EnvFormatter::new(Shell::Fish, true);
        let path = PathBuf::from("/home/user/.kopi/jdks/temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "set -gx JAVA_HOME \"/home/user/.kopi/jdks/temurin-21\"\n"
        );
    }

    #[test]
    fn test_fish_formatter_no_export() {
        let formatter = EnvFormatter::new(Shell::Fish, false);
        let path = PathBuf::from("/home/user/.kopi/jdks/temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "set -g JAVA_HOME \"/home/user/.kopi/jdks/temurin-21\"\n"
        );
    }

    #[test]
    fn test_powershell_formatter() {
        let formatter = EnvFormatter::new(Shell::PowerShell, true);
        let path = PathBuf::from("C:\\Users\\user\\.kopi\\jdks\\temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "$env:JAVA_HOME = \"C:\\Users\\user\\.kopi\\jdks\\temurin-21\"\n"
        );
    }

    #[test]
    fn test_cmd_formatter() {
        let formatter = EnvFormatter::new(Shell::Cmd, true);
        let path = PathBuf::from("C:\\Users\\user\\.kopi\\jdks\\temurin-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "set JAVA_HOME=C:\\Users\\user\\.kopi\\jdks\\temurin-21\n"
        );
    }

    #[test]
    fn test_bash_formatter_with_quotes() {
        let formatter = EnvFormatter::new(Shell::Bash, true);
        let path = PathBuf::from("/home/user/\"special\"/jdk");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "export JAVA_HOME=\"/home/user/\\\"special\\\"/jdk\"\n"
        );
    }

    #[test]
    fn test_powershell_formatter_with_quotes() {
        let formatter = EnvFormatter::new(Shell::PowerShell, true);
        let path = PathBuf::from("C:\\Program Files\\Java\\\"JDK\"\\bin");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "$env:JAVA_HOME = \"C:\\Program Files\\Java\\`\"JDK`\"\\bin\"\n"
        );
    }

    #[test]
    fn test_cmd_formatter_with_spaces() {
        let formatter = EnvFormatter::new(Shell::Cmd, true);
        let path = PathBuf::from("C:\\Program Files\\Java\\jdk-21");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "set JAVA_HOME=\"C:\\Program Files\\Java\\jdk-21\"\n"
        );
    }

    #[test]
    fn test_cmd_formatter_with_special_chars() {
        let formatter = EnvFormatter::new(Shell::Cmd, true);
        let path = PathBuf::from("C:\\Dev\\Java (x64)\\jdk");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(output, "set JAVA_HOME=\"C:\\Dev\\Java (x64)\\jdk\"\n");
    }

    #[test]
    fn test_fish_formatter_with_escaping() {
        let formatter = EnvFormatter::new(Shell::Fish, true);
        let path = PathBuf::from("/home/user/\"kopi\"/jdk");
        let output = formatter.format_env(&path).unwrap();
        assert_eq!(
            output,
            "set -gx JAVA_HOME \"/home/user/\\\"kopi\\\"/jdk\"\n"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_java_home_bundle_structure() {
        // Create a mock JDK with bundle structure
        let temp_dir = TempDir::new().unwrap();
        let jdk_root = temp_dir.path().join("temurin-21");
        let bundle_home = jdk_root.join("Contents").join("Home");
        let bundle_bin = bundle_home.join("bin");

        // Create the directory structure
        std::fs::create_dir_all(&bundle_bin).unwrap();

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.0").unwrap(),
            jdk_root.clone(),
            false,
        );

        // Test that resolve_java_home returns the Contents/Home path
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, bundle_home);

        // Test formatting for different shells
        let formatter = EnvFormatter::new(Shell::Bash, true);
        let output = formatter.format_env(&java_home).unwrap();
        assert!(output.contains(&bundle_home.to_string_lossy().to_string()));
        assert!(output.contains("Contents/Home"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_java_home_direct_structure() {
        // Create a mock JDK with direct structure
        let temp_dir = TempDir::new().unwrap();
        let jdk_root = temp_dir.path().join("liberica-21");
        let direct_bin = jdk_root.join("bin");

        // Create the directory structure
        std::fs::create_dir_all(&direct_bin).unwrap();

        let jdk = InstalledJdk::new(
            "liberica".to_string(),
            Version::from_str("21.0.0").unwrap(),
            jdk_root.clone(),
            false,
        );

        // Test that resolve_java_home returns the root path
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_root);

        // Test formatting for different shells
        let formatter = EnvFormatter::new(Shell::Bash, true);
        let output = formatter.format_env(&java_home).unwrap();
        assert!(output.contains(&jdk_root.to_string_lossy().to_string()));
        assert!(!output.contains("Contents/Home"));
    }

    #[test]
    fn test_env_output_includes_correct_java_home() {
        // Test bundle structure
        let temp_dir = TempDir::new().unwrap();
        let jdk_root = temp_dir.path().join("test-jdk");

        #[cfg(target_os = "macos")]
        {
            // Create bundle structure on macOS
            let bundle_home = jdk_root.join("Contents").join("Home");
            let bundle_bin = bundle_home.join("bin");
            std::fs::create_dir_all(&bundle_bin).unwrap();
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Create direct structure on other platforms
            let direct_bin = jdk_root.join("bin");
            std::fs::create_dir_all(&direct_bin).unwrap();
        }

        let jdk = InstalledJdk::new(
            "test".to_string(),
            Version::from_str("21.0.0").unwrap(),
            jdk_root.clone(),
            false,
        );

        let java_home = jdk.resolve_java_home();

        // Test bash output
        let formatter = EnvFormatter::new(Shell::Bash, true);
        let output = formatter.format_env(&java_home).unwrap();
        assert!(output.starts_with("export JAVA_HOME="));

        // Test zsh output
        let formatter = EnvFormatter::new(Shell::Zsh, false);
        let output = formatter.format_env(&java_home).unwrap();
        assert!(output.starts_with("JAVA_HOME="));

        // Test fish output
        let formatter = EnvFormatter::new(Shell::Fish, true);
        let output = formatter.format_env(&java_home).unwrap();
        assert!(output.starts_with("set -gx JAVA_HOME"));
    }

    #[test]
    fn test_path_includes_correct_bin_directory() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_root = temp_dir.path().join("test-jdk");

        #[cfg(target_os = "macos")]
        {
            // Create bundle structure
            let bundle_home = jdk_root.join("Contents").join("Home");
            let bundle_bin = bundle_home.join("bin");
            std::fs::create_dir_all(&bundle_bin).unwrap();
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Create direct structure
            let direct_bin = jdk_root.join("bin");
            std::fs::create_dir_all(&direct_bin).unwrap();
        }

        let jdk = InstalledJdk::new(
            "test".to_string(),
            Version::from_str("21.0.0").unwrap(),
            jdk_root.clone(),
            false,
        );

        // Verify bin path resolution
        let bin_path = jdk.resolve_bin_path().unwrap();
        assert!(bin_path.exists());
        assert!(bin_path.ends_with("bin"));

        #[cfg(target_os = "macos")]
        {
            // On macOS with bundle structure, bin should be under Contents/Home
            if jdk_root.join("Contents").join("Home").join("bin").exists() {
                assert!(bin_path.to_string_lossy().contains("Contents/Home/bin"));
            }
        }
    }

    #[test]
    fn test_shell_output_formats() {
        let jdk = InstalledJdk::new(
            "test".to_string(),
            Version::from_str("21.0.0").unwrap(),
            PathBuf::from("/test/jdk"),
            false,
        );

        let java_home = jdk.resolve_java_home();

        // Test all shell formats
        let shells = vec![
            (Shell::Bash, true, "export JAVA_HOME="),
            (Shell::Bash, false, "JAVA_HOME="),
            (Shell::Zsh, true, "export JAVA_HOME="),
            (Shell::Fish, true, "set -gx JAVA_HOME"),
            (Shell::Fish, false, "set -g JAVA_HOME"),
            (Shell::PowerShell, true, "$env:JAVA_HOME ="),
            (Shell::Cmd, true, "set JAVA_HOME="),
        ];

        for (shell, export, expected_prefix) in shells {
            let formatter = EnvFormatter::new(shell.clone(), export);
            let output = formatter.format_env(&java_home).unwrap();
            assert!(
                output.starts_with(expected_prefix),
                "Shell {shell:?} with export={export} should start with '{expected_prefix}', but got '{output}'"
            );
        }
    }

    #[test]
    fn test_error_handling_missing_bin_directory() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_root = temp_dir.path().join("broken-jdk");

        // Create JDK root but no bin directory
        std::fs::create_dir_all(&jdk_root).unwrap();

        let jdk = InstalledJdk::new(
            "broken".to_string(),
            Version::from_str("21.0.0").unwrap(),
            jdk_root.clone(),
            false,
        );

        // This should return an error since bin directory is missing
        let result = jdk.resolve_bin_path();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("bin directory not found")
        );
    }
}
