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

use crate::models::metadata::JdkMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSearchType {
    JavaVersion,
    DistributionVersion,
    Auto,
}

impl Default for VersionSearchType {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlatformFilter {
    pub architecture: Option<String>,
    pub operating_system: Option<String>,
    pub lib_c_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub distribution: String,
    pub display_name: String,
    pub package: JdkMetadata,
}
