use clap::{Parser, Subcommand};
use kopi::commands::cache::CacheCommand;
use kopi::commands::install::InstallCommand;
use kopi::error::Result;

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

    /// Switch to a specific JDK version
    Use {
        /// Version to use
        version: String,
    },

    /// Show current JDK version
    Current,

    /// Set global default JDK version
    Global {
        /// Version to set as global default
        version: String,
    },

    /// Set project-specific JDK version
    Local {
        /// Version to set for current project
        version: String,
    },

    /// Show JDK installation path
    Which {
        /// Version to show path for (defaults to current)
        version: Option<String>,
    },

    /// Manage metadata cache
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },

    /// Refresh metadata cache from foojay.io (alias for 'cache refresh')
    Refresh {
        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
    },

    /// Search for available JDK versions (alias for 'cache search')
    Search {
        /// Query to search for (e.g., "21", "17.0.9", "corretto@21", "corretto", "latest")
        version: String,
        /// Display minimal information (default)
        #[arg(long, conflicts_with_all = ["detailed", "json"])]
        compact: bool,
        /// Display detailed information including OS/Arch and Status
        #[arg(long, conflicts_with_all = ["compact", "json"])]
        detailed: bool,
        /// Output results as JSON for programmatic use
        #[arg(long, conflicts_with_all = ["compact", "detailed"])]
        json: bool,
        /// Filter to show only LTS versions
        #[arg(long)]
        lts_only: bool,
        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
    },
}

fn setup_logger(cli: &Cli) {
    // CLI flags set the default level
    let default_level = match cli.verbose {
        0 => "warn",  // Default: only warnings and errors
        1 => "info",  // -v: show info messages
        2 => "debug", // -vv: show debug messages
        _ => "trace", // -vvv or more: show everything
    };

    // RUST_LOG can override if set
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_timestamp(None) // No timestamps for CLI output
        .format_module_path(false) // Cleaner output
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger based on CLI flags and environment
    setup_logger(&cli);

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
            )?;
        }
        Commands::List { all } => {
            if all {
                println!("Listing all available JDK versions (not yet implemented)");
            } else {
                println!("Listing installed JDK versions (not yet implemented)");
            }
        }
        Commands::Use { version } => {
            println!("Switching to JDK {} (not yet implemented)", version);
        }
        Commands::Current => {
            println!("Current JDK version (not yet implemented)");
        }
        Commands::Global { version } => {
            println!("Setting global JDK to {} (not yet implemented)", version);
        }
        Commands::Local { version } => {
            println!("Setting local JDK to {} (not yet implemented)", version);
        }
        Commands::Which { version } => {
            let v = version.unwrap_or_else(|| "current".to_string());
            println!("Path for JDK {} (not yet implemented)", v);
        }
        Commands::Cache { command } => {
            command.execute()?;
        }
        Commands::Refresh { javafx_bundled } => {
            // Delegate to cache refresh command
            let cache_cmd = CacheCommand::Refresh { javafx_bundled };
            cache_cmd.execute()?;
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
                version,
                compact,
                detailed,
                json,
                lts_only,
                javafx_bundled,
            };
            cache_cmd.execute()?;
        }
    }

    Ok(())
}
