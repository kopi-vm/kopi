use crate::models::metadata::JdkMetadata;

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

#[derive(Debug)]
pub struct SearchResultRef<'a> {
    pub distribution: &'a str,
    pub display_name: &'a str,
    pub package: &'a JdkMetadata,
}
