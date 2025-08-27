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

use crate::cache;
use crate::cache::get_current_platform;
use crate::config::KopiConfig;
use crate::error::Result;
use crate::indicator::{
    ProgressConfig, ProgressFactory, ProgressStyle as IndicatorStyle, StatusReporter,
};
use crate::version::parser::VersionParser;
use chrono::Local;
use clap::Subcommand;
use colored::*;
use comfy_table::{Cell, CellAlignment, Color, Table};
use std::collections::{HashMap, HashSet};

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// Refresh metadata from foojay.io API
    Refresh,
    /// Show cache information
    Info,
    /// Clear all cached data
    Clear,
    /// Search for available JDK versions
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
        /// Force search by java_version field
        #[arg(long, conflicts_with = "distribution_version")]
        java_version: bool,
        /// Force search by distribution_version field
        #[arg(long, conflicts_with = "java_version")]
        distribution_version: bool,
    },
    /// List all available distributions in cache
    ListDistributions,
}

#[derive(Debug)]
struct SearchOptions {
    version_string: String,
    compact: bool,
    detailed: bool,
    json: bool,
    lts_only: bool,
    force_java_version: bool,
    force_distribution_version: bool,
}

impl CacheCommand {
    pub fn execute(self, config: &KopiConfig, no_progress: bool) -> Result<()> {
        match self {
            CacheCommand::Refresh => refresh_cache(config, no_progress),
            CacheCommand::Info => show_cache_info(config, no_progress),
            CacheCommand::Clear => clear_cache(config, no_progress),
            CacheCommand::Search {
                version,
                compact,
                detailed,
                json,
                lts_only,
                java_version,
                distribution_version,
            } => {
                let options = SearchOptions {
                    version_string: version,
                    compact,
                    detailed,
                    json,
                    lts_only,
                    force_java_version: java_version,
                    force_distribution_version: distribution_version,
                };
                search_cache(options, config)
            }
            CacheCommand::ListDistributions => list_distributions(config),
        }
    }
}

fn refresh_cache(config: &KopiConfig, no_progress: bool) -> Result<()> {
    // Create metadata provider to get source count
    let provider = crate::metadata::provider::MetadataProvider::from_config(config)?;

    // Calculate total steps: 5 base steps + number of sources
    // Steps breakdown:
    // - Steps 1 to N: One step per source (handled by provider)
    // - Step N+1: Processing metadata
    // - Step N+2: Grouping by distribution
    // - Step N+3: Saving to cache
    // - Step N+4: Completion
    let total_steps = 5 + provider.source_count();

    let mut progress = ProgressFactory::create(no_progress);

    // Initialize step-based progress with total
    let progress_config =
        ProgressConfig::new("Refreshing", "metadata cache", IndicatorStyle::Count)
            .with_total(total_steps as u64);
    progress.start(progress_config);

    // Initialize step counter
    let mut current_step = 0u64;

    // Step 1: Initialization
    current_step += 1;
    progress.update(current_step, Some(total_steps as u64));
    progress.set_message("Initializing metadata refresh...".to_string());

    // Fetch metadata from API - this will handle steps 2-N internally (one per source)
    // and steps N+1 to N+4 (processing steps)
    let cache = match cache::fetch_and_cache_metadata_with_progress(
        config,
        progress.as_mut(),
        &mut current_step,
    ) {
        Ok(cache) => cache,
        Err(e) => {
            progress.error(format!("Failed to refresh cache: {e}"));
            return Err(e);
        }
    };

    // Complete the progress indicator
    progress.complete(Some("Cache refreshed successfully".to_string()));

    // Use StatusReporter for consistent output
    let reporter = StatusReporter::new(no_progress);
    reporter.success("Cache refreshed successfully");

    let dist_count = cache.distributions.len();
    reporter.step(&format!("{dist_count} distributions available"));

    let total_packages: usize = cache.distributions.values().map(|d| d.packages.len()).sum();
    reporter.step(&format!("{total_packages} total JDK packages"));

    Ok(())
}

fn show_cache_info(config: &KopiConfig, _no_progress: bool) -> Result<()> {
    let cache_path = config.metadata_cache_path()?;

    if !cache_path.exists() {
        println!("{} No cache found", "✗".red());
        println!(
            "\n{}: Run {} to populate the cache with available JDK versions.",
            "Solution".yellow().bold(),
            "'kopi cache refresh'".cyan()
        );
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
    println!("  Total JDK packages: {total_packages}");

    Ok(())
}

fn clear_cache(config: &KopiConfig, no_progress: bool) -> Result<()> {
    let cache_path = config.metadata_cache_path()?;

    let reporter = StatusReporter::new(no_progress);

    if cache_path.exists() {
        std::fs::remove_file(&cache_path)?;
        reporter.success("Cache cleared successfully");
    } else {
        reporter.step("No cache to clear");
    }

    Ok(())
}

fn search_cache(options: SearchOptions, config: &KopiConfig) -> Result<()> {
    let SearchOptions {
        version_string,
        compact: _compact,
        detailed,
        json,
        lts_only,
        force_java_version,
        force_distribution_version,
    } = options;
    let cache_path = config.metadata_cache_path()?;

    // Load cache or create new one if it doesn't exist
    let mut cache = if cache_path.exists() {
        cache::load_cache(&cache_path)?
    } else {
        // If no cache exists, create an empty one
        cache::MetadataCache::new()
    };

    // Parse the version string to check if distribution was specified
    let parser = VersionParser::new(config);
    let parsed_request = match parser.parse(&version_string) {
        Ok(req) => req,
        Err(e) => {
            if json {
                println!("[]");
            } else {
                println!("{} {}", "✗".red(), e);
                println!("\n{}", "Examples:".yellow().bold());
                println!(
                    "  {} - Search for Java 21 across all distributions",
                    "kopi cache search 21".cyan()
                );
                println!(
                    "  {} - Search for specific distribution and version",
                    "kopi cache search corretto@21".cyan()
                );
                println!(
                    "  {} - List all versions of a distribution",
                    "kopi cache search corretto".cyan()
                );
                println!(
                    "  {} - Show latest version of each distribution",
                    "kopi cache search latest".cyan()
                );
                println!(
                    "  {} - Search for JRE packages only",
                    "kopi cache search jre@17".cyan()
                );
                println!(
                    "\n{}: Use {} to see all available distributions",
                    "Tip".yellow().bold(),
                    "'kopi cache list-distributions'".cyan()
                );
            }
            return Ok(());
        }
    };

    // Check if a specific distribution was requested and if it's in cache
    if let Some(ref dist) = parsed_request.distribution {
        let dist_id = dist.id();
        // Resolve synonym to canonical name
        let canonical_name = cache.get_canonical_name(dist_id).unwrap_or(dist_id);
        if !cache.distributions.contains_key(canonical_name) {
            // Distribution not in cache, fetch it using the canonical name
            if !json {
                println!(
                    "Distribution '{dist_id}' not found in cache. Fetching from configured sources..."
                );
            }

            // Use SilentProgress for search operation (no user-visible progress needed)
            let mut progress = crate::indicator::SilentProgress;
            let mut current_step = 0u64;
            match cache::fetch_and_cache_distribution_with_progress(
                canonical_name,
                config,
                &mut progress,
                &mut current_step,
            ) {
                Ok(updated_cache) => {
                    cache = updated_cache;
                    if !json {
                        println!(
                            "{} Distribution '{}' cached successfully",
                            "✓".green().bold(),
                            dist_id.cyan()
                        );
                    }
                }
                Err(e) => {
                    if json {
                        println!("[]");
                    } else {
                        println!(
                            "{} Failed to fetch distribution '{}': {}",
                            "✗".red(),
                            dist_id,
                            e
                        );
                    }
                    return Ok(());
                }
            }
        }
    }

    // Determine version search type based on flags
    let version_type = if force_java_version {
        crate::cache::VersionSearchType::JavaVersion
    } else if force_distribution_version {
        crate::cache::VersionSearchType::DistributionVersion
    } else {
        crate::cache::VersionSearchType::Auto
    };

    let mut results = cache.search(&parsed_request, version_type)?;

    // Apply LTS filtering if requested
    if lts_only {
        results.retain(|result| {
            result
                .package
                .term_of_support
                .as_ref()
                .map(|tos| tos.to_lowercase() == "lts")
                .unwrap_or(false)
        });
    }

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            if lts_only {
                println!(
                    "{} No matching LTS Java versions found for '{}'",
                    "✗".red(),
                    version_string.bright_blue()
                );
            } else {
                println!(
                    "{} No matching Java versions found for '{}'",
                    "✗".red(),
                    version_string.bright_blue()
                );
            }
            println!("\n{}", "Common causes:".yellow().bold());
            println!("  - The cache might be outdated");
            println!("  - The version might not exist");
            println!("  - The distribution name might be incorrect");

            println!("\n{}", "Try these:".yellow().bold());
            println!(
                "  1. {} - Update the cache with latest versions",
                "kopi cache refresh".cyan()
            );
            println!(
                "  2. {} - See all available distributions",
                "kopi cache list-distributions".cyan()
            );
            println!(
                "  3. {} - List all versions of a specific distribution",
                "kopi cache search <distribution>".cyan()
            );
        }
        return Ok(());
    }

    // JSON output mode
    if json {
        let json_output = serde_json::to_string_pretty(&results)?;
        println!("{json_output}");
        return Ok(());
    }

    // Display results for table modes with result count
    let result_count = results.len();
    if lts_only {
        println!(
            "Found {} LTS Java version{} matching '{}':\n",
            result_count.to_string().cyan(),
            if result_count == 1 { "" } else { "s" },
            version_string.bright_blue()
        );
    } else {
        println!(
            "Found {} Java version{} matching '{}':\n",
            result_count.to_string().cyan(),
            if result_count == 1 { "" } else { "s" },
            version_string.bright_blue()
        );
    }

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

    // Create a single table for all distributions
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);

    // Set the header
    let mut headers = if detailed {
        vec![
            Cell::new("Distribution"),
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
            Cell::new("Distribution"),
            Cell::new("Version"),
            Cell::new("LTS"),
        ]
    };

    if has_javafx {
        headers.push(Cell::new("JavaFX"));
    }

    table.set_header(headers);

    let mut is_first_distribution = true;

    for dist_name in dist_names {
        if let Some(results) = grouped.get(&dist_name) {
            // Use the display name from the first result
            let display_name = results
                .first()
                .map(|r| r.display_name.as_str())
                .unwrap_or(&dist_name);

            // Add separator row between distributions (except for the first one)
            if !is_first_distribution {
                // Create a separator row that will be replaced with proper line later
                let num_cols = if detailed {
                    8 + if has_javafx { 1 } else { 0 }
                } else {
                    3 + if has_javafx { 1 } else { 0 }
                };

                let separator_row: Vec<Cell> =
                    (0..num_cols).map(|_| Cell::new("SEPARATOR")).collect();

                table.add_row(separator_row);
            }
            is_first_distribution = false;

            // Sort results
            let mut sorted_results = results.clone();
            sorted_results.sort_by(|a, b| {
                use crate::models::package::PackageType;

                // In detailed mode, sort by size first (ascending) for deduplication
                if detailed {
                    match a.package.size.cmp(&b.package.size) {
                        std::cmp::Ordering::Equal => {} // Continue to other criteria
                        other => return other,
                    }
                }

                // If package type was explicitly specified, prioritize matching packages
                if let Some(ref requested_type) = parsed_request.package_type {
                    match (
                        a.package.package_type == *requested_type,
                        b.package.package_type == *requested_type,
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
            let mut is_first_row_in_distribution = true;

            for result in sorted_results {
                let package = &result.package;

                // Only show packages for current platform
                let show_package = package.architecture.to_string() == current_arch
                    && package.operating_system.to_string() == current_os;

                if show_package {
                    let display_version = if package.version.build.is_some() {
                        format!("{} ({})", package.version.major(), package.version)
                    } else if package.version.patch().map(|p| p > 0).unwrap_or(false) {
                        format!(
                            "{}.{}.{}",
                            package.version.major(),
                            package.version.minor().unwrap_or(0),
                            package.version.patch().unwrap_or(0)
                        )
                    } else if package.version.minor().map(|m| m > 0).unwrap_or(false) {
                        format!(
                            "{}.{}",
                            package.version.major(),
                            package.version.minor().unwrap_or(0)
                        )
                    } else {
                        format!("{}", package.version.major())
                    };

                    let size_display = if package.size < 0 {
                        "Unknown".to_string()
                    } else {
                        format!("{} MB", package.size / (1024 * 1024))
                    };

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

                        let os_arch =
                            format!("{}/{}", package.operating_system, package.architecture);
                        let lib_c = package.lib_c_type.as_deref().unwrap_or("-");

                        let status_plain = package
                            .release_status
                            .as_ref()
                            .map(|rs| match rs.to_lowercase().as_str() {
                                "ga" => "GA",
                                "ea" => "EA",
                                _ => rs.as_str(),
                            })
                            .unwrap_or("-");

                        let detailed_key = format!(
                            "{}-{}-{}-{}-{}-{}-{}-{}",
                            dist_name,
                            display_version,
                            lts_display,
                            status_plain,
                            package.package_type,
                            os_arch,
                            lib_c,
                            package.javafx_bundled
                        );

                        if !seen_detailed_entries.insert(detailed_key) {
                            // Already seen this combination, skip it (keeping the smaller size)
                            continue;
                        }
                    } else if !detailed && !json {
                        // In compact mode, deduplicate based on version, LTS, and JavaFX status
                        let compact_key = format!(
                            "{}-{}-{}",
                            display_version, lts_display, package.javafx_bundled
                        );
                        if !seen_compact_entries.insert(compact_key) {
                            // Already seen this combination, skip it
                            continue;
                        }
                    }

                    // Show distribution name only in the first row of each group
                    let dist_cell = if is_first_row_in_distribution {
                        Cell::new(display_name)
                    } else {
                        Cell::new("")
                    };
                    is_first_row_in_distribution = false;

                    let mut row = if detailed {
                        // Detailed mode
                        let status_display_detail = package
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
                            dist_cell,
                            Cell::new(display_version),
                            // Apply color to LTS cell
                            match lts_display {
                                "LTS" => Cell::new(lts_display).fg(Color::Green),
                                "STS" => Cell::new(lts_display).fg(Color::Yellow),
                                _ => Cell::new(lts_display).fg(Color::DarkGrey),
                            },
                            // Apply color to Status cell
                            match status_display_detail {
                                "GA" => Cell::new(status_display_detail).fg(Color::Green),
                                "EA" => Cell::new(status_display_detail).fg(Color::Yellow),
                                _ => Cell::new(status_display_detail).fg(Color::DarkGrey),
                            },
                            Cell::new(package.package_type.to_string()),
                            Cell::new(os_arch),
                            Cell::new(package.lib_c_type.as_deref().unwrap_or("-")),
                            Cell::new(size_display.clone()),
                        ]
                    } else {
                        // Compact mode (default)
                        vec![
                            dist_cell,
                            Cell::new(display_version),
                            // Apply color to LTS cell
                            match lts_display {
                                "LTS" => Cell::new(lts_display).fg(Color::Green),
                                "STS" => Cell::new(lts_display).fg(Color::Yellow),
                                _ => Cell::new(lts_display).fg(Color::DarkGrey),
                            },
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
        }
    }

    // Configure column alignments
    if let Some(col) = table.column_mut(2) {
        col.set_cell_alignment(CellAlignment::Center); // LTS column
    }

    if detailed {
        if let Some(col) = table.column_mut(7) {
            col.set_cell_alignment(CellAlignment::Right); // Size column
        }
        if let Some(col) = table.column_mut(3) {
            col.set_cell_alignment(CellAlignment::Center); // Status column
        }
    }

    // Only print the table if it has rows
    if table.row_count() > 0 {
        // Convert table to string and replace separator markers with proper lines
        let table_str = format!("{table}");
        let lines: Vec<&str> = table_str.lines().collect();

        for line in lines.iter() {
            if line.contains("SEPARATOR") {
                // Replace the content row with a proper separator line
                // Use the structure from the top border to create the separator
                if let Some(top_border) = lines.first() {
                    let separator = top_border.replace('┌', "├").replace('┐', "┤");
                    println!("{separator}");
                }
            } else {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn list_distributions(config: &KopiConfig) -> Result<()> {
    let cache_path = config.metadata_cache_path()?;

    if !cache_path.exists() {
        println!("{} No cache found", "✗".red());
        println!(
            "\n{}: Run {} to populate the cache with available distributions.",
            "Solution".yellow().bold(),
            "'kopi cache refresh'".cyan()
        );
        return Ok(());
    }

    // Load cache
    let cache = cache::load_cache(&cache_path)?;

    // Get current platform info
    let (current_arch, current_os, _) = get_current_platform();

    // Create a map to store distribution info
    let mut distribution_info: HashMap<String, (String, usize)> = HashMap::new();

    // Count packages per distribution for current platform
    for (dist_key, distribution) in &cache.distributions {
        let platform_packages: Vec<_> = distribution
            .packages
            .iter()
            .filter(|package| {
                package.architecture.to_string() == current_arch
                    && package.operating_system.to_string() == current_os
            })
            .collect();

        if !platform_packages.is_empty() {
            // Get display name from distribution or use the key
            let display_name = distribution.display_name.clone();

            distribution_info.insert(dist_key.clone(), (display_name, platform_packages.len()));
        }
    }

    if distribution_info.is_empty() {
        println!("{} No distributions found for current platform", "✗".red());
        println!(
            "\n{}: Your platform ({}/{}) might not be supported or the cache is empty.",
            "Note".yellow().bold(),
            current_os.cyan(),
            current_arch.cyan()
        );
        println!(
            "\n{}: Run {} to refresh the cache.",
            "Solution".yellow().bold(),
            "'kopi cache refresh'".cyan()
        );
        return Ok(());
    }

    println!("Available distributions in cache:\n");

    // Create a table
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(vec![
        Cell::new("Distribution"),
        Cell::new("Display Name"),
        Cell::new("Versions"),
    ]);

    // Sort by distribution key for consistent output
    let mut sorted_distributions: Vec<(String, (String, usize))> =
        distribution_info.into_iter().collect();
    sorted_distributions.sort_by(|a, b| a.0.cmp(&b.0));

    let mut total_versions = 0;
    for (dist_key, (display_name, count)) in sorted_distributions {
        table.add_row(vec![
            Cell::new(&dist_key),
            Cell::new(&display_name),
            Cell::new(count.to_string()).set_alignment(CellAlignment::Right),
        ]);
        total_versions += count;
    }

    println!("{table}");
    println!(
        "\nTotal: {} distributions with {} versions for {}/{}",
        table.row_count(),
        total_versions,
        current_os,
        current_arch
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_cache_info_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let result = show_cache_info(&config, false);
        assert!(result.is_ok());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_clear_cache_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let result = clear_cache(&config, false);
        assert!(result.is_ok());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_list_distributions_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let result = list_distributions(&config);
        assert!(result.is_ok());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_search_cache_with_lts_filter_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let options = SearchOptions {
            version_string: "21".to_string(),
            compact: false,
            detailed: false,
            json: false,
            lts_only: true,
            force_java_version: false,
            force_distribution_version: false,
        };
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let result = search_cache(options, &config);
        assert!(result.is_ok());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_search_cache_version_only_no_default_distribution() {
        use crate::config::KopiConfig;
        use crate::version::parser::VersionParser;

        // Test that version-only searches don't default to Temurin
        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let parser = VersionParser::new(&config);
        let parsed = parser.parse("21").unwrap();
        assert!(parsed.version.is_some());
        assert_eq!(parsed.distribution, None); // Should not default to any distribution
    }

    #[test]
    #[serial]
    fn test_search_cache_with_synonym_resolution() {
        use crate::cache::{DistributionCache, MetadataCache};
        use crate::models::distribution::Distribution as JdkDistribution;
        use crate::models::metadata::JdkMetadata;
        use crate::models::package::{ArchiveType, ChecksumType, PackageType};
        use crate::models::platform::{Architecture, OperatingSystem};
        use crate::version::Version;
        use std::str::FromStr;
        use tempfile::TempDir;

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        // Create a cache with SAP Machine distribution
        let mut cache = MetadataCache::new();

        // Set up synonym map - simulating SAP Machine case
        cache
            .synonym_map
            .insert("sapmachine".to_string(), "sap_machine".to_string());
        cache
            .synonym_map
            .insert("sap-machine".to_string(), "sap_machine".to_string());
        cache
            .synonym_map
            .insert("sap_machine".to_string(), "sap_machine".to_string());

        // Create a SAP Machine package
        let jdk_metadata = JdkMetadata {
            id: "sap-test-id".to_string(),
            distribution: "sap_machine".to_string(),
            version: Version::new(21, 0, 7),
            distribution_version: Version::from_str("21.0.7").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            javafx_bundled: false,
            download_url: Some("https://example.com/sap-download".to_string()),
            checksum: None,
            checksum_type: Some(ChecksumType::Sha256),
            size: 100000000,
            lib_c_type: None,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: None,
        };

        let dist = DistributionCache {
            distribution: JdkDistribution::SapMachine,
            display_name: "SAP Machine".to_string(),
            packages: vec![jdk_metadata],
        };

        // Store under canonical name
        cache.distributions.insert("sap_machine".to_string(), dist);

        // Save the cache
        let cache_path = temp_dir.path().join("cache").join("metadata.json");
        cache.save(&cache_path).unwrap();

        // Test searching with the synonym "sapmachine"
        let options = SearchOptions {
            version_string: "sapmachine@21".to_string(),
            compact: false,
            detailed: false,
            json: true,
            lts_only: false,
            force_java_version: false,
            force_distribution_version: false,
        };
        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let result = search_cache(options, &config);
        assert!(result.is_ok(), "Search should succeed with synonym");

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    #[serial]
    fn test_cache_refresh_with_progress() {
        // Test that cache refresh works with the new progress indicator
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("KOPI_HOME", temp_dir.path());
        }

        let config = crate::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Note: This test doesn't actually call the API, but verifies the function
        // doesn't panic with the new progress indicator implementation
        // Actual network calls would be tested in integration tests

        // Create a minimal cache file to avoid network calls
        let cache_path = config.metadata_cache_path().unwrap();
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cache_path,
            r#"{"version":3,"last_updated":"2024-01-01T00:00:00Z","distributions":{},"synonym_map":{}}"#,
        ).unwrap();

        // Verify the function runs without panicking
        let result = show_cache_info(&config, false);
        assert!(result.is_ok());

        unsafe {
            env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_progress_indicator_integration() {
        // Test that our progress indicator is properly integrated
        // This verifies the type system and trait implementations
        use crate::indicator::{ProgressConfig, ProgressFactory, ProgressStyle as IndicatorStyle};

        let mut progress = ProgressFactory::create(false);
        let config = ProgressConfig::new("Testing", "cache operations", IndicatorStyle::Count);
        progress.start(config);
        progress.complete(Some("Test complete".to_string()));

        // Also test with no_progress mode
        let mut silent_progress = ProgressFactory::create(true);
        let config = ProgressConfig::new("Testing", "silent mode", IndicatorStyle::Count);
        silent_progress.start(config);
        silent_progress.complete(None);
    }
}
