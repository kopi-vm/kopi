pub mod client;
pub mod query;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub result: Vec<serde_json::Value>,
    pub message: String,
}

// Re-export API client types
pub use client::ApiClient;
pub use query::PackageQuery;
