mod models;
mod searcher;

#[cfg(test)]
mod tests;

// Re-export commonly used types
pub use models::{PlatformFilter, SearchResult, SearchResultRef, VersionSearchType};
pub use searcher::PackageSearcher;

// Re-export platform functions from the main platform module for convenience
pub use crate::platform::{get_current_architecture, get_current_os, get_current_platform};
