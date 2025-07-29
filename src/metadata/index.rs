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
