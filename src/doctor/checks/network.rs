use crate::api::client::{API_VERSION, FOOJAY_API_BASE};
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use crate::user_agent;
use std::env;
use std::time::{Duration, Instant};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(5);

fn get_api_health_check_url() -> String {
    format!("{FOOJAY_API_BASE}/{API_VERSION}")
}

pub struct ApiConnectivityCheck;

impl DiagnosticCheck for ApiConnectivityCheck {
    fn name(&self) -> &str {
        "API Connectivity"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();

        // Create HTTP client with timeout
        let mut session = attohttpc::Session::new();
        session.timeout(NETWORK_TIMEOUT);
        session.header("User-Agent", user_agent::doctor_client());

        match session.get(get_api_health_check_url()).send() {
            Ok(response) => {
                if response.is_success() {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        "Successfully connected to Foojay API",
                        duration,
                    )
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Fail,
                        format!("API returned status: {}", response.status()),
                        duration,
                    )
                    .with_details(format!("URL: {}", get_api_health_check_url()))
                    .with_suggestion("Check if api.foojay.io is accessible from your network")
                }
            }
            Err(e) => {
                let error_str = e.to_string();
                let (message, suggestion) =
                    if error_str.contains("timed out") || error_str.contains("timeout") {
                        (
                            "Connection timed out after 5 seconds".to_string(),
                            "Check your internet connection or proxy settings",
                        )
                    } else if error_str.contains("connection") || error_str.contains("connect") {
                        (
                            "Failed to connect to API".to_string(),
                            "Check your internet connection and firewall settings",
                        )
                    } else {
                        (
                            format!("Network error: {e}"),
                            "Check your network configuration",
                        )
                    };

                CheckResult::new(self.name(), category, CheckStatus::Fail, message, duration)
                    .with_details(format!("URL: {}", get_api_health_check_url()))
                    .with_suggestion(suggestion)
            }
        }
    }
}

pub struct ProxyConfigurationCheck;

impl DiagnosticCheck for ProxyConfigurationCheck {
    fn name(&self) -> &str {
        "Proxy Configuration"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();

        let http_proxy = env::var("HTTP_PROXY").or_else(|_| env::var("http_proxy"));
        let https_proxy = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy"));
        let no_proxy = env::var("NO_PROXY").or_else(|_| env::var("no_proxy"));

        match (http_proxy, https_proxy) {
            (Ok(http), Ok(https)) => {
                let mut details = format!("HTTP_PROXY: {http}\nHTTPS_PROXY: {https}");
                if let Ok(no) = no_proxy {
                    details.push_str(&format!("\nNO_PROXY: {no}"));
                }

                // Validate proxy format
                if validate_proxy_url(&http) && validate_proxy_url(&https) {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        "Proxy configuration detected and valid",
                        duration,
                    )
                    .with_details(details)
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        "Proxy configuration detected but may be invalid",
                        duration,
                    )
                    .with_details(details)
                    .with_suggestion(
                        "Ensure proxy URLs are in the format: http://proxy.example.com:port",
                    )
                }
            }
            (Ok(http), Err(_)) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                "Only HTTP_PROXY is set, HTTPS_PROXY is missing",
                duration,
            )
            .with_details(format!("HTTP_PROXY: {http}"))
            .with_suggestion(
                "Set HTTPS_PROXY for secure connections: export HTTPS_PROXY=<proxy-url>",
            ),
            (Err(_), Ok(https)) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                "Only HTTPS_PROXY is set, HTTP_PROXY is missing",
                duration,
            )
            .with_details(format!("HTTPS_PROXY: {https}"))
            .with_suggestion(
                "Set HTTP_PROXY for non-secure connections: export HTTP_PROXY=<proxy-url>",
            ),
            (Err(_), Err(_)) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "No proxy configuration detected",
                duration,
            )
            .with_details("Direct internet connection assumed"),
        }
    }
}

pub struct DnsResolutionCheck;

impl DiagnosticCheck for DnsResolutionCheck {
    fn name(&self) -> &str {
        "DNS Resolution"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();

        // Try to resolve api.foojay.io
        match std::net::ToSocketAddrs::to_socket_addrs(&("api.foojay.io", 443)) {
            Ok(addrs) => {
                let addr_list: Vec<_> = addrs.collect();
                if addr_list.is_empty() {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Fail,
                        "DNS resolution succeeded but no addresses returned",
                        duration,
                    )
                    .with_suggestion("Check your DNS configuration")
                } else {
                    let addr_strings: Vec<String> =
                        addr_list.iter().map(|addr| addr.to_string()).collect();

                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        "Successfully resolved api.foojay.io",
                        duration,
                    )
                    .with_details(format!("Resolved addresses: {}", addr_strings.join(", ")))
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Failed to resolve api.foojay.io: {e}"),
                duration,
            )
            .with_suggestion(
                "Check your DNS settings or try using a different DNS server (e.g., 8.8.8.8)",
            ),
        }
    }
}

pub struct TlsVerificationCheck;

impl DiagnosticCheck for TlsVerificationCheck {
    fn name(&self) -> &str {
        "TLS/SSL Verification"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();

        // Test TLS connection with certificate verification
        let mut client = attohttpc::Session::new();
        client.timeout(NETWORK_TIMEOUT);
        client.header("User-Agent", user_agent::doctor_client());

        match client.head(get_api_health_check_url()).send() {
            Ok(_) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Pass,
                "TLS certificate verification successful",
                duration,
            )
            .with_details("Successfully verified api.foojay.io certificate"),
            Err(e) => {
                let error_str = e.to_string();
                let (message, suggestion) = if error_str.contains("certificate")
                    || error_str.contains("TLS")
                    || error_str.contains("SSL")
                {
                    (
                        "TLS/SSL certificate verification failed".to_string(),
                        "Check system certificate store or proxy MITM certificates",
                    )
                } else {
                    (
                        format!("TLS connection failed: {e}"),
                        "Check network connectivity and TLS configuration",
                    )
                };

                CheckResult::new(self.name(), category, CheckStatus::Fail, message, duration)
                    .with_suggestion(suggestion)
            }
        }
    }
}

// Helper function to validate proxy URL format
fn validate_proxy_url(url: &str) -> bool {
    // Basic validation - check if it starts with http:// or https://
    // and contains a host
    if url.starts_with("http://") || url.starts_with("https://") {
        let without_scheme = url
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        !without_scheme.is_empty() && without_scheme.contains('.')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_proxy_url() {
        assert!(validate_proxy_url("http://proxy.example.com:8080"));
        assert!(validate_proxy_url("https://proxy.example.com:8080"));
        assert!(validate_proxy_url("http://10.0.0.1:3128"));

        assert!(!validate_proxy_url("proxy.example.com:8080"));
        assert!(!validate_proxy_url("http://"));
        assert!(!validate_proxy_url("https://"));
        assert!(!validate_proxy_url("not-a-url"));
        assert!(!validate_proxy_url(""));
    }

    #[test]
    fn test_proxy_check_name() {
        let check = ProxyConfigurationCheck;
        assert_eq!(check.name(), "Proxy Configuration");
    }

    #[test]
    fn test_api_connectivity_check_name() {
        let check = ApiConnectivityCheck;
        assert_eq!(check.name(), "API Connectivity");
    }

    #[test]
    fn test_dns_resolution_check_name() {
        let check = DnsResolutionCheck;
        assert_eq!(check.name(), "DNS Resolution");
    }

    #[test]
    fn test_tls_verification_check_name() {
        let check = TlsVerificationCheck;
        assert_eq!(check.name(), "TLS/SSL Verification");
    }
}
