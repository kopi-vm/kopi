// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

pub fn download_jdk(
    package: &crate::models::metadata::JdkMetadata,
    no_progress: bool,
    timeout_secs: Option<u64>,
) -> Result<DownloadResult> {
    // Security validation
    let download_url = package.download_url.as_ref().ok_or_else(|| {
        crate::error::KopiError::InvalidConfig(
            "Missing download URL in package metadata".to_string(),
        )
    })?;
    crate::security::verify_https_security(download_url)?;

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
        checksum_type: package.checksum_type,
        resume: true,
        timeout: timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_TIMEOUT),
        max_size: MAX_DOWNLOAD_SIZE,
    };

    // Determine download path
    let temp_dir = tempfile::tempdir()?;
    let file_name = download_url.split('/').next_back().unwrap_or("jdk.tar.gz");
    let download_path = temp_dir.path().join(file_name);

    // Download the file
    let result_path = downloader.download(download_url, &download_path, &options)?;

    Ok(DownloadResult::new(result_path, temp_dir))
}
