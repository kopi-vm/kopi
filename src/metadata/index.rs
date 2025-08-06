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

use crate::metadata::GeneratorConfig;
use serde::{Deserialize, Serialize};

/// Index file structure for metadata repository
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexFile {
    pub version: u32,
    pub updated: String,
    pub files: Vec<IndexFileEntry>,
    /// Generator configuration used to create this metadata (added in version 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator_config: Option<GeneratorConfig>,
}

/// Entry in the index file describing a metadata file
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexFileEntry {
    pub path: String,
    pub distribution: String,
    pub architectures: Option<Vec<String>>,
    pub operating_systems: Option<Vec<String>>,
    pub lib_c_types: Option<Vec<String>>,
    pub size: u64,
    pub checksum: Option<String>,
    pub last_modified: Option<String>,
}
