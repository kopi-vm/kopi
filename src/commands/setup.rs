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
use crate::error::Result;
use crate::indicator::StatusReporter;
use crate::platform::file_ops::make_executable;
use crate::platform::shell::{Shell, detect_shell};
use crate::platform::shim_binary_name;
use crate::shim::installer::ShimInstaller;
use crate::shim::tools::default_shim_tools;
use colored::Colorize;
use std::env;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
#[cfg(debug_assertions)]
use std::process::Command;

pub struct SetupCommand<'a> {
    config: &'a KopiConfig,
    status: StatusReporter,
}

impl<'a> SetupCommand<'a> {
    pub fn new(config: &'a KopiConfig, no_progress: bool) -> Result<Self> {
        Ok(Self {
            config,
            status: StatusReporter::new(no_progress),
        })
    }

    pub fn execute(&self, force: bool) -> Result<()> {
        self.status.operation("Setting up", "Kopi");

        // Step 1: Create directories
        self.create_directories()?;

        // Step 2: Build kopi-shim binary
        self.build_shim_binary()?;

        // Step 3: Install default shims
        self.install_default_shims(force)?;

        // Step 4: Generate PATH update instructions
        self.show_path_instructions()?;

        self.status.success("Setup completed successfully!");
        Ok(())
    }

    fn create_directories(&self) -> Result<()> {
        self.status.step("Creating Kopi directories");

        let jdks_dir = self.config.jdks_dir()?;
        let bin_dir = self.config.bin_dir()?;
        let shims_dir = self.config.shims_dir()?;
        let cache_dir = self.config.cache_dir()?;

        let dirs = vec![
            self.config.kopi_home(),
            &jdks_dir,
            &bin_dir,
            &shims_dir,
            &cache_dir,
        ];

        for dir in dirs {
            if !dir.exists() {
                fs::create_dir_all(dir)?;
                self.status.step(&format!("Created: {}", dir.display()));
            } else {
                self.status.step(&format!("Exists: {}", dir.display()));
            }
        }

        Ok(())
    }

    fn build_shim_binary(&self) -> Result<()> {
        self.status.step("Building kopi-shim binary");

        let current_exe = env::current_exe()?;
        log::debug!("Current executable: {}", current_exe.display());

        #[cfg(debug_assertions)]
        {
            // Check if we're running from a development environment
            let is_development = current_exe
                .to_str()
                .map(|p| p.contains("target/debug") || p.contains("target/release"))
                .unwrap_or(false);

            if is_development {
                // We're in development, build the shim binary
                log::info!("Building from source...");

                let output = Command::new("cargo")
                    .args(["build", "--bin", "kopi-shim", "--release"])
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(crate::error::KopiError::SystemError(format!(
                        "Failed to build kopi-shim: {stderr}"
                    )));
                }

                let source = PathBuf::from("target/release/kopi-shim");
                self.copy_and_configure_shim(&source, "kopi-shim")?;
                return Ok(());
            }
        }

        // We're running from an installed version
        // The kopi-shim should be installed alongside the main binary
        let shim_name = shim_binary_name();

        let source = current_exe
            .parent()
            .map(|p| p.join(shim_name))
            .ok_or_else(|| {
                crate::error::KopiError::SystemError(
                    "Failed to locate kopi-shim binary".to_string(),
                )
            })?;

        self.copy_and_configure_shim(&source, shim_name)?;

        Ok(())
    }

    fn copy_and_configure_shim(&self, source: &Path, shim_name: &str) -> Result<()> {
        if !source.exists() {
            return Err(crate::error::KopiError::SystemError(format!(
                "kopi-shim binary not found at: {}",
                source.display()
            )));
        }

        let dest = self.config.bin_dir()?.join(shim_name);

        log::debug!("Source path: {}", source.display());
        log::debug!("Destination path: {}", dest.display());

        // Check if source and destination are the same
        if source.canonicalize().ok() == dest.canonicalize().ok() {
            log::debug!("kopi-shim already in place (source and destination are the same)");
        } else {
            log::debug!("Attempting to copy...");

            match fs::copy(source, &dest) {
                Ok(bytes) => {
                    log::debug!("Successfully copied {bytes} bytes");
                    log::info!("Installed kopi-shim to: {}", dest.display());
                }
                Err(e) => {
                    self.status.error(&format!("Failed to copy file: {e}"));
                    log::debug!("  Source: {}", source.display());
                    log::debug!("  Destination: {}", dest.display());

                    // Check if destination file already exists and is in use
                    if dest.exists() {
                        log::debug!("Destination file already exists");

                        // Try to check if file is locked by attempting to open it
                        match OpenOptions::new().write(true).open(&dest) {
                            Ok(_) => log::debug!("File is accessible"),
                            Err(e2) => log::debug!("Cannot access file: {e2}"),
                        }
                    }

                    return Err(crate::error::KopiError::Io(e));
                }
            }
        }

        // Make it executable
        log::debug!("Setting executable permissions...");
        make_executable(&dest)?;
        log::debug!("Executable permissions set successfully");

        Ok(())
    }

    fn install_default_shims(&self, force: bool) -> Result<()> {
        self.status.step("Installing default shims");

        let installer = ShimInstaller::new(self.config.kopi_home());

        // Get core tools that should be installed by default
        let core_tools = default_shim_tools();

        for tool_name in core_tools {
            match installer.create_shim(tool_name) {
                Ok(_) => self.status.step(&format!("✓ {tool_name}")),
                Err(e) => {
                    if !force {
                        self.status.step(&format!("⚠ {tool_name} ({e})"));
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    fn show_path_instructions(&self) -> Result<()> {
        let shims_dir = self.config.shims_dir()?;

        // Check if shims directory is already in PATH
        if let Ok(path_env) = env::var("PATH") {
            let is_in_path = env::split_paths(&path_env).any(|p| {
                // Normalize paths for comparison
                p.canonicalize().ok() == shims_dir.canonicalize().ok()
            });

            if is_in_path {
                println!("\n{}", "PATH is already configured ✓".green().bold());
                println!("The shims directory is already in your PATH:");
                println!("  {}", shims_dir.display().to_string().bold());
                return Ok(());
            }
        }

        // If not in PATH, show configuration instructions
        println!("\n{}", "PATH Configuration Required".yellow().bold());
        println!("{}", "─".repeat(50));

        let (shell, _shell_path) = detect_shell().unwrap_or_else(|_| {
            // Fallback to a default shell if detection fails
            #[cfg(unix)]
            let fallback = Shell::Bash;
            #[cfg(windows)]
            let fallback = Shell::PowerShell;
            (fallback, PathBuf::from(""))
        });

        println!("Add the following directory to your PATH:");
        println!("  {}", shims_dir.display().to_string().bold());

        println!("\n{}", format!("For {shell:?} shell:").italic());
        match shell {
            Shell::Bash => {
                println!("\nAdd to ~/.bashrc:");
                println!(
                    "  {}",
                    format!("export PATH=\"{}:$PATH\"", shims_dir.display()).cyan()
                );
                println!("\nThen reload your shell:");
                println!("  {}", "source ~/.bashrc".cyan());
            }
            Shell::Zsh => {
                println!("\nAdd to ~/.zshrc:");
                println!(
                    "  {}",
                    format!("export PATH=\"{}:$PATH\"", shims_dir.display()).cyan()
                );
                println!("\nThen reload your shell:");
                println!("  {}", "source ~/.zshrc".cyan());
            }
            Shell::Fish => {
                println!("\nAdd to ~/.config/fish/config.fish:");
                println!(
                    "  {}",
                    format!("set -gx PATH {} $PATH", shims_dir.display()).cyan()
                );
                println!("\nThen reload your shell:");
                println!("  {}", "source ~/.config/fish/config.fish".cyan());
            }
            Shell::PowerShell => {
                println!("\nAdd to your PowerShell profile:");
                println!(
                    "  {}",
                    format!(
                        "$env:Path = \"{}\" + \";\" + $env:Path",
                        shims_dir.display()
                    )
                    .cyan()
                );
                println!("\nTo find your profile location:");
                println!("  {}", "$PROFILE".cyan());
                println!("\nThen reload your profile:");
                println!("  {}", ". $PROFILE".cyan());
            }
            Shell::Cmd => {
                println!("\nFor permanent configuration, use System Properties:");
                println!("  1. Open System Properties → Advanced → Environment Variables");
                println!("  2. Edit the 'Path' variable");
                println!("  3. Add: {}", shims_dir.display().to_string().cyan());
                println!("\nFor temporary use in current session:");
                println!(
                    "  {}",
                    format!("set PATH={};%PATH%", shims_dir.display()).cyan()
                );
            }
            Shell::Unknown(_) => {
                println!("\nAdd this directory to your shell's PATH configuration:");
                println!("  {}", shims_dir.display().to_string().cyan());
                println!("\nConsult your shell's documentation for the exact syntax.");
            }
        }

        println!("\n{}", "After updating PATH:".italic());
        println!("  1. Restart your terminal or reload your shell configuration");
        println!("  2. Verify with: {}", "kopi current".cyan());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let setup = SetupCommand {
            config: &config,
            status: StatusReporter::new(true), // Use silent mode for tests
        };

        setup.create_directories().unwrap();

        assert!(temp_dir.path().join("jdks").exists());
        assert!(temp_dir.path().join("bin").exists());
        assert!(temp_dir.path().join("shims").exists());
        assert!(temp_dir.path().join("cache").exists());
    }

    #[test]
    fn test_show_path_instructions() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let setup = SetupCommand {
            config: &config,
            status: StatusReporter::new(true), // Use silent mode for tests
        };

        // This should not fail even if shell detection fails
        let result = setup.show_path_instructions();
        assert!(result.is_ok());
    }
}
