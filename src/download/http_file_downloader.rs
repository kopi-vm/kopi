use crate::download::checksum::verify_checksum;
use crate::download::client::{AttohttpcClient, HttpClient, HttpResponse};
use crate::download::options::DownloadOptions;
use crate::error::{KopiError, Result};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

const DOWNLOAD_CHUNK_SIZE: usize = 8192;

pub trait ProgressReporter: Send + Sync {
    fn on_start(&mut self, total_bytes: u64);

    fn on_progress(&mut self, bytes_downloaded: u64);

    fn on_complete(&mut self);
}

pub struct HttpFileDownloader {
    pub(crate) http_client: Box<dyn HttpClient>,
    progress_reporter: Option<Box<dyn ProgressReporter>>,
}

impl Default for HttpFileDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpFileDownloader {
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
            headers.push(("Range".to_string(), format!("bytes={start_byte}-")));
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
            verify_checksum(&downloaded_path, expected_checksum)?;
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
                "Download failed with status: {status}"
            )));
        }

        // Check content length if available
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(length) = content_length.parse::<u64>() {
                if length > max_size {
                    return Err(KopiError::ValidationError(format!(
                        "Download size {length} exceeds maximum allowed size {max_size}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn get_total_size(&self, response: &dyn HttpResponse, start_byte: u64) -> Result<u64> {
        // Try to get size from Content-Range header (for resumed downloads)
        if let Some(content_range) = response.header("Content-Range") {
            if let Some(total) = parse_content_range(content_range) {
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
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }

        writer.flush()?;
        Ok(path.to_path_buf())
    }
}

pub(crate) fn parse_content_range(range_str: &str) -> Option<u64> {
    if let Some(slash_pos) = range_str.rfind('/') {
        if let Ok(total) = range_str[slash_pos + 1..].parse::<u64>() {
            return Some(total);
        }
    }
    None
}

#[cfg(test)]
#[path = "http_file_downloader_tests.rs"]
mod http_file_downloader_tests;
