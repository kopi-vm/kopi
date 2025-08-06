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

use serde::{Deserialize, Serialize};

use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::version::Version;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    pub distribution_version: Version,
    pub architecture: Architecture,
    pub operating_system: OperatingSystem,
    pub package_type: PackageType,
    pub archive_type: ArchiveType,

    // Lazy-loaded fields (may be None if not yet loaded from foojay)
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,

    pub size: i64,
    pub lib_c_type: Option<String>,
    pub javafx_bundled: bool,
    pub term_of_support: Option<String>,
    pub release_status: Option<String>,
    pub latest_build_available: Option<bool>,
}

impl JdkMetadata {
    /// Check if the metadata has all required fields for installation
    pub fn is_complete(&self) -> bool {
        // Only require download_url to be present
        // Checksum is optional - if not present, download will proceed without verification
        self.download_url.is_some()
    }
}
