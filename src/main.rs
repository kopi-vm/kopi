use clap::{Parser, Subcommand};
use kopi::commands::install::InstallCommand;
use kopi::error::Result;

#[derive(Parser)]
#[command(name = "kopi")]
#[command(author, version, about = "JDK version management tool", long_about = None)]
struct Cli {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            version,
            force,
            dry_run,
            no_progress,
            timeout,
        } => {
            let command = InstallCommand::new()?;
            command.execute(&version, force, dry_run, no_progress, timeout)?;
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
    }

    Ok(())
}
