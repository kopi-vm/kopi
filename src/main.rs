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

use clap::{Parser, Subcommand};
use kopi::commands::cache::CacheCommand;
use kopi::commands::current::CurrentCommand;
use kopi::commands::doctor::DoctorCommand;
use kopi::commands::env::EnvCommand;
use kopi::commands::global::GlobalCommand;
use kopi::commands::install::InstallCommand;
use kopi::commands::list::ListCommand;
use kopi::commands::local::LocalCommand;
use kopi::commands::setup::SetupCommand;
use kopi::commands::shell::ShellCommand;
use kopi::commands::shim::ShimCommand;
use kopi::commands::uninstall::UninstallCommand;
use kopi::commands::which::WhichCommand;
use kopi::config::new_kopi_config;
use kopi::error::{Result, format_error_chain, get_exit_code};
use kopi::logging;

#[derive(Parser)]
#[command(name = "kopi")]
#[command(author, version, about = "JDK version management tool", long_about = None)]
struct Cli {
    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a JDK version
    #[command(visible_alias = "i")]
    Install {
        /// Version to install (e.g., "21", "17.0.9", "corretto@21")
        version: String,

        /// Force reinstall even if already installed
        #[arg(short, long)]
        force: bool,

        /// Show what would be installed without actually installing
        #[arg(long)]
        dry_run: bool,

        /// Disable progress indicators
        #[arg(long)]
        no_progress: bool,

        /// Download timeout in seconds
        #[arg(long, value_name = "SECONDS")]
        timeout: Option<u64>,
    },

    /// List installed JDK versions
    #[command(visible_alias = "ls")]
    List,

    /// Set JDK version for current shell session
    #[command(visible_alias = "use")]
    Shell {
        /// JDK version to use
        version: String,
        /// Override shell detection
        #[arg(long)]
        shell: Option<String>,
    },

    /// Show currently active JDK version
    Current {
        /// Show only version number
        #[arg(short = 'q', long)]
        quiet: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Output environment variables for shell evaluation
    ///
    /// Sets JAVA_HOME for the current JDK. Use with eval/source in your shell.
    #[command(long_about = "Output environment variables for shell evaluation

Sets JAVA_HOME for the current or specified JDK version.

Examples:
  eval \"$(kopi env)\"              # Bash/Zsh
  kopi env | source               # Fish
  kopi env | Invoke-Expression    # PowerShell")]
    Env {
        /// Specific version to use (defaults to current)
        version: Option<String>,
        /// Override shell detection
        #[arg(long)]
        shell: Option<String>,
        /// Output export statements (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        export: bool,
    },

    /// Set the global default JDK version
    #[command(visible_alias = "g", alias = "default")]
    Global {
        /// Version to set as global default
        version: String,
    },

    /// Set the local project JDK version
    #[command(visible_alias = "l", alias = "pin")]
    Local {
        /// Version to set for current project
        version: String,
    },

    /// Show installation path for a JDK version
    #[command(visible_alias = "w")]
    Which {
        /// JDK version specification (optional)
        version: Option<String>,

        /// Show path for specific JDK tool
        #[arg(long, default_value = "java")]
        tool: String,

        /// Show JDK home directory instead of executable path
        #[arg(long)]
        home: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Manage JDK metadata cache
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },

    /// Refresh JDK metadata cache (alias for cache refresh)
    #[command(visible_alias = "r", hide = true)]
    Refresh,

    /// Search available JDK versions (alias for cache search)
    #[command(visible_alias = "s", aliases = ["ls-remote", "list-remote"], hide = true)]
    Search {
        /// Version pattern to search (e.g., "21", "corretto", "corretto@17")
        #[arg(value_name = "VERSION")]
        version: Option<String>,

        /// Show compact output (version numbers only)
        #[arg(short, long, conflicts_with = "detailed")]
        compact: bool,

        /// Show detailed information including download URLs
        #[arg(short, long, conflicts_with = "compact")]
        detailed: bool,

        /// Output results as JSON
        #[arg(long, conflicts_with_all = ["compact", "detailed"])]
        json: bool,

        /// Show only LTS versions
        #[arg(long)]
        lts_only: bool,
    },

    /// Initial setup and configuration
    Setup {
        /// Force recreation of shims even if they exist
        #[arg(short, long)]
        force: bool,
    },

    /// Manage tool shims
    Shim {
        #[command(subcommand)]
        command: ShimCommand,
    },

    /// Uninstall a JDK version
    #[command(visible_alias = "u", alias = "remove")]
    Uninstall {
        /// Version to uninstall (e.g., "21", "17.0.9", "corretto@21")
        version: Option<String>,

        /// Skip confirmation prompts
        #[arg(short, long)]
        force: bool,

        /// Show what would be uninstalled without actually removing
        #[arg(long)]
        dry_run: bool,

        /// Uninstall all versions of a distribution
        #[arg(long)]
        all: bool,

        /// Clean up failed or partial uninstall operations
        #[arg(long)]
        cleanup: bool,
    },

    /// Run diagnostics on kopi installation
    Doctor {
        /// Output results in JSON format
        #[arg(long)]
        json: bool,

        /// Run only specific category of checks
        #[arg(long, value_name = "CATEGORY")]
        check: Option<String>,
    },
}

fn setup_logger(cli: &Cli) {
    logging::setup_logger(cli.verbose);
}

fn main() {
    let cli = Cli::parse();

    // Initialize logger based on CLI flags and environment
    setup_logger(&cli);

    // Load configuration once at startup
    let config = match new_kopi_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", format_error_chain(&e));
            std::process::exit(get_exit_code(&e));
        }
    };

    let result: Result<()> = (|| {
        match cli.command {
            Commands::Install {
                version,
                force,
                dry_run,
                no_progress,
                timeout,
            } => {
                let command = InstallCommand::new(&config, no_progress)?;
                command.execute(&version, force, dry_run, no_progress, timeout)
            }
            Commands::List => {
                let command = ListCommand::new(&config)?;
                command.execute()
            }
            Commands::Shell { version, shell } => {
                let command = ShellCommand::new(&config)?;
                command.execute(&version, shell.as_deref())
            }
            Commands::Current { quiet, json } => {
                let command = CurrentCommand::new(&config)?;
                command.execute(quiet, json)
            }
            Commands::Env {
                version,
                shell,
                export,
            } => {
                let command = EnvCommand::new(&config)?;
                command.execute(version.as_deref(), shell.as_deref(), export)
            }
            Commands::Global { version } => {
                let command = GlobalCommand::new(&config)?;
                command.execute(&version)
            }
            Commands::Local { version } => {
                let command = LocalCommand::new(&config)?;
                command.execute(&version)
            }
            Commands::Which {
                version,
                tool,
                home,
                json,
            } => {
                let command = WhichCommand::new(&config)?;
                command.execute(version.as_deref(), &tool, home, json)
            }
            Commands::Cache { command } => command.execute(&config),
            Commands::Refresh => {
                // Delegate to cache refresh command
                let cache_cmd = CacheCommand::Refresh;
                cache_cmd.execute(&config)
            }
            Commands::Search {
                version,
                compact,
                detailed,
                json,
                lts_only,
            } => {
                // Delegate to cache search command
                let cache_cmd = CacheCommand::Search {
                    version: version.unwrap_or_else(|| "latest".to_string()),
                    compact,
                    detailed,
                    json,
                    lts_only,
                    java_version: false,
                    distribution_version: false,
                };
                cache_cmd.execute(&config)
            }
            Commands::Setup { force } => {
                let command = SetupCommand::new(&config, false)?;
                command.execute(force)
            }
            Commands::Shim { command } => command.execute(&config),
            Commands::Uninstall {
                version,
                force,
                dry_run,
                all,
                cleanup,
            } => {
                let command = UninstallCommand::new(&config)?;
                command.execute(version.as_deref(), force, dry_run, all, cleanup)
            }
            Commands::Doctor { json, check } => {
                let command = DoctorCommand::new(&config)?;
                command.execute(json, cli.verbose > 0, check.as_deref())
            }
        }
    })();

    if let Err(e) = result {
        eprintln!("{}", format_error_chain(&e));
        std::process::exit(get_exit_code(&e));
    }
}
