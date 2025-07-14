use clap::{Parser, Subcommand};
use kopi::commands::cache::CacheCommand;
use kopi::commands::current::CurrentCommand;
use kopi::commands::global::GlobalCommand;
use kopi::commands::install::InstallCommand;
use kopi::commands::local::LocalCommand;
use kopi::commands::setup::SetupCommand;
use kopi::commands::shell::ShellCommand;
use kopi::commands::shim::ShimCommand;
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

        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
    },

    /// List installed JDK versions
    #[command(visible_alias = "ls")]
    List {
        /// Show all versions including remote ones
        #[arg(short, long)]
        all: bool,
    },

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
        /// Version to locate (defaults to current)
        version: Option<String>,
    },

    /// Manage JDK metadata cache
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },

    /// Refresh JDK metadata cache (alias for cache refresh)
    #[command(visible_alias = "r", hide = true)]
    Refresh {
        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
    },

    /// Search available JDK versions (alias for cache search)
    #[command(visible_alias = "s", hide = true)]
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

        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
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
}

fn setup_logger(cli: &Cli) {
    logging::setup_logger(cli.verbose);
}

fn main() {
    let cli = Cli::parse();

    // Initialize logger based on CLI flags and environment
    setup_logger(&cli);

    let result: Result<()> = (|| {
        match cli.command {
            Commands::Install {
                version,
                force,
                dry_run,
                no_progress,
                timeout,
                javafx_bundled,
            } => {
                let command = InstallCommand::new()?;
                command.execute(
                    &version,
                    force,
                    dry_run,
                    no_progress,
                    timeout,
                    javafx_bundled,
                )
            }
            Commands::List { all } => {
                if all {
                    println!("Listing all available JDK versions (not yet implemented)");
                } else {
                    println!("Listing installed JDK versions (not yet implemented)");
                }
                Ok(())
            }
            Commands::Shell { version, shell } => {
                let command = ShellCommand::new()?;
                command.execute(&version, shell.as_deref())
            }
            Commands::Current { quiet, json } => {
                let command = CurrentCommand::new()?;
                command.execute(quiet, json)
            }
            Commands::Global { version } => {
                let command = GlobalCommand::new()?;
                command.execute(&version)
            }
            Commands::Local { version } => {
                let command = LocalCommand::new()?;
                command.execute(&version)
            }
            Commands::Which { version } => {
                let v = version.unwrap_or_else(|| "current".to_string());
                println!("Path for JDK {v} (not yet implemented)");
                Ok(())
            }
            Commands::Cache { command } => command.execute(),
            Commands::Refresh { javafx_bundled } => {
                // Delegate to cache refresh command
                let cache_cmd = CacheCommand::Refresh { javafx_bundled };
                cache_cmd.execute()
            }
            Commands::Search {
                version,
                compact,
                detailed,
                json,
                lts_only,
                javafx_bundled,
            } => {
                // Delegate to cache search command
                let cache_cmd = CacheCommand::Search {
                    version: version.unwrap_or_else(|| "latest".to_string()),
                    compact,
                    detailed,
                    json,
                    lts_only,
                    javafx_bundled,
                    java_version: false,
                    distribution_version: false,
                };
                cache_cmd.execute()
            }
            Commands::Setup { force } => {
                let command = SetupCommand::new()?;
                command.execute(force)
            }
            Commands::Shim { command } => command.execute(),
        }
    })();

    if let Err(e) = result {
        eprintln!("{}", format_error_chain(&e));
        std::process::exit(get_exit_code(&e));
    }
}
