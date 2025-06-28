use crate::cache;
use crate::error::Result;
use crate::models::jdk::Distribution;
use crate::search::{PackageSearcher, get_current_platform};
use crate::version::parser::VersionParser;
use chrono::Local;
use clap::Subcommand;
use comfy_table::{Cell, CellAlignment, Table};
use std::collections::{HashMap, HashSet};

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
        /// Display minimal information (default)
        #[arg(long, conflicts_with_all = ["detailed", "json"])]
        compact: bool,
        /// Display detailed information including OS/Arch and Status
        #[arg(long, conflicts_with_all = ["compact", "json"])]
        detailed: bool,
        /// Output results as JSON for programmatic use
        #[arg(long, conflicts_with_all = ["compact", "detailed"])]
        json: bool,
    },
}

impl CacheCommand {
    pub fn execute(self) -> Result<()> {
        match self {
            CacheCommand::Refresh { javafx_bundled } => refresh_cache(javafx_bundled),
            CacheCommand::Info => show_cache_info(),
            CacheCommand::Clear => clear_cache(),
            CacheCommand::Search {
                version,
                compact,
                detailed,
                json,
            } => search_cache(version, compact, detailed, json),
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

fn search_cache(version_string: String, _compact: bool, detailed: bool, json: bool) -> Result<()> {
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
        if json {
            println!("[]");
        } else {
            println!("No matching Java versions found for '{}'", version_string);
            println!("\nTip: Run 'kopi cache refresh' to update available versions.");
        }
        return Ok(());
    }

    // JSON output mode
    if json {
        let json_output = serde_json::to_string_pretty(&results)?;
        println!("{}", json_output);
        return Ok(());
    }

    // Display results for table modes
    println!("Available Java versions matching '{}':\n", version_string);

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

    // Check if any package has JavaFX bundled to determine if we need that column
    let has_javafx = grouped
        .values()
        .any(|results| results.iter().any(|r| r.package.javafx_bundled));

    for dist_name in dist_names {
        if let Some(results) = grouped.get(&dist_name) {
            // Use the display name from the first result
            let display_name = results
                .first()
                .map(|r| r.display_name.as_str())
                .unwrap_or(&dist_name);

            // Create a table for this distribution
            let mut table = Table::new();
            table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);

            // Set the header with distribution name in the top-left
            let mut headers = if detailed {
                vec![
                    Cell::new(display_name),
                    Cell::new("Version"),
                    Cell::new("LTS"),
                    Cell::new("Status"),
                    Cell::new("Type"),
                    Cell::new("OS/Arch"),
                    Cell::new("LibC"),
                    Cell::new("Size"),
                ]
            } else {
                // Compact mode (default)
                vec![
                    Cell::new(display_name),
                    Cell::new("Version"),
                    Cell::new("LTS"),
                ]
            };

            if has_javafx {
                headers.push(Cell::new("JavaFX"));
            }

            table.set_header(headers);

            // Configure column alignments
            table
                .column_mut(2)
                .unwrap()
                .set_cell_alignment(CellAlignment::Center); // LTS column

            if detailed {
                table
                    .column_mut(7)
                    .unwrap()
                    .set_cell_alignment(CellAlignment::Right); // Size column
                table
                    .column_mut(3)
                    .unwrap()
                    .set_cell_alignment(CellAlignment::Center); // Status column
            }

            // Sort results
            let mut sorted_results = results.clone();
            sorted_results.sort_by(|a, b| {
                use crate::models::jdk::PackageType;

                // In detailed mode, sort by size first (ascending) for deduplication
                if detailed {
                    match a.package.size.cmp(&b.package.size) {
                        std::cmp::Ordering::Equal => {} // Continue to other criteria
                        other => return other,
                    }
                }

                // If package type was explicitly specified, prioritize matching packages
                if let Some(requested_type) = parsed_request.package_type {
                    match (
                        a.package.package_type == requested_type,
                        b.package.package_type == requested_type,
                    ) {
                        (true, false) => return std::cmp::Ordering::Less,
                        (false, true) => return std::cmp::Ordering::Greater,
                        _ => {} // Both match or both don't match, continue to next criteria
                    }
                }

                // If no package type specified, prioritize JDK over JRE
                if parsed_request.package_type.is_none() {
                    match (a.package.package_type, b.package.package_type) {
                        (PackageType::Jdk, PackageType::Jre) => return std::cmp::Ordering::Less,
                        (PackageType::Jre, PackageType::Jdk) => return std::cmp::Ordering::Greater,
                        _ => {} // Same package type, continue to next criteria
                    }
                }

                // Finally, sort by version (descending)
                b.package.version.cmp(&a.package.version)
            });

            // Deduplication tracking
            let mut seen_compact_entries = HashSet::new();
            let mut seen_detailed_entries = HashSet::new();

            for result in sorted_results {
                let package = &result.package;

                // Only show packages for current platform
                let show_package = package.architecture.to_string() == current_arch
                    && package.operating_system.to_string() == current_os;

                if show_package {
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

                    // Determine LTS status
                    let lts_display = package
                        .term_of_support
                        .as_ref()
                        .map(|tos| match tos.to_lowercase().as_str() {
                            "lts" => "LTS",
                            "sts" => "STS",
                            _ => "-",
                        })
                        .unwrap_or("-");

                    // Deduplication based on display mode
                    if detailed && !json {
                        // In detailed mode, deduplicate based on all visible fields except size
                        let status_display = package
                            .release_status
                            .as_ref()
                            .map(|rs| match rs.to_lowercase().as_str() {
                                "ga" => "GA",
                                "ea" => "EA",
                                _ => rs.as_str(),
                            })
                            .unwrap_or("-");

                        let os_arch =
                            format!("{}/{}", package.operating_system, package.architecture);
                        let lib_c = package.lib_c_type.as_deref().unwrap_or("-");

                        let detailed_key = format!(
                            "{}-{}-{}-{}-{}-{}-{}",
                            dist_name,
                            display_version,
                            lts_display,
                            status_display,
                            package.package_type,
                            os_arch,
                            lib_c
                        );

                        if !seen_detailed_entries.insert(detailed_key) {
                            // Already seen this combination, skip it (keeping the smaller size)
                            continue;
                        }
                    } else if !detailed && !json {
                        // In compact mode, deduplicate based on version and LTS
                        let compact_key = format!("{}-{}", display_version, lts_display);
                        if !seen_compact_entries.insert(compact_key) {
                            // Already seen this combination, skip it
                            continue;
                        }
                    }

                    let mut row = if detailed {
                        // Detailed mode
                        let status_display = package
                            .release_status
                            .as_ref()
                            .map(|rs| match rs.to_lowercase().as_str() {
                                "ga" => "GA",
                                "ea" => "EA",
                                _ => rs.as_str(),
                            })
                            .unwrap_or("-");

                        let os_arch =
                            format!("{}/{}", package.operating_system, package.architecture);

                        vec![
                            Cell::new(""), // Empty first cell for distribution name
                            Cell::new(display_version),
                            Cell::new(lts_display),
                            Cell::new(status_display),
                            Cell::new(package.package_type.to_string()),
                            Cell::new(os_arch),
                            Cell::new(package.lib_c_type.as_deref().unwrap_or("-")),
                            Cell::new(format!("{} MB", size_mb)),
                        ]
                    } else {
                        // Compact mode (default)
                        vec![
                            Cell::new(""), // Empty first cell for distribution name
                            Cell::new(display_version),
                            Cell::new(lts_display),
                        ]
                    };

                    if has_javafx {
                        row.push(
                            Cell::new(if package.javafx_bundled { "✓" } else { "" })
                                .set_alignment(CellAlignment::Center),
                        );
                    }

                    table.add_row(row);
                }
            }

            // Only print the table if it has rows
            if table.row_count() > 0 {
                println!("{}\n", table);
            }
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
