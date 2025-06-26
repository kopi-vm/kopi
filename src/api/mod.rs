use crate::error::{KopiError, Result};
use attohttpc::{RequestBuilder, Session};
use log::{debug, trace};
use retry::{OperationResult, delay::Exponential, retry_with_index};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

const FOOJAY_API_BASE: &str = "https://api.foojay.io/disco";
const API_VERSION: &str = "v3.0";
const USER_AGENT: &str = concat!("kopi/", env!("CARGO_PKG_VERSION"));
const DEFAULT_TIMEOUT: u64 = 30;
const MAX_RETRIES: usize = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct ApiClient {
    session: Session,
    base_url: String,
}

impl ApiClient {
    pub fn new() -> Self {
        let mut session = Session::new();
        session.header("User-Agent", USER_AGENT);
        session.timeout(Duration::from_secs(DEFAULT_TIMEOUT));

        Self {
            session,
            base_url: FOOJAY_API_BASE.to_string(),
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

    pub fn fetch_all_metadata(&self) -> Result<ApiMetadata> {
        // Fetch distributions
        let distributions = self.get_distributions()?;

        // For each distribution, fetch available packages
        let mut metadata = ApiMetadata {
            distributions: Vec::new(),
        };

        for dist in distributions {
            let query = PackageQuery {
                distribution: Some(dist.api_parameter.clone()),
                latest: Some("available".to_string()),
                directly_downloadable: Some(true),
                ..Default::default()
            };

            let packages = self.get_packages(Some(query))?;

            metadata.distributions.push(DistributionMetadata {
                distribution: dist,
                packages,
            });
        }

        Ok(metadata)
    }

    pub fn get_packages(&self, query: Option<PackageQuery>) -> Result<Vec<Package>> {
        let url = format!("{}/{}/packages", self.base_url, API_VERSION);
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
                if let Some(ref latest) = q.latest {
                    request = request.param("latest", latest);
                    query_params.push(format!("latest={}", latest));
                }
                if let Some(directly_downloadable) = q.directly_downloadable {
                    request =
                        request.param("directly_downloadable", directly_downloadable.to_string());
                    query_params.push(format!("directly_downloadable={}", directly_downloadable));
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
                debug!("API Request: {}", full_url);
            }

            request
        })
    }

    pub fn get_distributions(&self) -> Result<Vec<Distribution>> {
        let url = format!("{}/{}/distributions", self.base_url, API_VERSION);
        self.execute_with_retry(move || self.session.get(&url))
    }

    pub fn get_major_versions(&self) -> Result<Vec<MajorVersion>> {
        let url = format!("{}/{}/major_versions", self.base_url, API_VERSION);
        self.execute_with_retry(move || self.session.get(&url))
    }

    pub fn get_package_by_id(&self, package_id: &str) -> Result<PackageInfo> {
        // Special handling for package by ID endpoint which returns an array
        let url = format!("{}/{}/ids/{}", self.base_url, API_VERSION, package_id);
        debug!("Fetching package info for ID: {}", package_id);
        let package_id_copy = package_id.to_string();

        // Use the common retry logic but handle the array response
        self.execute_with_retry_raw(
            move || self.session.get(&url),
            move |body| match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(json_value) => {
                    // API v3.0 always wraps responses with "result" field
                    if let Some(result) = json_value.get("result") {
                        // The result is an array of PackageInfo
                        match serde_json::from_value::<Vec<PackageInfo>>(result.clone()) {
                            Ok(packages) => {
                                if let Some(package) = packages.into_iter().next() {
                                    Ok(package)
                                } else {
                                    Err(KopiError::MetadataFetch(format!(
                                        "No package info found for ID: {} (API v{})",
                                        package_id_copy, API_VERSION
                                    )))
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse 'result' field as array: {}", e);
                                trace!("Result field: {:?}", result);
                                Err(KopiError::MetadataFetch(format!(
                                    "Failed to parse API v{} response: {}",
                                    API_VERSION, e
                                )))
                            }
                        }
                    } else {
                        Err(KopiError::MetadataFetch(format!(
                            "Invalid API v{} response: missing 'result' field",
                            API_VERSION
                        )))
                    }
                }
                Err(e) => {
                    debug!("Failed to parse as JSON: {}", e);
                    Err(KopiError::MetadataFetch(format!(
                        "Invalid JSON response from API v{}: {}",
                        API_VERSION, e
                    )))
                }
            },
        )
    }

    fn execute_with_retry<T, F>(&self, request_builder: F) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        F: Fn() -> RequestBuilder,
    {
        self.execute_with_retry_raw(request_builder, |body| {
            // Parse JSON response
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(json_value) => {
                    // API v3.0 always wraps responses with "result" field
                    if let Some(result) = json_value.get("result") {
                        // Try to deserialize the result field
                        match serde_json::from_value::<T>(result.clone()) {
                            Ok(data) => Ok(data),
                            Err(e) => {
                                debug!("Failed to parse 'result' field: {}", e);
                                trace!("Result field: {:?}", result);
                                Err(KopiError::MetadataFetch(format!(
                                    "Failed to parse API v{} response: {}",
                                    API_VERSION, e
                                )))
                            }
                        }
                    } else {
                        Err(KopiError::MetadataFetch(format!(
                            "Invalid API v{} response: missing 'result' field",
                            API_VERSION
                        )))
                    }
                }
                Err(e) => {
                    debug!("Failed to parse as JSON: {}", e);
                    Err(KopiError::MetadataFetch(format!(
                        "Invalid JSON response from API v{}: {}",
                        API_VERSION, e
                    )))
                }
            }
        })
    }

    fn execute_with_retry_raw<T, F, P>(&self, request_builder: F, parser: P) -> Result<T>
    where
        F: Fn() -> RequestBuilder,
        P: Fn(String) -> Result<T>,
    {
        let result = retry_with_index(
            Exponential::from_millis(INITIAL_BACKOFF_MS).take(MAX_RETRIES),
            |current_try| {
                let response = match request_builder().send() {
                    Ok(resp) => resp,
                    Err(e) => {
                        let user_error = KopiError::MetadataFetch(format!(
                            "Network error connecting to foojay.io API v{}: {}. Please check your internet connection and try again.",
                            API_VERSION, e
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
                                thread::sleep(Duration::from_secs(seconds));
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
                        404 => format!(
                            "The requested resource was not found on foojay.io API v{}. The API endpoint may have changed.",
                            API_VERSION
                        ),
                        500..=599 => format!(
                            "Server error occurred on foojay.io API v{}. Please try again later.",
                            API_VERSION
                        ),
                        401 | 403 => format!(
                            "Authentication failed for foojay.io API v{}. Please check your credentials.",
                            API_VERSION
                        ),
                        _ => format!(
                            "HTTP error ({}) from foojay.io API v{}: {}",
                            status.as_u16(),
                            API_VERSION,
                            status.canonical_reason().unwrap_or("Unknown error")
                        ),
                    };
                    return OperationResult::Err(KopiError::MetadataFetch(error_msg));
                }

                // Try to get response text for debugging
                match response.text() {
                    Ok(body) => match parser(body) {
                        Ok(data) => OperationResult::Ok(data),
                        Err(e) => OperationResult::Err(e),
                    },
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
    pub latest: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new();
        assert_eq!(client.base_url, FOOJAY_API_BASE);
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
            latest: Some("per_version".to_string()),
            directly_downloadable: Some(true),
            lib_c_type: None,
        };

        assert_eq!(query.version, Some("21".to_string()));
        assert_eq!(query.distribution, Some("temurin".to_string()));
    }

    #[test]
    fn test_package_info_structure() {
        // Test that PackageInfo can be deserialized from expected JSON structure
        let json = r#"{
            "filename": "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
            "direct_download_uri": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
            "download_site_uri": null,
            "checksum": "5a04c9d9e89e685e56b3780ebec4134c723f6e5e9495513a2f23bf5798a3e70f",
            "checksum_type": "sha256",
            "checksum_uri": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz.sha256.txt",
            "signature_uri": null
        }"#;

        let package_info: PackageInfo =
            serde_json::from_str(json).expect("Failed to parse PackageInfo");
        assert_eq!(
            package_info.filename,
            "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz"
        );
        assert_eq!(package_info.checksum_type, "sha256");
        assert_eq!(package_info.checksum.len(), 64); // SHA256 is 64 hex chars
    }

    #[test]
    fn test_parse_distributions_api_response() {
        // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/distributions
        let json_response = r#"{
  "result":
  [{
  "name":"ZuluPrime",
  "api_parameter":"zulu_prime",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://www.azul.com/products/prime/stream-download/",
  "synonyms": [
    "zing",
    "ZING",
    "Zing",
    "prime",
    "PRIME",
    "Prime",
    "zuluprime",
    "ZULUPRIME",
    "ZuluPrime",
    "zulu_prime",
    "ZULU_PRIME",
    "Zulu_Prime",
    "zulu prime",
    "ZULU PRIME",
    "Zulu Prime"
  ],
  "versions": [
    "23.0.2+7",
    "23.0.1+11",
    "21.0.7+6",
    "17.0.15+6",
    "11.0.27+6",
    "8.0.452+5"
  ]
},
{
  "name":"Zulu",
  "api_parameter":"zulu",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://www.azul.com/downloads/?package=jdk",
  "synonyms": [
    "zulu",
    "ZULU",
    "Zulu",
    "zulucore",
    "ZULUCORE",
    "ZuluCore",
    "zulu_core",
    "ZULU_CORE",
    "Zulu_Core",
    "zulu core",
    "ZULU CORE",
    "Zulu Core"
  ],
  "versions": [
    "25-ea+27",
    "24.0.1+9",
    "23.0.2+7",
    "22.0.2+9",
    "21.0.7+6",
    "17.0.15+21"
  ]
},
{
  "name":"Temurin",
  "api_parameter":"temurin",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://adoptium.net/temurin/releases",
  "synonyms": [
    "temurin",
    "Temurin",
    "TEMURIN"
  ],
  "versions": [
    "25-ea+28",
    "24.0.1+9",
    "23.0.2+7",
    "21.0.7+6",
    "17.0.15+21"
  ]
}]
}"#;

        // First test parsing the wrapped response
        let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
        let distributions: Vec<Distribution> =
            serde_json::from_value(wrapped_response["result"].clone()).unwrap();

        assert_eq!(distributions.len(), 3);
        assert_eq!(distributions[0].name, "ZuluPrime");
        assert_eq!(distributions[0].api_parameter, "zulu_prime");
        assert!(distributions[0].maintained);
        assert!(distributions[0].available);
        assert!(distributions[0].build_of_openjdk);
        assert!(!distributions[0].build_of_graalvm);

        assert_eq!(distributions[1].name, "Zulu");
        assert_eq!(distributions[1].api_parameter, "zulu");

        assert_eq!(distributions[2].name, "Temurin");
        assert_eq!(distributions[2].api_parameter, "temurin");
        assert!(distributions[2].versions.contains(&"21.0.7+6".to_string()));
    }

    #[test]
    fn test_parse_packages_api_response() {
        // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/packages?version=21&distribution=temurin&architecture=x64&package_type=jdk&operating_system=linux&latest=available
        let json_response = r#"{
"result":[
{"id":"4c4f879899012ff0a8b2e2117df03b0e","archive_type":"tar.gz","distribution":"temurin","major_version":21,"java_version":"21.0.7+6","distribution_version":"21.0.7","jdk_version":21,"latest_build_available":true,"release_status":"ga","term_of_support":"lts","operating_system":"linux","lib_c_type":"glibc","architecture":"x64","fpu":"unknown","package_type":"jdk","javafx_bundled":false,"directly_downloadable":true,"filename":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz","links":{"pkg_info_uri":"https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e","pkg_download_redirect":"https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e/redirect"},"free_use_in_production":true,"tck_tested":"unknown","tck_cert_uri":"","aqavit_certified":"unknown","aqavit_cert_uri":"","size":206919519,"feature":[]},
{"id":"6e5c475d47280d8ce27447611d50c645","archive_type":"tar.gz","distribution":"temurin","major_version":21,"java_version":"21.0.7+6","distribution_version":"21.0.7","jdk_version":21,"latest_build_available":true,"release_status":"ga","term_of_support":"lts","operating_system":"linux","lib_c_type":"musl","architecture":"x64","fpu":"unknown","package_type":"jdk","javafx_bundled":false,"directly_downloadable":true,"filename":"OpenJDK21U-jdk_x64_alpine-linux_hotspot_21.0.7_6.tar.gz","links":{"pkg_info_uri":"https://api.foojay.io/disco/v3.0/ids/6e5c475d47280d8ce27447611d50c645","pkg_download_redirect":"https://api.foojay.io/disco/v3.0/ids/6e5c475d47280d8ce27447611d50c645/redirect"},"free_use_in_production":true,"tck_tested":"unknown","tck_cert_uri":"","aqavit_certified":"unknown","aqavit_cert_uri":"","size":207113831,"feature":[]}
],
"message":"2 package(s) found"}"#;

        // Test parsing the wrapped response
        let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
        let packages: Vec<Package> =
            serde_json::from_value(wrapped_response["result"].clone()).unwrap();

        assert_eq!(packages.len(), 2);

        // Test first package (glibc)
        assert_eq!(packages[0].id, "4c4f879899012ff0a8b2e2117df03b0e");
        assert_eq!(packages[0].archive_type, "tar.gz");
        assert_eq!(packages[0].distribution, "temurin");
        assert_eq!(packages[0].major_version, 21);
        assert_eq!(packages[0].java_version, "21.0.7+6");
        assert_eq!(packages[0].distribution_version, "21.0.7");
        assert_eq!(packages[0].jdk_version, 21);
        assert!(packages[0].directly_downloadable);
        assert_eq!(
            packages[0].filename,
            "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
        );
        assert_eq!(packages[0].operating_system, "linux");
        assert_eq!(packages[0].lib_c_type, Some("glibc".to_string()));
        assert_eq!(packages[0].size, 206919519);
        assert!(packages[0].free_use_in_production);
        assert_eq!(
            packages[0].links.pkg_download_redirect,
            "https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e/redirect"
        );

        // Test second package (musl)
        assert_eq!(packages[1].id, "6e5c475d47280d8ce27447611d50c645");
        assert_eq!(packages[1].lib_c_type, Some("musl".to_string()));
        assert_eq!(
            packages[1].filename,
            "OpenJDK21U-jdk_x64_alpine-linux_hotspot_21.0.7_6.tar.gz"
        );
    }

    #[test]
    fn test_parse_package_info_by_id_response() {
        // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e
        let json_response = r#"{
  "result":[
    {
    "filename":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz",
    "direct_download_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz",
    "download_site_uri":"",
    "signature_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sig",
    "checksum_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sha256.txt",
    "checksum":"974d3acef0b7193f541acb61b76e81670890551366625d4f6ca01b91ac152ce0",
    "checksum_type":"sha256"
  }
    ],
  "message":""
}"#;

        // Test parsing the wrapped response
        let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
        let package_infos: Vec<PackageInfo> =
            serde_json::from_value(wrapped_response["result"].clone()).unwrap();

        assert_eq!(package_infos.len(), 1);
        let package_info = &package_infos[0];

        assert_eq!(
            package_info.filename,
            "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
        );
        assert_eq!(
            package_info.direct_download_uri,
            "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
        );
        assert_eq!(package_info.download_site_uri, Some("".to_string()));
        assert_eq!(package_info.signature_uri, Some("https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sig".to_string()));
        assert_eq!(
            package_info.checksum_uri,
            "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sha256.txt"
        );
        assert_eq!(
            package_info.checksum,
            "974d3acef0b7193f541acb61b76e81670890551366625d4f6ca01b91ac152ce0"
        );
        assert_eq!(package_info.checksum_type, "sha256");
        assert_eq!(package_info.checksum.len(), 64); // SHA256 is 64 hex chars
    }

    #[test]
    fn test_parse_major_versions_api_response() {
        // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/major_versions
        let json_response = r#"{
  "result":[
    {
      "major_version":24,
      "term_of_support":"STS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "24.0.1+11",
        "24.0.1+9",
        "24.0.1",
        "24+37",
        "24+36"
      ]
    },
    {
      "major_version":21,
      "term_of_support":"LTS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "21.0.7+6",
        "21.0.6+7",
        "21.0.5+11",
        "21.0.4+7",
        "21.0.3+9"
      ]
    },
    {
      "major_version":17,
      "term_of_support":"LTS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "17.0.15+21",
        "17.0.14+7",
        "17.0.13+11",
        "17.0.12+7",
        "17.0.11+9"
      ]
    }
  ]
}"#;

        // Test parsing the wrapped response
        let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
        let major_versions: Vec<MajorVersion> =
            serde_json::from_value(wrapped_response["result"].clone()).unwrap();

        assert_eq!(major_versions.len(), 3);

        // Test first version (24 - STS)
        assert_eq!(major_versions[0].major_version, 24);
        assert_eq!(major_versions[0].term_of_support, "STS");
        assert!(
            major_versions[0]
                .versions
                .contains(&"24.0.1+11".to_string())
        );

        // Test second version (21 - LTS)
        assert_eq!(major_versions[1].major_version, 21);
        assert_eq!(major_versions[1].term_of_support, "LTS");
        assert!(major_versions[1].versions.contains(&"21.0.7+6".to_string()));

        // Test third version (17 - LTS)
        assert_eq!(major_versions[2].major_version, 17);
        assert_eq!(major_versions[2].term_of_support, "LTS");
        assert!(
            major_versions[2]
                .versions
                .contains(&"17.0.15+21".to_string())
        );
    }
}
