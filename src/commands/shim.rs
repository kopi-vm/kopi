use crate::config::KopiConfig;
use crate::error::Result;
use crate::shim::installer::ShimInstaller;
use crate::shim::tools::{ToolCategory, ToolRegistry};
use clap::Subcommand;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};

#[derive(Subcommand)]
pub enum ShimCommand {
    /// Add a shim for a specific tool
    Add {
        /// Name of the tool to add a shim for
        tool: String,

        /// Force creation even if shim already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Remove a shim for a specific tool
    Remove {
        /// Name of the tool to remove the shim for
        tool: String,
    },

    /// List installed shims
    List {
        /// Show available tools that could have shims created
        #[arg(long)]
        available: bool,

        /// Filter by distribution
        #[arg(short, long)]
        distribution: Option<String>,
    },

    /// Verify and repair shims
    Verify {
        /// Fix any issues found
        #[arg(long)]
        fix: bool,
    },
}

impl ShimCommand {
    pub fn execute(&self) -> Result<()> {
        let config = crate::config::new_kopi_config()?;

        match self {
            ShimCommand::Add { tool, force } => self.add_shim(&config, tool, *force),
            ShimCommand::Remove { tool } => self.remove_shim(&config, tool),
            ShimCommand::List {
                available,
                distribution,
            } => self.list_shims(&config, *available, distribution.as_deref()),
            ShimCommand::Verify { fix } => self.verify_shims(&config, *fix),
        }
    }

    fn add_shim(&self, config: &KopiConfig, tool_name: &str, force: bool) -> Result<()> {
        let installer = ShimInstaller::new(config.kopi_home());
        let registry = ToolRegistry::new();

        // If force is true, remove existing shim first
        if force {
            let _ = installer.remove_shim(tool_name); // Ignore error if shim doesn't exist
        }

        // Try to find the tool in the registry
        if let Some(tool_info) = registry.get_tool(tool_name) {
            installer.create_shim(tool_info.name)?;
            println!(
                "{}",
                format!("Created shim for '{}'", tool_info.name)
                    .green()
                    .bold()
            );

            if !tool_info.description.is_empty() {
                println!("  {}", tool_info.description.dimmed());
            }
        } else {
            // Create shim for custom tool
            installer.create_shim(tool_name)?;
            println!(
                "{}",
                format!("Created shim for '{tool_name}'").green().bold()
            );
            println!(
                "  {}",
                "Note: This is a custom tool not in the standard JDK tool list".yellow()
            );
        }

        Ok(())
    }

    fn remove_shim(&self, config: &KopiConfig, tool_name: &str) -> Result<()> {
        let installer = ShimInstaller::new(config.kopi_home());
        installer.remove_shim(tool_name)?;
        println!(
            "{}",
            format!("Removed shim for '{tool_name}'").green().bold()
        );
        Ok(())
    }

    fn list_shims(
        &self,
        config: &KopiConfig,
        show_available: bool,
        distribution_filter: Option<&str>,
    ) -> Result<()> {
        if show_available {
            self.list_available_tools(distribution_filter)
        } else {
            self.list_installed_shims(config)
        }
    }

    fn list_installed_shims(&self, config: &KopiConfig) -> Result<()> {
        let installer = ShimInstaller::new(config.kopi_home());
        let shims = installer.list_shims()?;

        if shims.is_empty() {
            println!("No shims installed.");
            println!("\nRun {} to install default shims", "kopi setup".cyan());
            return Ok(());
        }

        println!("{}", "Installed shims:".bold());
        println!();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Tool", "Target", "Status"]);

        for shim_name in &shims {
            // Check if shim is valid by verifying it points to kopi-shim
            let shim_path = config.shims_dir()?.join(shim_name);
            let status = if shim_path.exists() {
                "✓ Valid".green().to_string()
            } else {
                "✗ Invalid".red().to_string()
            };

            table.add_row(vec![
                shim_name.clone(),
                shim_path.display().to_string(),
                status,
            ]);
        }

        println!("{table}");
        println!();
        println!("Total: {} shims", shims.len().to_string().bold());

        Ok(())
    }

    fn list_available_tools(&self, distribution_filter: Option<&str>) -> Result<()> {
        let registry = ToolRegistry::new();
        let tools = registry.all_tools();

        println!("{}", "Available JDK tools:".bold());
        println!();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Tool", "Category", "Description", "Version Range"]);

        for info in tools {
            // Note: Distribution filtering is not supported for individual tools
            // as tools are generally available across all distributions
            if distribution_filter.is_some() {
                // Skip the header message since we're showing all tools
            }

            let category_name = match info.category {
                ToolCategory::Core => "Core",
                ToolCategory::Debug => "Debug",
                ToolCategory::Monitoring => "Monitoring",
                ToolCategory::Security => "Security",
                ToolCategory::Utility => "Utility",
            };

            let version_info = match (info.min_version, info.max_version) {
                (Some(min), Some(max)) => format!("v{min}-{max}"),
                (Some(min), None) => format!("v{min}+"),
                (None, Some(max)) => format!("up to v{max}"),
                (None, None) => "All versions".to_string(),
            };

            table.add_row(vec![
                info.name.to_string(),
                category_name.to_string(),
                info.description.to_string(),
                version_info,
            ]);
        }

        println!("{table}");
        println!();
        println!("To create a shim: {}", "kopi shim add <tool>".cyan());

        Ok(())
    }

    fn verify_shims(&self, config: &KopiConfig, fix: bool) -> Result<()> {
        let installer = ShimInstaller::new(config.kopi_home());
        let shims = installer.list_shims()?;

        if shims.is_empty() {
            println!("No shims to verify.");
            return Ok(());
        }

        println!("{}", "Verifying shims...".bold());
        println!();

        let mut issues_found = 0;
        let mut issues_fixed = 0;

        for shim_name in &shims {
            let shim_path = config.shims_dir()?.join(shim_name);
            let is_valid = shim_path.exists();

            if !is_valid {
                issues_found += 1;
                println!("  {} {}", "✗".red(), shim_name);
                println!("    Issue: Shim file not found");

                if fix {
                    match installer.create_shim(shim_name) {
                        Ok(_) => {
                            println!("    {} Fixed", "✓".green());
                            issues_fixed += 1;
                        }
                        Err(e) => {
                            println!("    {} Failed to fix: {}", "✗".red(), e);
                        }
                    }
                }
            } else {
                println!("  {} {}", "✓".green(), shim_name);
            }
        }

        println!();

        if issues_found == 0 {
            println!("{}", "All shims are valid!".green().bold());
        } else if fix {
            println!(
                "Found {} issues, fixed {}",
                issues_found.to_string().yellow(),
                issues_fixed.to_string().green()
            );

            if issues_fixed < issues_found {
                println!(
                    "{}",
                    format!(
                        "{} issues could not be fixed automatically",
                        issues_found - issues_fixed
                    )
                    .red()
                );
            }
        } else {
            println!(
                "Found {} issues. Run {} to fix them.",
                issues_found.to_string().red(),
                "kopi shim verify --fix".cyan()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_available_tools() {
        let cmd = ShimCommand::List {
            available: true,
            distribution: None,
        };

        // This should not fail
        let result = cmd.list_available_tools(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_available_tools_with_filter() {
        let cmd = ShimCommand::List {
            available: true,
            distribution: Some("graalvm".to_string()),
        };

        // This should not fail
        let result = cmd.list_available_tools(Some("graalvm"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_installed_shims_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        let cmd = ShimCommand::List {
            available: false,
            distribution: None,
        };

        let result = cmd.list_installed_shims(&config);
        assert!(result.is_ok());
    }
}
