use clap::{Parser, Subcommand};
use kopi::error::{format_error_with_color, get_exit_code};
use kopi::metadata::{GeneratorConfig, MetadataGenConfigFile, MetadataGenerator, Platform};
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

        /// Configuration file path (TOML format)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Update existing metadata
    Update {
        /// Input directory with existing metadata
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for updated metadata
        #[arg(short, long)]
        output: PathBuf,

        /// Dry run - show what would be updated without actually writing files
        #[arg(long)]
        dry_run: bool,

        /// Force fresh generation, ignoring any existing state files
        #[arg(long)]
        force: bool,

        /// Override parallel requests setting
        #[arg(long)]
        parallel: Option<usize>,

        /// Configuration file path (TOML format)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Validate metadata structure
    Validate {
        /// Directory to validate
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Generate example configuration file
    GenerateConfig {
        /// Output path for configuration file
        #[arg(short, long)]
        output: PathBuf,
    },
}

/// Load and apply configuration file to the generator config
fn load_and_apply_config(config_path: Option<PathBuf>, generator_config: &mut GeneratorConfig) {
    if let Some(config_path) = config_path {
        match MetadataGenConfigFile::load(&config_path) {
            Ok(config_file) => {
                if let Err(e) = config_file.apply_to_config(generator_config) {
                    eprintln!("Error applying configuration: {e}");
                    std::process::exit(get_exit_code(&e));
                }
                println!("📄 Loaded configuration from {}", config_path.display());
            }
            Err(e) => {
                eprintln!("Error loading configuration file: {e}");
                std::process::exit(get_exit_code(&e));
            }
        }
    }
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
            config,
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

            let mut generator_config = GeneratorConfig {
                distributions: dist_list,
                platforms: platform_list,
                javafx_bundled: javafx,
                parallel_requests: parallel,
                dry_run,
                minify_json: !no_minify,
                force,
            };

            // Load and apply configuration file if provided
            load_and_apply_config(config, &mut generator_config);

            let generator = MetadataGenerator::new(generator_config);
            generator.generate(&output)
        }
        Commands::Update {
            input,
            output,
            dry_run,
            force,
            parallel,
            config,
        } => {
            // Load the existing index.json to get the original generator config
            let index_path = input.join("index.json");
            if !index_path.exists() {
                eprintln!("Error: index.json not found in {}", input.display());
                std::process::exit(1);
            }

            let index_content = match std::fs::read_to_string(&index_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading index.json: {e}");
                    std::process::exit(1);
                }
            };

            let index: kopi::metadata::index::IndexFile = match serde_json::from_str(&index_content)
            {
                Ok(index) => index,
                Err(e) => {
                    eprintln!("Error parsing index.json: {e}");
                    std::process::exit(1);
                }
            };

            // Use the generator config from index.json if available, otherwise use defaults
            let mut generator_config = if let Some(mut saved_config) = index.generator_config {
                // Apply runtime flags and overrides
                saved_config.dry_run = dry_run;
                saved_config.force = force;
                if let Some(p) = parallel {
                    saved_config.parallel_requests = p;
                }
                saved_config
            } else {
                // Fallback for older index.json files without generator_config
                GeneratorConfig {
                    distributions: None,
                    platforms: None,
                    javafx_bundled: false,
                    parallel_requests: parallel.unwrap_or(4),
                    dry_run,
                    minify_json: true,
                    force,
                }
            };

            // Load and apply configuration file if provided
            load_and_apply_config(config, &mut generator_config);

            let generator = MetadataGenerator::new(generator_config);
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
        Commands::GenerateConfig { output } => (|| -> kopi::error::Result<()> {
            let example_config = MetadataGenConfigFile::default_example();
            let toml_content = toml::to_string_pretty(&example_config).map_err(|e| {
                kopi::error::KopiError::InvalidConfig(format!("Failed to serialize config: {e}"))
            })?;

            std::fs::write(&output, toml_content)?;

            println!(
                "\u{2705} Generated example configuration file at {}",
                output.display()
            );
            println!(
                "\n\u{1f527} Usage: kopi-metadata-gen generate --config {} --output ./metadata",
                output.display()
            );
            Ok(())
        })(),
    };

    if let Err(e) = result {
        eprintln!(
            "{}",
            format_error_with_color(&e, std::io::stderr().is_terminal())
        );
        std::process::exit(get_exit_code(&e));
    }
}
