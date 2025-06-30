use crate::cache::{MetadataCache, get_cache_path, load_cache};
use crate::error::Result;

mod models;
mod searcher;

#[cfg(test)]
mod tests;

// Re-export commonly used types
pub use models::{PlatformFilter, SearchResult, SearchResultRef};
pub use searcher::PackageSearcher;

// Re-export platform functions from the main platform module for convenience
pub use crate::platform::{get_current_architecture, get_current_os, get_current_platform};

pub fn create_searcher_with_cache() -> Result<(MetadataCache, PackageSearcher<'static>)> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok((MetadataCache::new(), PackageSearcher::new(None)));
    }

    let cache = load_cache(&cache_path)?;
    // This is a bit tricky - we need to ensure the cache outlives the searcher
    // In practice, the caller will need to manage this lifetime
    Ok((cache, PackageSearcher::new(None)))
}
