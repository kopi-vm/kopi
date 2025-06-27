/// Download management module for Kopi JDK version manager
///
/// This module provides functionality for downloading JDK distributions with:
/// - Progress reporting
/// - Resume support for interrupted downloads
/// - Checksum verification
/// - Configurable timeouts and size limits
mod checksum;
mod client;
mod http_file_downloader;
mod options;
mod progress;

// Re-export public types and traits
pub use client::{HttpClient, HttpResponse};
pub use http_file_downloader::{HttpFileDownloader, ProgressReporter};
pub use options::{DEFAULT_TIMEOUT, DownloadOptions, DownloadResult, MAX_DOWNLOAD_SIZE};
pub use progress::IndicatifProgressReporter;

use crate::error::Result;
use std::time::Duration;

/// Download a JDK package from the given URL
///
/// # Arguments
/// * `package` - JDK metadata containing download URL and checksum
/// * `no_progress` - Disable progress reporting
/// * `timeout_secs` - Optional timeout in seconds
///
/// # Returns
/// A `DownloadResult` containing the path to the downloaded file
pub fn download_jdk(
    package: &crate::models::jdk::JdkMetadata,
    no_progress: bool,
    timeout_secs: Option<u64>,
) -> Result<DownloadResult> {
    // Security validation
    crate::security::verify_https_security(&package.download_url)?;

    // Create HTTP file downloader
    let mut downloader = HttpFileDownloader::new();

    // Set timeout if provided
    if let Some(timeout) = timeout_secs {
        downloader
            .http_client
            .set_timeout(Duration::from_secs(timeout));
    }

    // Add progress reporter unless disabled
    if !no_progress {
        downloader = downloader.with_progress_reporter(Box::new(IndicatifProgressReporter::new()));
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
    let result_path = downloader.download(&package.download_url, &download_path, &options)?;

    Ok(DownloadResult::new(result_path, temp_dir))
}
