use clap::{Parser, Subcommand};
use kopi::error::{format_error_with_color, get_exit_code};
use kopi::metadata::{GeneratorConfig, MetadataGenerator, Platform};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "kopi-metadata-gen")]
#[command(about = "Generate metadata files from foojay API")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate metadata from foojay API
    Generate {
        /// Output directory for metadata files
        #[arg(short, long)]
        output: PathBuf,

        /// Specific distributions to include (comma-separated)
        #[arg(long)]
        distributions: Option<String>,

        /// Specific platforms to include (format: os-arch-libc)
        #[arg(long)]
        platforms: Option<String>,

        /// Include JavaFX bundled versions
        #[arg(long)]
        javafx: bool,

        /// Number of parallel API requests
        #[arg(long, default_value = "4")]
        parallel: usize,

        /// Dry run - show what would be generated without actually writing files
        #[arg(long)]
        dry_run: bool,

        /// Don't minify JSON output (default is to minify)
        #[arg(long = "no-minify")]
        no_minify: bool,

        /// Force fresh generation, ignoring any existing state files
        #[arg(long)]
        force: bool,
    },

    /// Update existing metadata (not implemented yet)
    Update {
        /// Input directory with existing metadata
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for updated metadata
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Validate metadata structure
    Validate {
        /// Directory to validate
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Generate {
            output,
            distributions,
            platforms,
            javafx,
            parallel,
            dry_run,
            no_minify,
            force,
        } => {
            // Parse distributions
            let dist_list =
                distributions.map(|d| d.split(',').map(|s| s.trim().to_string()).collect());

            // Parse platforms
            let platform_list = if let Some(p) = platforms {
                let mut list = Vec::new();
                for platform_str in p.split(',') {
                    match Platform::from_str(platform_str.trim()) {
                        Ok(platform) => list.push(platform),
                        Err(e) => {
                            eprintln!("Error parsing platform '{platform_str}': {e}");
                            std::process::exit(get_exit_code(&e));
                        }
                    }
                }
                Some(list)
            } else {
                None
            };

            let config = GeneratorConfig {
                distributions: dist_list,
                platforms: platform_list,
                javafx_bundled: javafx,
                parallel_requests: parallel,
                dry_run,
                minify_json: !no_minify,
                force,
            };

            let generator = MetadataGenerator::new(config);
            generator.generate(&output)
        }
        Commands::Update { input, output } => {
            let config = GeneratorConfig {
                distributions: None, // Use same filters as existing metadata
                platforms: None,
                javafx_bundled: false,
                parallel_requests: 4,
                dry_run: false,
                minify_json: true,
                force: false,
            };
            let generator = MetadataGenerator::new(config);
            generator.update(&input, &output)
        }
        Commands::Validate { input } => {
            let config = GeneratorConfig {
                distributions: None,
                platforms: None,
                javafx_bundled: false,
                parallel_requests: 1,
                dry_run: false,
                minify_json: true,
                force: false,
            };
            let generator = MetadataGenerator::new(config);
            generator.validate(&input)
        }
    };

    if let Err(e) = result {
        eprintln!(
            "{}",
            format_error_with_color(&e, std::io::stderr().is_terminal())
        );
        std::process::exit(get_exit_code(&e));
    }
}
