use crate::error::Result;
use attohttpc::{Response, Session};
use std::io::{self, Read};
use std::time::Duration;

/// HTTP client trait for abstracting HTTP operations
pub trait HttpClient: Send + Sync {
    /// Perform a GET request with custom headers
    fn get(&self, url: &str, headers: Vec<(String, String)>) -> Result<Box<dyn HttpResponse>>;

    /// Set the timeout for requests
    fn set_timeout(&mut self, timeout: Duration);
}

/// HTTP response trait for abstracting response handling
pub trait HttpResponse: Read + Send {
    /// Get the HTTP status code
    fn status(&self) -> u16;

    /// Get a header value by name
    fn header(&self, name: &str) -> Option<&str>;

    /// Get the final URL after redirects
    fn final_url(&self) -> Option<&str>;
}

/// Default timeout for HTTP requests
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Implementation of HttpClient using attohttpc
pub struct AttohttpcClient {
    timeout: Duration,
    user_agent: String,
}

impl AttohttpcClient {
    /// Create a new HTTP client with default settings
    pub fn new() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            user_agent: "kopi/0.1.0".to_string(),
        }
    }
}

impl Default for AttohttpcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for AttohttpcClient {
    fn get(&self, url: &str, headers: Vec<(String, String)>) -> Result<Box<dyn HttpResponse>> {
        // Create a new session for each request
        let session = Session::new();

        // Build request with method chaining to avoid lifetime issues
        let mut request_builder = session
            .get(url)
            .timeout(self.timeout)
            .header("User-Agent", &self.user_agent)
            .follow_redirects(true);

        // For Range header specifically, we can use a match pattern
        // This avoids the generic loop that causes lifetime issues
        for (key, value) in headers {
            match key.as_str() {
                "Range" => {
                    // Range header is the only custom header we use for resume
                    let range_value = value.clone();
                    request_builder = request_builder.header("Range", range_value);
                }
                _ => {
                    // For other headers, we can add them as needed
                    // Currently, we only use Range header for resume functionality
                }
            }
        }

        let response = request_builder.send()?;
        Ok(Box::new(AttohttpcResponse { response }))
    }

    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

struct AttohttpcResponse {
    response: Response,
}

impl Read for AttohttpcResponse {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.response.read(buf)
    }
}

impl HttpResponse for AttohttpcResponse {
    fn status(&self) -> u16 {
        self.response.status().as_u16()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.response.headers().get(name)?.to_str().ok()
    }

    fn final_url(&self) -> Option<&str> {
        Some(self.response.url().as_ref())
    }
}
