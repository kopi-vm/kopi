//! Package search functionality for finding JDK distributions.
//!
//! This module provides a comprehensive search system for finding JDK packages
//! from cached metadata. It supports various search strategies including:
//!
//! ## Search Strategies
//!
//! ### 1. Version-based Search
//! - Search by major version: `"21"` finds all JDK 21.x.x versions
//! - Search by specific version: `"21.0.1"` finds exact matches
//! - Pattern matching: Major version searches match all minor/patch versions
//!
//! ### 2. Distribution-based Search
//! - Search by distribution: `"temurin"` finds all Temurin packages
//! - Combined search: `"temurin@21"` finds Temurin JDK 21.x.x
//! - Distribution-only search returns all versions for that distribution
//!
//! ### 3. Latest Version Search
//! - Global latest: `"latest"` finds newest version across all distributions
//! - Distribution latest: `"temurin@latest"` finds newest Temurin version
//! - Version-filtered latest: When `latest` flag is set with version constraint
//!
//! ### 4. Platform Filtering
//! - Architecture filtering: x64, aarch64, arm32, etc.
//! - Operating system filtering: linux, windows, macos, etc.
//! - libc type filtering: glibc, musl (important for Linux compatibility)
//!
//! ### 5. Package Type Filtering
//! - JDK vs JRE selection
//! - Default preference: JDK over JRE when both available
//! - Explicit selection: `jre@21` or `jdk@21` prefix
//!
//! ## Auto-selection Strategy
//!
//! When multiple packages match the criteria (common for same version/platform),
//! the auto-selection follows this priority:
//!
//! 1. **Package type preference**:
//!    - If type requested explicitly, filter to that type
//!    - Otherwise, prefer JDK over JRE
//!
//! 2. **libc compatibility** (Linux only):
//!    - Match system libc type when possible
//!    - Falls back to first available if no exact match
//!
//! 3. **First match fallback**:
//!    - Returns the first package if no other criteria differentiate
//!
//! ## Usage Examples
//!
//! ```no_run
//! use kopi::search::{PackageSearcher, PlatformFilter};
//!
//! // Simple version search
//! let searcher = PackageSearcher::new(Some(&cache));
//! let results = searcher.search("21")?;
//!
//! // Platform-filtered search
//! let filter = PlatformFilter {
//!     architecture: Some("x64".to_string()),
//!     operating_system: Some("linux".to_string()),
//!     lib_c_type: Some("glibc".to_string()),
//! };
//! let searcher = PackageSearcher::new(Some(&cache))
//!     .with_platform_filter(filter);
//! let results = searcher.search("17")?;
//! ```

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

/// Load cache and create a searcher
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
