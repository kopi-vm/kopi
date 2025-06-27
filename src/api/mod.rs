pub mod client;
pub mod models;
pub mod query;

#[cfg(test)]
mod tests;

// Re-export commonly used types
pub use client::ApiClient;
pub use models::{
    ApiMetadata, Distribution, DistributionMetadata, Links, MajorVersion, Package, PackageInfo,
};
pub use query::PackageQuery;
