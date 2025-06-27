use crate::cache;
use crate::error::Result;
use crate::models::jdk::Distribution;
use crate::search::{PackageSearcher, get_current_platform};
use crate::version::parser::VersionParser;
use chrono::Local;
use clap::Subcommand;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// Refresh metadata from foojay.io API
    Refresh {
        /// Include packages regardless of JavaFX bundled status
        #[arg(long)]
        javafx_bundled: bool,
    },
    /// Show cache information
    Info,
    /// Clear all cached data
    Clear,
    /// Search for available JDK versions
    Search {
        /// Version to search for (e.g., "21", "17.0.9", "corretto@21")
        version: String,
    },
}

impl CacheCommand {
    pub fn execute(self) -> Result<()> {
        match self {
            CacheCommand::Refresh { javafx_bundled } => refresh_cache(javafx_bundled),
            CacheCommand::Info => show_cache_info(),
            CacheCommand::Clear => clear_cache(),
            CacheCommand::Search { version } => search_cache(version),
        }
    }
}

fn refresh_cache(javafx_bundled: bool) -> Result<()> {
    println!("Refreshing metadata cache from foojay.io...");

    let cache = cache::fetch_and_cache_metadata_with_options(javafx_bundled)?;

    println!("✓ Cache refreshed successfully");
    println!("  - {} distributions available", cache.distributions.len());

    let total_packages: usize = cache.distributions.values().map(|d| d.packages.len()).sum();
    println!("  - {} total JDK packages", total_packages);

    Ok(())
}

fn show_cache_info() -> Result<()> {
    let cache_path = cache::get_cache_path()?;

    if !cache_path.exists() {
        println!("No cache found. Run 'kopi cache refresh' to populate the cache.");
        return Ok(());
    }

    let cache = cache::load_cache(&cache_path)?;
    let metadata = std::fs::metadata(&cache_path)?;
    let file_size = metadata.len();

    println!("Cache Information:");
    println!("  Location: {}", cache_path.display());
    println!("  Size: {} KB", file_size / 1024);
    println!(
        "  Last updated: {}",
        cache
            .last_updated
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
    );
    println!("  Distributions: {}", cache.distributions.len());

    let total_packages: usize = cache.distributions.values().map(|d| d.packages.len()).sum();
    println!("  Total JDK packages: {}", total_packages);

    Ok(())
}

fn clear_cache() -> Result<()> {
    let cache_path = cache::get_cache_path()?;

    if cache_path.exists() {
        std::fs::remove_file(&cache_path)?;
        println!("✓ Cache cleared successfully");
    } else {
        println!("No cache to clear");
    }

    Ok(())
}

fn search_cache(version_string: String) -> Result<()> {
    let cache_path = cache::get_cache_path()?;

    if !cache_path.exists() {
        println!("No cache found. Run 'kopi cache refresh' to populate the cache.");
        return Ok(());
    }

    // Load cache
    let cache = cache::load_cache(&cache_path)?;

    // Parse the version string to check if distribution was specified
    let mut parsed_request = VersionParser::parse(&version_string)?;

    // If no distribution specified, use Temurin as default (same as install command)
    if parsed_request.distribution.is_none() {
        parsed_request.distribution = Some(Distribution::Temurin);
    }

    // Use the shared searcher
    let searcher = PackageSearcher::new(Some(&cache));
    let results = searcher.search_parsed(&parsed_request)?;

    if results.is_empty() {
        println!("No matching JDK versions found for '{}'", version_string);
        println!("\nTip: Run 'kopi cache refresh' to update available versions.");
        return Ok(());
    }

    // Display results
    println!("Available JDK versions matching '{}':\n", version_string);

    // Get current platform info for determining auto-selection
    let (current_arch, current_os, _) = get_current_platform();

    // Group by distribution for better display
    let mut grouped: HashMap<String, Vec<_>> = HashMap::new();
    for result in results {
        grouped
            .entry(result.distribution.clone())
            .or_default()
            .push(result);
    }

    // Sort distribution names for consistent output
    let mut dist_names: Vec<String> = grouped.keys().cloned().collect();
    dist_names.sort();

    // No need to check for Temurin since we're already filtering by it when no distribution specified

    for dist_name in dist_names {
        if let Some(results) = grouped.get(&dist_name) {
            // Use the display name from the first result
            let display_name = results
                .first()
                .map(|r| r.display_name.as_str())
                .unwrap_or(&dist_name);

            println!("{}:", display_name);

            // Group by version for better display
            let mut version_groups: HashMap<String, Vec<_>> = HashMap::new();
            for result in results {
                let version_key = result.package.version.to_string();
                version_groups.entry(version_key).or_default().push(result);
            }

            for (version_str, version_results) in version_groups {
                // Determine which package would be auto-selected for this version
                let distribution = Distribution::from_str(&dist_name).ok();
                let auto_selected = if let Some(dist) = &distribution {
                    searcher.find_auto_selected_package(
                        dist,
                        &version_str,
                        &current_arch,
                        &current_os,
                    )
                } else {
                    None
                };

                for result in version_results {
                    let package = &result.package;
                    let display_version = if package.version.build.is_some() {
                        format!("{} ({})", package.version.major, package.version)
                    } else if package.version.patch > 0 {
                        format!(
                            "{}.{}.{}",
                            package.version.major, package.version.minor, package.version.patch
                        )
                    } else if package.version.minor > 0 {
                        format!("{}.{}", package.version.major, package.version.minor)
                    } else {
                        format!("{}", package.version.major)
                    };

                    let size_mb = package.size / (1024 * 1024);

                    // Check if this package would be auto-selected
                    let is_auto_selected = auto_selected
                        .as_ref()
                        .map(|selected| selected.id == package.id)
                        .unwrap_or(false);

                    // Only show platform info if it's different from current platform
                    let show_platform = package.architecture.to_string() != current_arch
                        || package.operating_system.to_string() != current_os;

                    if show_platform {
                        let javafx_info = if package.javafx_bundled {
                            ", JavaFX"
                        } else {
                            ""
                        };
                        println!(
                            "  {} - {} {} ({} MB, {}, {}{})",
                            display_version,
                            package.architecture,
                            package.operating_system,
                            size_mb,
                            package.package_type,
                            package.archive_type,
                            javafx_info
                        );
                    } else {
                        // For current platform, show if it would be auto-selected
                        let indicator = if is_auto_selected { " [default]" } else { "" };
                        let lib_c_info = package
                            .lib_c_type
                            .as_ref()
                            .map(|l| format!(" ({})", l))
                            .unwrap_or_default();
                        let javafx_info = if package.javafx_bundled {
                            ", JavaFX"
                        } else {
                            ""
                        };
                        println!(
                            "  {} - {} MB, {}, {}{}{}{}",
                            display_version,
                            size_mb,
                            package.package_type,
                            package.archive_type,
                            javafx_info,
                            lib_c_info,
                            indicator
                        );
                    }
                }
            }
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_cache_info_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let result = show_cache_info();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_cache_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let result = clear_cache();
        assert!(result.is_ok());
    }
}
