use serde::{Deserialize, Serialize};

/// Index file structure for metadata repository
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexFile {
    pub version: u32,
    pub updated: String,
    pub files: Vec<IndexFileEntry>,
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
