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

    pub fn execute(
        &self,
        version: Option<&str>,
        shell: Option<&str>,
        export: bool,
        quiet: bool,
    ) -> Result<()> {
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
        let output = formatter.format_env(&jdk.path)?;

        // Output to stdout
        let mut stdout = std::io::stdout();
        stdout.write_all(output.as_bytes())?;
        stdout.flush()?;

        // Show helpful message on stderr unless quiet
        if !quiet {
            eprintln!("# Run this command to configure your shell:");
            eprintln!("# eval \"$(kopi env)\"");
        }

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
                Ok(format!("set -gx JAVA_HOME \"{escaped_path}\"\n"))
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
    use std::path::PathBuf;

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
}
