use serde::{Deserialize, Serialize};

use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::version::Version;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    pub distribution_version: String,
    pub architecture: Architecture,
    pub operating_system: OperatingSystem,
    pub package_type: PackageType,
    pub archive_type: ArchiveType,
    pub download_url: String,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
    pub size: u64,
    pub lib_c_type: Option<String>,
    pub javafx_bundled: bool,
    pub term_of_support: Option<String>,
    pub release_status: Option<String>,
    pub latest_build_available: Option<bool>,
}
