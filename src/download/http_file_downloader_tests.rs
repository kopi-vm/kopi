#[cfg(test)]
mod tests {
    use crate::download::{DownloadOptions, HttpClient, HttpFileDownloader, HttpResponse};
    use crate::error::{KopiError, Result};
    use std::io::{Cursor, Read};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::tempdir;

    // Mock implementations for testing
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
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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

        fn final_url(&self) -> Option<&str> {
            None
        }
    }

    #[test]
    fn test_parse_content_range() {
        use crate::download::http_file_downloader::parse_content_range;

        assert_eq!(parse_content_range("bytes 200-1023/1024"), Some(1024));
        assert_eq!(parse_content_range("bytes 0-499/1234"), Some(1234));
        assert_eq!(parse_content_range("bytes */5000"), Some(5000));
        assert_eq!(parse_content_range("invalid"), None);
        assert_eq!(parse_content_range("bytes 0-499/invalid"), None);
    }

    #[test]
    fn test_download_with_mock_client() {
        let test_content = b"Hello, JDK!";
        let mock_client = MockHttpClient::new(vec![MockResponse {
            status: 200,
            headers: vec![("Content-Length".to_string(), test_content.len().to_string())],
            body: test_content.to_vec(),
        }]);

        let mut downloader = HttpFileDownloader::with_client(Box::new(mock_client));
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = downloader.download(
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

        let mut downloader = HttpFileDownloader::with_client(Box::new(mock_client));
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        // Calculate expected checksum
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(test_content);
        let expected_checksum = hex::encode(hasher.finalize());

        let options = DownloadOptions {
            checksum: Some(expected_checksum),
            checksum_type: Some(crate::models::package::ChecksumType::Sha256),
            ..Default::default()
        };

        let result = downloader.download("http://example.com/jdk.tar.gz", &dest_path, &options);

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

        let mut downloader = HttpFileDownloader::with_client(Box::new(mock_client));
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let options = DownloadOptions {
            checksum: Some("invalid_checksum".to_string()),
            checksum_type: Some(crate::models::package::ChecksumType::Sha256),
            ..Default::default()
        };

        let result = downloader.download("http://example.com/jdk.tar.gz", &dest_path, &options);

        assert!(result.is_err());
        match result.unwrap_err() {
            KopiError::ValidationError(msg) => {
                assert!(msg.contains("Checksum verification failed"))
            }
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

        let mut downloader = HttpFileDownloader::with_client(Box::new(mock_client));
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = downloader.download(
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

        let mut downloader = HttpFileDownloader::with_client(Box::new(mock_client));
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.jar");

        let result = downloader.download(
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
