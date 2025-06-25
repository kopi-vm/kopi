use crate::error::{KopiError, Result};
use attohttpc::{Response, Session};
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

const DOWNLOAD_CHUNK_SIZE: usize = 8192;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_DOWNLOAD_SIZE: u64 = 1_073_741_824; // 1GB

pub trait HttpClient: Send + Sync {
    fn get(&self, url: &str, headers: Vec<(String, String)>) -> Result<Box<dyn HttpResponse>>;
    fn set_timeout(&mut self, timeout: Duration);
}

pub trait HttpResponse: Read + Send {
    fn status(&self) -> u16;
    fn header(&self, name: &str) -> Option<&str>;
}

pub struct DownloadManager {
    http_client: Box<dyn HttpClient>,
    progress_reporter: Option<Box<dyn ProgressReporter>>,
}

pub trait ProgressReporter: Send + Sync {
    fn on_start(&mut self, total_bytes: u64);
    fn on_progress(&mut self, bytes_downloaded: u64);
    fn on_complete(&mut self);
}

pub struct DownloadOptions {
    pub checksum: Option<String>,
    pub resume: bool,
    pub timeout: Duration,
    pub max_size: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            checksum: None,
            resume: true,
            timeout: DEFAULT_TIMEOUT,
            max_size: MAX_DOWNLOAD_SIZE,
        }
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    pub fn new() -> Self {
        Self::with_client(Box::new(AttohttpcClient::new()))
    }

    pub fn with_client(http_client: Box<dyn HttpClient>) -> Self {
        Self {
            http_client,
            progress_reporter: None,
        }
    }

    pub fn with_progress_reporter(mut self, reporter: Box<dyn ProgressReporter>) -> Self {
        self.progress_reporter = Some(reporter);
        self
    }

    pub fn download(
        &mut self,
        url: &str,
        destination: &Path,
        options: &DownloadOptions,
    ) -> Result<PathBuf> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        // Check if we can resume from existing destination file
        let (download_path, start_byte, is_temp) = if options.resume && destination.exists() {
            // Resume from existing file
            let existing_size = fs::metadata(destination)?.len();
            (destination.to_path_buf(), existing_size, false)
        } else {
            // Use a temporary file for atomic writes
            let temp_file =
                NamedTempFile::new_in(destination.parent().unwrap_or_else(|| Path::new(".")))?;
            let temp_path = temp_file.path().to_path_buf();
            // Keep the temp file handle, but we'll use the path
            // The file will be automatically deleted if we encounter an error
            (temp_path, 0, true)
        };

        // Build headers for the request
        let mut headers = Vec::new();
        if start_byte > 0 {
            headers.push(("Range".to_string(), format!("bytes={}-", start_byte)));
        }

        // Execute request
        let response = self.http_client.get(url, headers)?;

        // Validate response
        self.validate_response(response.as_ref(), options.max_size)?;

        // Get total size from Content-Length header
        let total_size = self.get_total_size(response.as_ref(), start_byte)?;

        // Start progress reporting
        if let Some(reporter) = &mut self.progress_reporter {
            reporter.on_start(total_size);
        }

        // Download file
        let downloaded_path =
            self.download_to_file(response, &download_path, start_byte, total_size)?;

        // Verify checksum if provided
        if let Some(expected_checksum) = &options.checksum {
            self.verify_checksum(&downloaded_path, expected_checksum)?;
        }

        // Move temp file to final destination if we used a temp file
        if is_temp {
            fs::rename(&downloaded_path, destination)?;
        }

        // Complete progress reporting
        if let Some(reporter) = &mut self.progress_reporter {
            reporter.on_complete();
        }

        Ok(destination.to_path_buf())
    }

    fn validate_response(&self, response: &dyn HttpResponse, max_size: u64) -> Result<()> {
        let status = response.status();

        if !(200..300).contains(&status) && status != 206 {
            return Err(KopiError::NetworkError(format!(
                "Download failed with status: {}",
                status
            )));
        }

        // Check content length if available
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(length) = content_length.parse::<u64>() {
                if length > max_size {
                    return Err(KopiError::ValidationError(format!(
                        "Download size {} exceeds maximum allowed size {}",
                        length, max_size
                    )));
                }
            }
        }

        Ok(())
    }

    fn get_total_size(&self, response: &dyn HttpResponse, start_byte: u64) -> Result<u64> {
        // Try to get size from Content-Range header (for resumed downloads)
        if let Some(content_range) = response.header("Content-Range") {
            if let Some(total) = self.parse_content_range(content_range) {
                return Ok(total);
            }
        }

        // Fall back to Content-Length
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(length) = content_length.parse::<u64>() {
                return Ok(start_byte + length);
            }
        }

        // If we can't determine size, return 0 (unknown)
        Ok(0)
    }

    fn parse_content_range(&self, range_str: &str) -> Option<u64> {
        // Parse "bytes start-end/total" format
        if let Some(slash_pos) = range_str.rfind('/') {
            if let Ok(total) = range_str[slash_pos + 1..].parse::<u64>() {
                return Some(total);
            }
        }
        None
    }

    fn download_to_file(
        &mut self,
        mut response: Box<dyn HttpResponse>,
        path: &Path,
        start_byte: u64,
        _total_size: u64,
    ) -> Result<PathBuf> {
        let file = if start_byte > 0 {
            fs::OpenOptions::new().append(true).open(path)?
        } else {
            File::create(path)?
        };

        let mut writer = BufWriter::new(file);
        let mut downloaded = start_byte;
        let mut buffer = vec![0; DOWNLOAD_CHUNK_SIZE];

        loop {
            match response.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    writer.write_all(&buffer[..n])?;
                    downloaded += n as u64;

                    if let Some(reporter) = &mut self.progress_reporter {
                        reporter.on_progress(downloaded);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }

        writer.flush()?;
        Ok(path.to_path_buf())
    }

    fn verify_checksum(&self, file_path: &Path, expected: &str) -> Result<()> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; DOWNLOAD_CHUNK_SIZE];

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buffer[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }

        let result = format!("{:x}", hasher.finalize());
        if result != expected {
            return Err(KopiError::ValidationError(format!(
                "Checksum mismatch: expected {}, got {}",
                expected, result
            )));
        }

        Ok(())
    }
}

pub struct ConsoleProgressReporter {
    total_bytes: u64,
    last_printed_percent: u32,
}

impl ConsoleProgressReporter {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            last_printed_percent: 0,
        }
    }
}

impl Default for ConsoleProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for ConsoleProgressReporter {
    fn on_start(&mut self, total_bytes: u64) {
        self.total_bytes = total_bytes;
        self.last_printed_percent = 0;
        if total_bytes > 0 {
            println!("Downloading... ({})", format_bytes(total_bytes));
        } else {
            println!("Downloading...");
        }
    }

    fn on_progress(&mut self, bytes_downloaded: u64) {
        if self.total_bytes > 0 {
            let percent = ((bytes_downloaded as f64 / self.total_bytes as f64) * 100.0) as u32;
            if percent > self.last_printed_percent && percent % 10 == 0 {
                self.last_printed_percent = percent;
                println!("{}% complete", percent);
            }
        }
    }

    fn on_complete(&mut self) {
        println!("Download complete");
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

// Real HTTP client implementation
pub struct AttohttpcClient {
    timeout: Duration,
    user_agent: String,
}

impl AttohttpcClient {
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
            .header("User-Agent", &self.user_agent);

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
}

/// Download a JDK package from the given URL
pub fn download_jdk(
    package: &crate::models::jdk::JdkMetadata,
    no_progress: bool,
    timeout_secs: Option<u64>,
) -> Result<PathBuf> {
    // Security validation
    let security_manager = crate::security::SecurityManager::new();
    security_manager.verify_https_security(&package.download_url)?;

    // Create download manager
    let mut download_manager = DownloadManager::new();

    // Set timeout if provided
    if let Some(timeout) = timeout_secs {
        download_manager
            .http_client
            .set_timeout(Duration::from_secs(timeout));
    }

    // Add progress reporter unless disabled
    if !no_progress {
        download_manager =
            download_manager.with_progress_reporter(Box::new(ConsoleProgressReporter::new()));
    }

    // Prepare download options
    let options = DownloadOptions {
        checksum: package.checksum.clone(),
        resume: true,
        timeout: timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_TIMEOUT),
        max_size: MAX_DOWNLOAD_SIZE,
    };

    // Determine download path
    let temp_dir = tempfile::tempdir()?;
    let file_name = package
        .download_url
        .split('/')
        .next_back()
        .unwrap_or("jdk.tar.gz");
    let download_path = temp_dir.path().join(file_name);

    // Download the file
    let result_path = download_manager.download(&package.download_url, &download_path, &options)?;

    // Keep the temp directory alive by leaking it
    // The caller is responsible for cleaning up the file
    std::mem::forget(temp_dir);

    Ok(result_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    // Mock HTTP client for testing
    struct MockHttpClient {
        responses: Vec<MockResponse>,
        request_count: Arc<Mutex<usize>>,
    }

    struct MockResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl MockHttpClient {
        fn new(responses: Vec<MockResponse>) -> Self {
            Self {
                responses,
                request_count: Arc::new(Mutex::new(0)),
            }
        }
    }

    impl HttpClient for MockHttpClient {
        fn get(
            &self,
            _url: &str,
            _headers: Vec<(String, String)>,
        ) -> Result<Box<dyn HttpResponse>> {
            let mut count = self.request_count.lock().unwrap();
            if *count >= self.responses.len() {
                return Err(KopiError::NetworkError(
                    "No more mock responses".to_string(),
                ));
            }

            let response = &self.responses[*count];
            *count += 1;

            Ok(Box::new(MockHttpResponse {
                status: response.status,
                headers: response.headers.clone(),
                body: Cursor::new(response.body.clone()),
            }))
        }

        fn set_timeout(&mut self, _timeout: Duration) {
            // Mock implementation - no-op
        }
    }

    struct MockHttpResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: Cursor<Vec<u8>>,
    }

    impl Read for MockHttpResponse {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.body.read(buf)
        }
    }

    impl HttpResponse for MockHttpResponse {
        fn status(&self) -> u16 {
            self.status
        }

        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(1023), "1023.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_parse_content_range() {
        let manager = DownloadManager::new();

        assert_eq!(
            manager.parse_content_range("bytes 200-1023/1024"),
            Some(1024)
        );
        assert_eq!(manager.parse_content_range("bytes 0-499/1234"), Some(1234));
        assert_eq!(manager.parse_content_range("invalid"), None);
    }

    #[test]
    fn test_download_options_default() {
        let options = DownloadOptions::default();
        assert_eq!(options.checksum, None);
        assert_eq!(options.resume, true);
        assert_eq!(options.timeout, DEFAULT_TIMEOUT);
        assert_eq!(options.max_size, MAX_DOWNLOAD_SIZE);
    }

    #[test]
    fn test_download_with_mock_client() {
        let test_content = b"Hello, JDK!";
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 200,
            headers: vec![("Content-Length".to_string(), test_content.len().to_string())],
            body: test_content.to_vec(),
        }]);

        let mut manager = DownloadManager::with_client(Box::new(mock_client));
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = manager.download(
            "http://example.com/jdk.tar.gz",
            &dest_path,
            &DownloadOptions::default(),
        );

        assert!(result.is_ok());
        assert!(dest_path.exists());

        let content = std::fs::read(&dest_path).unwrap();
        assert_eq!(content, test_content);
    }

    #[test]
    fn test_download_with_checksum_validation() {
        let test_content = b"Hello, JDK!";
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 200,
            headers: vec![("Content-Length".to_string(), test_content.len().to_string())],
            body: test_content.to_vec(),
        }]);

        let mut manager = DownloadManager::with_client(Box::new(mock_client));
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        // Calculate expected checksum
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(test_content);
        let expected_checksum = format!("{:x}", hasher.finalize());

        let options = DownloadOptions {
            checksum: Some(expected_checksum),
            ..Default::default()
        };

        let result = manager.download("http://example.com/jdk.tar.gz", &dest_path, &options);

        assert!(result.is_ok());
    }

    #[test]
    fn test_download_with_invalid_checksum() {
        let test_content = b"Hello, JDK!";
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 200,
            headers: vec![("Content-Length".to_string(), test_content.len().to_string())],
            body: test_content.to_vec(),
        }]);

        let mut manager = DownloadManager::with_client(Box::new(mock_client));
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let options = DownloadOptions {
            checksum: Some("invalid_checksum".to_string()),
            ..Default::default()
        };

        let result = manager.download("http://example.com/jdk.tar.gz", &dest_path, &options);

        assert!(result.is_err());
        match result.unwrap_err() {
            KopiError::ValidationError(msg) => assert!(msg.contains("Checksum mismatch")),
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_download_with_http_error() {
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 404,
            headers: vec![],
            body: vec![],
        }]);

        let mut manager = DownloadManager::with_client(Box::new(mock_client));
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = manager.download(
            "http://example.com/jdk.tar.gz",
            &dest_path,
            &DownloadOptions::default(),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            KopiError::NetworkError(msg) => assert!(msg.contains("404")),
            _ => panic!("Expected NetworkError"),
        }
    }

    #[test]
    fn test_download_exceeding_size_limit() {
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 200,
            headers: vec![
                ("Content-Length".to_string(), "2000000000".to_string()), // 2GB
            ],
            body: vec![],
        }]);

        let mut manager = DownloadManager::with_client(Box::new(mock_client));
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = manager.download(
            "http://example.com/jdk.tar.gz",
            &dest_path,
            &DownloadOptions::default(),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            KopiError::ValidationError(msg) => assert!(msg.contains("exceeds maximum")),
            _ => panic!("Expected ValidationError"),
        }
    }
}
