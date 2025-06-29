//! Data models for package search operations.

use crate::models::jdk::JdkMetadata;

/// Platform-specific filters for narrowing search results.
///
/// All fields are optional, allowing flexible filtering combinations.
/// When a field is `None`, that filter is not applied.
///
/// # Examples
///
/// ```
/// # use kopi::search::PlatformFilter;
/// // Filter for Linux x64 with glibc
/// let filter = PlatformFilter {
///     architecture: Some("x64".to_string()),
///     operating_system: Some("linux".to_string()),
///     lib_c_type: Some("glibc".to_string()),
/// };
///
/// // Filter only by architecture
/// let arch_only = PlatformFilter {
///     architecture: Some("aarch64".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct PlatformFilter {
    /// Target architecture (e.g., "x64", "aarch64", "arm32")
    pub architecture: Option<String>,
    
    /// Target operating system (e.g., "linux", "windows", "macos")
    pub operating_system: Option<String>,
    
    /// C library type for Linux (e.g., "glibc", "musl")
    /// This is particularly important for Alpine Linux compatibility
    pub lib_c_type: Option<String>,
}

/// A search result containing full package information.
///
/// This struct owns all data and is suitable for returning from searches
/// where the results need to outlive the cache reference.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    /// Distribution identifier (e.g., "temurin", "zulu")
    pub distribution: String,
    
    /// Human-readable distribution name (e.g., "Eclipse Temurin", "Azul Zulu")
    pub display_name: String,
    
    /// Complete package metadata including version, download info, etc.
    pub package: JdkMetadata,
}

/// A search result containing references to cached data.
///
/// This struct borrows data from the cache and is more efficient when
/// the results don't need to outlive the cache reference. Useful for
/// internal operations where cloning can be avoided.
///
/// # Lifetime
///
/// The lifetime parameter `'a` is tied to the cache lifetime, ensuring
/// the references remain valid as long as the cache exists.
#[derive(Debug)]
pub struct SearchResultRef<'a> {
    /// Reference to distribution identifier
    pub distribution: &'a str,
    
    /// Reference to human-readable distribution name
    pub display_name: &'a str,
    
    /// Reference to package metadata
    pub package: &'a JdkMetadata,
}
