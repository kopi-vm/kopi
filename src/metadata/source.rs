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

use crate::error::Result;
use crate::indicator::ProgressIndicator;
use crate::models::metadata::JdkMetadata;
use crate::models::package::ChecksumType;

/// Trait for metadata sources (synchronous)
pub trait MetadataSource: Send + Sync {
    /// Get a unique identifier for this source
    fn id(&self) -> &str;

    /// Get a human-readable name for this source
    fn name(&self) -> &str;

    /// Check if the source is available and can be accessed
    fn is_available(&self) -> Result<bool>;

    /// Fetch all available metadata from this source
    /// For foojay: returns metadata with is_complete=false
    /// For local/GitHub: returns metadata with is_complete=true
    fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>>;

    /// Fetch metadata for a specific distribution
    fn fetch_distribution(
        &self,
        distribution: &str,
        progress: &mut dyn ProgressIndicator,
    ) -> Result<Vec<JdkMetadata>>;

    /// Fetch complete details for a specific package (used by MetadataProvider)
    /// Only needed for sources that return incomplete metadata
    fn fetch_package_details(
        &self,
        package_id: &str,
        progress: &mut dyn ProgressIndicator,
    ) -> Result<PackageDetails>;

    /// Get the last update time of the source (if applicable)
    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>>;
}

/// Details fetched for lazy-loaded fields
#[derive(Debug, Clone)]
pub struct PackageDetails {
    pub download_url: String,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
}
