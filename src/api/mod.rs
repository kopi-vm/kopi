use crate::error::{KopiError, Result};
use attohttpc::{RequestBuilder, Session};
use retry::{OperationResult, delay::Exponential, retry_with_index};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const FOOJAY_API_BASE: &str = "https://api.foojay.io/disco";
const API_VERSIONS: &[&str] = &["v3.0", "v2.0", "v1.0"];
const USER_AGENT: &str = concat!("kopi/", env!("CARGO_PKG_VERSION"));
const DEFAULT_TIMEOUT: u64 = 30;
const MAX_RETRIES: usize = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct ApiClient {
    session: Session,
    base_url: String,
    api_version: Option<String>,
}

impl ApiClient {
    pub fn new() -> Self {
        let mut session = Session::new();
        session.header("User-Agent", USER_AGENT);
        session.timeout(Duration::from_secs(DEFAULT_TIMEOUT));

        Self {
            session,
            base_url: FOOJAY_API_BASE.to_string(),
            api_version: None,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.session.timeout(timeout);
        self
    }

    pub fn get_packages(&self, query: Option<PackageQuery>) -> Result<Vec<Package>> {
        self.execute_with_version_fallback(|version| {
            let url = format!("{}/{}/packages", self.base_url, version);
            let query = query.clone();

            self.execute_with_retry(move || {
                let mut request = self.session.get(&url);

                // Build query parameters for logging
                let mut query_params = Vec::new();

                if let Some(ref q) = query {
                    if let Some(ref version) = q.version {
                        request = request.param("version", version);
                        query_params.push(format!("version={}", version));
                    }
                    if let Some(ref distribution) = q.distribution {
                        request = request.param("distribution", distribution);
                        query_params.push(format!("distribution={}", distribution));
                    }
                    if let Some(ref architecture) = q.architecture {
                        request = request.param("architecture", architecture);
                        query_params.push(format!("architecture={}", architecture));
                    }
                    if let Some(ref package_type) = q.package_type {
                        request = request.param("package_type", package_type);
                        query_params.push(format!("package_type={}", package_type));
                    }
                    if let Some(ref operating_system) = q.operating_system {
                        request = request.param("operating_system", operating_system);
                        query_params.push(format!("operating_system={}", operating_system));
                    }
                    if let Some(ref archive_type) = q.archive_type {
                        request = request.param("archive_type", archive_type);
                        query_params.push(format!("archive_type={}", archive_type));
                    }
                    if let Some(latest) = q.latest {
                        request = request.param("latest", latest.to_string());
                        query_params.push(format!("latest={}", latest));
                    }
                    if let Some(directly_downloadable) = q.directly_downloadable {
                        request = request
                            .param("directly_downloadable", directly_downloadable.to_string());
                        query_params
                            .push(format!("directly_downloadable={}", directly_downloadable));
                    }
                    if let Some(ref lib_c_type) = q.lib_c_type {
                        request = request.param("lib_c_type", lib_c_type);
                        query_params.push(format!("lib_c_type={}", lib_c_type));
                    }

                    // Log the complete URL with query parameters
                    let full_url = if query_params.is_empty() {
                        url.clone()
                    } else {
                        format!("{}?{}", url, query_params.join("&"))
                    };
                    eprintln!("API Request: {}", full_url);
                }

                request
            })
        })
    }

    pub fn get_distributions(&self) -> Result<Vec<Distribution>> {
        self.execute_with_version_fallback(|version| {
            let url = format!("{}/{}/distributions", self.base_url, version);
            self.execute_with_retry(move || self.session.get(&url))
        })
    }

    pub fn get_major_versions(&self) -> Result<Vec<MajorVersion>> {
        self.execute_with_version_fallback(|version| {
            let url = format!("{}/{}/major_versions", self.base_url, version);
            self.execute_with_retry(move || self.session.get(&url))
        })
    }

    fn execute_with_version_fallback<T, F>(&self, mut operation: F) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        F: FnMut(&str) -> Result<T>,
    {
        // If we have a cached working version, try it first
        if let Some(ref version) = self.api_version {
            match operation(version) {
                Ok(result) => return Ok(result),
                Err(_) => {
                    // Cached version failed, try others
                }
            }
        }

        // Try each API version in order
        let mut last_error = None;
        for version in API_VERSIONS {
            eprintln!("Trying API version: {}", version);
            match operation(version) {
                Ok(result) => {
                    eprintln!("API version {} succeeded", version);
                    // Cache the working version for future requests
                    // Note: In a real implementation, we'd want to make this mutable
                    // or use interior mutability
                    return Ok(result);
                }
                Err(e) => {
                    eprintln!("API version {} failed: {:?}", version, e);
                    last_error = Some(e);
                }
            }
        }

        // All versions failed, return the last error
        Err(last_error.unwrap_or_else(|| {
            KopiError::MetadataFetch(
                "Failed to connect to foojay.io API. All API versions were tried without success. \
                 Please check your internet connection or try again later."
                    .to_string(),
            )
        }))
    }

    fn execute_with_retry<T, F>(&self, request_builder: F) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        F: Fn() -> RequestBuilder,
    {
        let result = retry_with_index(
            Exponential::from_millis(INITIAL_BACKOFF_MS).take(MAX_RETRIES),
            |current_try| {
                let response = match request_builder().send() {
                    Ok(resp) => resp,
                    Err(e) => {
                        let user_error = KopiError::MetadataFetch(format!(
                            "Network error: {}. Please check your internet connection and try again.",
                            e
                        ));

                        if current_try < (MAX_RETRIES - 1) as u64 {
                            return OperationResult::Retry(user_error);
                        }
                        return OperationResult::Err(user_error);
                    }
                };

                if response.status() == attohttpc::StatusCode::TOO_MANY_REQUESTS
                    && current_try < (MAX_RETRIES - 1) as u64
                {
                    if let Some(retry_after) = response.headers().get("Retry-After") {
                        if let Ok(retry_str) = retry_after.to_str() {
                            if let Ok(seconds) = retry_str.parse::<u64>() {
                                std::thread::sleep(Duration::from_secs(seconds));
                            }
                        }
                    }
                    return OperationResult::Retry(KopiError::MetadataFetch(
                        "Too many requests. Waiting before retrying...".to_string(),
                    ));
                }

                if !response.is_success() {
                    let status = response.status();
                    let error_msg = match status.as_u16() {
                        404 => "The requested resource was not found. The API endpoint may have changed.".to_string(),
                        500..=599 => "Server error occurred. Please try again later.".to_string(),
                        401 | 403 => "Authentication failed. Please check your credentials.".to_string(),
                        _ => format!("HTTP error ({}): {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown error")),
                    };
                    return OperationResult::Err(KopiError::MetadataFetch(error_msg));
                }

                // Try to get response text for debugging
                match response.text() {
                    Ok(body) => {
                        // First try to parse as a JSON value to handle wrapped responses
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json_value) => {
                                // Check if response is wrapped with "result" field
                                if let Some(result) = json_value.get("result") {
                                    // Try to deserialize the result field
                                    match serde_json::from_value::<T>(result.clone()) {
                                        Ok(data) => OperationResult::Ok(data),
                                        Err(e) => {
                                            eprintln!("Failed to parse 'result' field: {}", e);
                                            eprintln!("Result field: {:?}", result);
                                            OperationResult::Err(KopiError::MetadataFetch(format!(
                                                "Failed to parse wrapped response: {}",
                                                e
                                            )))
                                        }
                                    }
                                } else {
                                    // No "result" field, try to parse the entire value as T
                                    match serde_json::from_value::<T>(json_value) {
                                        Ok(data) => OperationResult::Ok(data),
                                        Err(e) => {
                                            eprintln!("JSON parse error: {}", e);
                                            eprintln!(
                                                "Response body (first 500 chars): {}",
                                                &body.chars().take(500).collect::<String>()
                                            );
                                            OperationResult::Err(KopiError::MetadataFetch(format!(
                                                "Failed to parse response: {}",
                                                e
                                            )))
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse as JSON: {}", e);
                                OperationResult::Err(KopiError::MetadataFetch(format!(
                                    "Invalid JSON response: {}",
                                    e
                                )))
                            }
                        }
                    }
                    Err(e) => OperationResult::Err(KopiError::MetadataFetch(format!(
                        "Failed to read response body: {}",
                        e
                    ))),
                }
            },
        );

        result.map_err(|e| e.error)
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// Wrapper for v3.0 API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiV3Response<T> {
    result: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PackageQuery {
    pub version: Option<String>,
    pub distribution: Option<String>,
    pub architecture: Option<String>,
    pub package_type: Option<String>,
    pub operating_system: Option<String>,
    pub archive_type: Option<String>,
    pub latest: Option<bool>,
    pub directly_downloadable: Option<bool>,
    pub lib_c_type: Option<String>,
}

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
    pub id: String,
    pub name: String,
    pub api_parameter: String,
    pub free_use_in_production: bool,
    pub synonyms: Vec<String>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorVersion {
    pub major_version: u32,
    pub term_of_support: String,
    pub versions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new();
        assert_eq!(client.base_url, FOOJAY_API_BASE);
        assert!(client.api_version.is_none());
    }

    #[test]
    fn test_api_client_with_custom_base_url() {
        let custom_url = "https://test.example.com";
        let client = ApiClient::new().with_base_url(custom_url.to_string());
        assert_eq!(client.base_url, custom_url);
    }

    #[test]
    fn test_package_query_builder() {
        let query = PackageQuery {
            version: Some("21".to_string()),
            distribution: Some("temurin".to_string()),
            architecture: Some("x64".to_string()),
            package_type: Some("jdk".to_string()),
            operating_system: Some("linux".to_string()),
            archive_type: Some("tar.gz".to_string()),
            latest: Some(true),
            directly_downloadable: Some(true),
            lib_c_type: None,
        };

        assert_eq!(query.version, Some("21".to_string()));
        assert_eq!(query.distribution, Some("temurin".to_string()));
    }
}
