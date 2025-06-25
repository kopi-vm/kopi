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

pub struct DownloadManager {
    session: Session,
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
        let mut session = Session::new();
        session.timeout(DEFAULT_TIMEOUT);
        session.header("User-Agent", "kopi/0.1.0");

        Self {
            session,
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

        // Use a temporary file for atomic writes
        let temp_file =
            NamedTempFile::new_in(destination.parent().unwrap_or_else(|| Path::new(".")))?;
        let temp_path = temp_file.path().to_path_buf();

        // Check if we can resume a partial download
        let start_byte = if options.resume && temp_path.exists() {
            fs::metadata(&temp_path)?.len()
        } else {
            0
        };

        // Build request with optional range header for resume
        let mut request = self.session.get(url).timeout(options.timeout);
        if start_byte > 0 {
            request = request.header("Range", format!("bytes={}-", start_byte));
        }

        // Execute request
        let response = request.send()?;

        // Validate response
        self.validate_response(&response, options.max_size)?;

        // Get total size from Content-Length header
        let total_size = self.get_total_size(&response, start_byte)?;

        // Start progress reporting
        if let Some(reporter) = &mut self.progress_reporter {
            reporter.on_start(total_size);
        }

        // Download file
        let downloaded_path =
            self.download_to_file(response, &temp_path, start_byte, total_size)?;

        // Verify checksum if provided
        if let Some(expected_checksum) = &options.checksum {
            self.verify_checksum(&downloaded_path, expected_checksum)?;
        }

        // Move temp file to final destination
        fs::rename(&downloaded_path, destination)?;

        // Complete progress reporting
        if let Some(reporter) = &mut self.progress_reporter {
            reporter.on_complete();
        }

        Ok(destination.to_path_buf())
    }

    fn validate_response(&self, response: &Response, max_size: u64) -> Result<()> {
        let status = response.status();

        if !status.is_success() && status.as_u16() != 206 {
            return Err(KopiError::NetworkError(format!(
                "Download failed with status: {}",
                status
            )));
        }

        // Check content length if available
        if let Some(content_length) = response.headers().get("Content-Length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<u64>() {
                    if length > max_size {
                        return Err(KopiError::ValidationError(format!(
                            "Download size {} exceeds maximum allowed size {}",
                            length, max_size
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    fn get_total_size(&self, response: &Response, start_byte: u64) -> Result<u64> {
        // Try to get size from Content-Range header (for resumed downloads)
        if let Some(content_range) = response.headers().get("Content-Range") {
            if let Ok(range_str) = content_range.to_str() {
                if let Some(total) = self.parse_content_range(range_str) {
                    return Ok(total);
                }
            }
        }

        // Fall back to Content-Length
        if let Some(content_length) = response.headers().get("Content-Length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<u64>() {
                    return Ok(start_byte + length);
                }
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
        mut response: Response,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
