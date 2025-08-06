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
use crate::platform::file_ops::make_executable;
use crate::platform::shell::{Shell, detect_shell};
use crate::platform::shim_binary_name;
use crate::shim::installer::ShimInstaller;
use crate::shim::tools::default_shim_tools;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct SetupCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> SetupCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, force: bool) -> Result<()> {
        println!("{}", "Setting up Kopi...".bold());

        // Step 1: Create directories
        self.create_directories()?;

        // Step 2: Build kopi-shim binary
        self.build_shim_binary()?;

        // Step 3: Install default shims
        self.install_default_shims(force)?;

        // Step 4: Generate PATH update instructions
        self.show_path_instructions()?;

        println!("\n{}", "Setup completed successfully!".green().bold());
        Ok(())
    }

    fn create_directories(&self) -> Result<()> {
        println!("Creating Kopi directories...");

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
                println!("  Created: {}", dir.display());
            } else {
                println!("  Exists: {}", dir.display());
            }
        }

        Ok(())
    }

    fn build_shim_binary(&self) -> Result<()> {
        println!("\nBuilding kopi-shim binary...");

        // Check if we're running from a development environment
        let current_exe = env::current_exe()?;
        let is_development = current_exe
            .to_str()
            .map(|p| p.contains("target/debug") || p.contains("target/release"))
            .unwrap_or(false);

        if is_development {
            // We're in development, build the shim binary
            println!("  Building from source...");

            let output = Command::new("cargo")
                .args(["build", "--bin", "kopi-shim", "--release"])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::error::KopiError::SystemError(format!(
                    "Failed to build kopi-shim: {stderr}"
                )));
            }

            // Copy the built binary to the bin directory
            let source = PathBuf::from("target/release/kopi-shim");
            let dest = self.config.bin_dir()?.join("kopi-shim");

            if source.exists() {
                fs::copy(&source, &dest)?;
                println!("  Installed kopi-shim to: {}", dest.display());

                // Make it executable
                make_executable(&dest)?
            } else {
                return Err(crate::error::KopiError::SystemError(
                    "Failed to find built kopi-shim binary".to_string(),
                ));
            }
        } else {
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

            if source.exists() {
                let dest = self.config.bin_dir()?.join(shim_name);
                fs::copy(&source, &dest)?;
                println!("  Installed kopi-shim to: {}", dest.display());

                // Make it executable
                make_executable(&dest)?
            } else {
                return Err(crate::error::KopiError::SystemError(format!(
                    "kopi-shim binary not found at: {}",
                    source.display()
                )));
            }
        }

        Ok(())
    }

    fn install_default_shims(&self, force: bool) -> Result<()> {
        println!("\nInstalling default shims...");

        let installer = ShimInstaller::new(self.config.kopi_home());

        // Get core tools that should be installed by default
        let core_tools = default_shim_tools();

        for tool_name in core_tools {
            match installer.create_shim(tool_name) {
                Ok(_) => println!("  ✓ {tool_name}"),
                Err(e) => {
                    if !force {
                        println!("  ⚠ {tool_name} ({e})");
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    fn show_path_instructions(&self) -> Result<()> {
        println!("\n{}", "PATH Configuration Required".yellow().bold());
        println!("{}", "─".repeat(50));

        let shims_dir = self.config.shims_dir()?;
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
        let setup = SetupCommand { config: &config };

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
        let setup = SetupCommand { config: &config };

        // This should not fail even if shell detection fails
        let result = setup.show_path_instructions();
        assert!(result.is_ok());
    }
}
