use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub archive_type: String,
    pub distribution: String,
    pub major_version: u32,
    pub java_version: String,
    pub distribution_version: String,
    pub jdk_version: u32,
    pub directly_downloadable: bool,
    pub filename: String,
    pub links: Links,
    pub free_use_in_production: bool,
    pub tck_tested: String,
    pub size: u64,
    pub operating_system: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib_c_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    pub pkg_download_redirect: String,
    pub pkg_info_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub api_parameter: String,
    pub maintained: bool,
    pub available: bool,
    pub build_of_openjdk: bool,
    pub build_of_graalvm: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub official_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_use_in_production: Option<bool>,
    pub synonyms: Vec<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorVersion {
    pub major_version: u32,
    pub term_of_support: String,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetadata {
    pub distributions: Vec<DistributionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetadata {
    pub distribution: Distribution,
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub filename: String,
    pub direct_download_uri: String,
    pub download_site_uri: Option<String>,
    pub checksum: String,
    pub checksum_type: String,
    pub checksum_uri: String,
    pub signature_uri: Option<String>,
}
