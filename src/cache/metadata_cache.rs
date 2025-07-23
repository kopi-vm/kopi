use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::error::Result;
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataCache {
    pub version: u32,
    pub last_updated: DateTime<Utc>,
    pub distributions: HashMap<String, DistributionCache>,
    /// Maps distribution synonyms to their canonical api_parameter names
    #[serde(default)]
    pub synonym_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistributionCache {
    pub distribution: JdkDistribution,
    pub display_name: String,
    pub packages: Vec<JdkMetadata>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            distributions: HashMap::new(),
            synonym_map: HashMap::new(),
        }
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataCache {
    /// Check if the cache is stale based on the given maximum age
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_updated);

        // Convert chrono::Duration to std::time::Duration for comparison
        match elapsed.to_std() {
            Ok(std_duration) => std_duration > max_age,
            Err(_) => true, // If time went backwards or conversion failed, consider stale
        }
    }

    pub fn has_version(&self, version: &str) -> bool {
        for dist in self.distributions.values() {
            for package in &dist.packages {
                if package.version.to_string() == version {
                    return true;
                }
            }
        }
        false
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        super::storage::save_cache(self, path)
    }

    /// Get the canonical name for a distribution from the synonym map
    /// Returns None if not found
    pub fn get_canonical_name(&self, name: &str) -> Option<&str> {
        self.synonym_map.get(name).map(|s| s.as_str())
    }
}
