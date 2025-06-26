use crate::cache;
use crate::error::Result;
use chrono::Local;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// Refresh metadata from foojay.io API
    Refresh,
    /// Show cache information
    Info,
    /// Clear all cached data
    Clear,
}

impl CacheCommand {
    pub fn execute(self) -> Result<()> {
        match self {
            CacheCommand::Refresh => refresh_cache(),
            CacheCommand::Info => show_cache_info(),
            CacheCommand::Clear => clear_cache(),
        }
    }
}

fn refresh_cache() -> Result<()> {
    println!("Refreshing metadata cache from foojay.io...");

    let cache = cache::fetch_and_cache_metadata()?;

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
