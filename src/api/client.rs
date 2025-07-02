use crate::api::models::*;
use crate::api::query::PackageQuery;
use crate::error::{KopiError, Result};
use crate::platform::get_foojay_libc_type;
use attohttpc::{RequestBuilder, Session};
use log::{debug, trace};
use retry::{OperationResult, delay::Exponential, retry_with_index};
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
    pub(crate) session: Session,
    pub(crate) base_url: String,
}

impl ApiClient {
    pub fn new() -> Self {
        let mut session = Session::new();
        session.header("User-Agent", USER_AGENT);
        session.timeout(Duration::from_secs(DEFAULT_TIMEOUT));
        session.proxy_settings(attohttpc::ProxySettings::from_env());

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

        // Get platform-specific parameters
        let architecture = crate::platform::get_current_architecture();
        let operating_system = crate::platform::get_current_os();
        let lib_c_type = get_foojay_libc_type();

        // For each distribution, fetch available packages
        let mut metadata = ApiMetadata {
            distributions: Vec::new(),
        };

        // Archive types to query for (as expected by foojay.io API)
        let archive_types = vec![
            "tar.gz".to_string(),
            "zip".to_string(),
            "tgz".to_string(),
            "tar".to_string(),
        ];

        for dist in distributions {
            let query = PackageQuery {
                distribution: Some(dist.api_parameter.clone()),
                architecture: Some(architecture.clone()),
                package_type: None,
                operating_system: Some(operating_system.clone()),
                lib_c_type: Some(lib_c_type.to_string()),
                archive_types: Some(archive_types.clone()),
                latest: Some("available".to_string()),
                directly_downloadable: Some(true),
                version: None,
                javafx_bundled: Some(false),
            };

            let packages = match self.get_packages(Some(query)) {
                Ok(packages) => packages,
                Err(e) => {
                    debug!("Failed to fetch packages for {}: {e}", dist.api_parameter);
                    Vec::new()
                }
            };

            metadata.distributions.push(DistributionMetadata {
                distribution: dist,
                packages,
            });
        }

        Ok(metadata)
    }

    pub fn fetch_all_metadata_with_options(&self, javafx_bundled: bool) -> Result<ApiMetadata> {
        // Fetch distributions
        let distributions = self.get_distributions()?;

        // Get platform-specific parameters
        let architecture = crate::platform::get_current_architecture();
        let operating_system = crate::platform::get_current_os();
        let lib_c_type = get_foojay_libc_type();

        // For each distribution, fetch available packages
        let mut metadata = ApiMetadata {
            distributions: Vec::new(),
        };

        // Archive types to query for (as expected by foojay.io API)
        let archive_types = vec![
            "tar.gz".to_string(),
            "zip".to_string(),
            "tgz".to_string(),
            "tar".to_string(),
        ];

        for dist in distributions {
            let query = PackageQuery {
                distribution: Some(dist.api_parameter.clone()),
                architecture: Some(architecture.clone()),
                package_type: None,
                operating_system: Some(operating_system.clone()),
                lib_c_type: Some(lib_c_type.to_string()),
                archive_types: Some(archive_types.clone()),
                latest: Some("available".to_string()),
                directly_downloadable: Some(true),
                version: None,
                javafx_bundled: if javafx_bundled { None } else { Some(false) },
            };

            let packages = match self.get_packages(Some(query)) {
                Ok(packages) => packages,
                Err(e) => {
                    debug!("Failed to fetch packages for {}: {e}", dist.api_parameter);
                    Vec::new()
                }
            };

            metadata.distributions.push(DistributionMetadata {
                distribution: dist,
                packages,
            });
        }

        Ok(metadata)
    }

    pub fn get_packages(&self, query: Option<PackageQuery>) -> Result<Vec<Package>> {
        let url = format!("{}/{API_VERSION}/packages", self.base_url);
        let query = query.clone();

        self.execute_with_retry(move || {
            let mut request = self.session.get(&url);

            if let Some(ref q) = query {
                if let Some(ref version) = q.version {
                    request = request.param("version", version);
                }
                if let Some(ref distribution) = q.distribution {
                    request = request.param("distribution", distribution);
                }
                if let Some(ref architecture) = q.architecture {
                    request = request.param("architecture", architecture);
                }
                if let Some(ref package_type) = q.package_type {
                    request = request.param("package_type", package_type);
                }
                if let Some(ref operating_system) = q.operating_system {
                    request = request.param("operating_system", operating_system);
                }
                if let Some(ref archive_types) = q.archive_types {
                    for archive_type in archive_types {
                        request = request.param("archive_type", archive_type);
                    }
                }
                if let Some(ref latest) = q.latest {
                    request = request.param("latest", latest);
                }
                if let Some(directly_downloadable) = q.directly_downloadable {
                    request =
                        request.param("directly_downloadable", directly_downloadable.to_string());
                }
                if let Some(ref lib_c_type) = q.lib_c_type {
                    request = request.param("lib_c_type", lib_c_type);
                }
                if let Some(javafx_bundled) = q.javafx_bundled {
                    request = request.param("javafx_bundled", javafx_bundled.to_string());
                }
            }

            request
        })
    }

    pub fn get_distributions(&self) -> Result<Vec<Distribution>> {
        let url = format!("{}/{API_VERSION}/distributions", self.base_url);
        self.execute_with_retry(move || self.session.get(&url))
    }

    pub fn get_major_versions(&self) -> Result<Vec<MajorVersion>> {
        let url = format!("{}/{API_VERSION}/major_versions", self.base_url);
        self.execute_with_retry(move || self.session.get(&url))
    }

    pub fn get_package_by_id(&self, package_id: &str) -> Result<PackageInfo> {
        // Special handling for package by ID endpoint which returns an array
        let url = format!("{}/{API_VERSION}/ids/{package_id}", self.base_url);
        debug!("Fetching package info for ID: {package_id}");
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
                                        "No package info found for ID: {package_id_copy} (API v{API_VERSION})"
                                    )))
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse 'result' field as array: {e}");
                                trace!("Result field: {result:?}");
                                Err(KopiError::MetadataFetch(format!(
                                    "Failed to parse API v{API_VERSION} response: {e}"
                                )))
                            }
                        }
                    } else {
                        Err(KopiError::MetadataFetch(format!(
                            "Invalid API v{API_VERSION} response: missing 'result' field"
                        )))
                    }
                }
                Err(e) => {
                    debug!("Failed to parse as JSON: {e}");
                    Err(KopiError::MetadataFetch(format!(
                        "Invalid JSON response from API v{API_VERSION}: {e}"
                    )))
                }
            },
        )
    }

    fn execute_with_retry<T, F>(&self, request_builder: F) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
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
                                debug!("Failed to parse 'result' field: {e}");
                                trace!("Result field: {result:?}");
                                Err(KopiError::MetadataFetch(format!(
                                    "Failed to parse API v{API_VERSION} response: {e}"
                                )))
                            }
                        }
                    } else {
                        Err(KopiError::MetadataFetch(format!(
                            "Invalid API v{API_VERSION} response: missing 'result' field"
                        )))
                    }
                }
                Err(e) => {
                    debug!("Failed to parse as JSON: {e}");
                    Err(KopiError::MetadataFetch(format!(
                        "Invalid JSON response from API v{API_VERSION}: {e}"
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
                            "Network error connecting to foojay.io API v{API_VERSION}: {e}. Please check your internet connection and try again."
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

                    // Try to parse error response body for more specific error message
                    let error_msg = if status.as_u16() == 400 {
                        match response.text() {
                            Ok(body) => {
                                // Try to parse as API error response
                                match serde_json::from_str::<crate::api::models::ApiErrorResponse>(
                                    &body,
                                ) {
                                    Ok(error_response) => {
                                        // Check if the message indicates version not found
                                        if error_response.message.contains("not released yet") {
                                            format!(
                                                "Version not available: {}",
                                                error_response.message
                                            )
                                        } else {
                                            format!("Bad request: {}", error_response.message)
                                        }
                                    }
                                    Err(_) => format!(
                                        "HTTP error ({}) from foojay.io API v{API_VERSION}: {}",
                                        status.as_u16(),
                                        status.canonical_reason().unwrap_or("Unknown error")
                                    ),
                                }
                            }
                            Err(_) => format!(
                                "HTTP error ({}) from foojay.io API v{API_VERSION}: {}",
                                status.as_u16(),
                                status.canonical_reason().unwrap_or("Unknown error")
                            ),
                        }
                    } else {
                        match status.as_u16() {
                            404 => format!(
                                "The requested resource was not found on foojay.io API v{API_VERSION}. The API endpoint may have changed."
                            ),
                            500..=599 => format!(
                                "Server error occurred on foojay.io API v{API_VERSION}. Please try again later."
                            ),
                            401 | 403 => format!(
                                "Authentication failed for foojay.io API v{API_VERSION}. Please check your credentials."
                            ),
                            _ => format!(
                                "HTTP error ({}) from foojay.io API v{API_VERSION}: {}",
                                status.as_u16(),
                                status.canonical_reason().unwrap_or("Unknown error")
                            ),
                        }
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
                        "Failed to read response body: {e}"
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
