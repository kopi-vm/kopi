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

use super::ProgressReporter;
use crate::indicator::{ProgressConfig, ProgressFactory, ProgressIndicator, ProgressStyle};

pub struct DownloadProgressAdapter {
    indicator: Box<dyn ProgressIndicator>,
    operation: String,
    context: String,
}

impl DownloadProgressAdapter {
    pub fn new(no_progress: bool, operation: String, context: String) -> Self {
        Self {
            indicator: ProgressFactory::create(no_progress),
            operation,
            context,
        }
    }

    pub fn for_jdk_download(no_progress: bool, package_name: &str) -> Self {
        Self::new(
            no_progress,
            "Downloading".to_string(),
            package_name.to_string(),
        )
    }
}

impl ProgressReporter for DownloadProgressAdapter {
    fn on_start(&mut self, total_bytes: u64) {
        let config = if total_bytes > 0 {
            ProgressConfig::new(&self.operation, &self.context, ProgressStyle::Bytes)
                .with_total(total_bytes)
        } else {
            ProgressConfig::new(&self.operation, &self.context, ProgressStyle::Bytes)
        };
        self.indicator.start(config);
    }

    fn on_progress(&mut self, bytes_downloaded: u64) {
        self.indicator.update(bytes_downloaded, None);
    }

    fn on_complete(&mut self) {
        self.indicator
            .complete(Some("Download complete".to_string()));
    }
}

// Keep the old name for backward compatibility during migration
pub type IndicatifProgressReporter = DownloadProgressAdapter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_progress_with_total() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download(false, "temurin@21");

        // Start with known total size
        adapter.on_start(1024 * 1024); // 1MB

        // Report some progress
        adapter.on_progress(512 * 1024); // 512KB
        adapter.on_progress(1024 * 1024); // 1MB

        // Complete the download
        adapter.on_complete();
    }

    #[test]
    fn test_download_progress_without_total() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download(false, "liberica@17");

        // Start with unknown total size
        adapter.on_start(0);

        // Report some progress
        adapter.on_progress(256 * 1024); // 256KB
        adapter.on_progress(512 * 1024); // 512KB

        // Complete the download
        adapter.on_complete();
    }

    #[test]
    fn test_download_progress_no_progress_mode() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download(true, "corretto@11");

        // Should work silently without panicking
        adapter.on_start(2048 * 1024); // 2MB
        adapter.on_progress(1024 * 1024); // 1MB
        adapter.on_complete();
    }

    #[test]
    fn test_custom_operation_context() {
        let mut adapter = DownloadProgressAdapter::new(
            false,
            "Fetching".to_string(),
            "archive.tar.gz".to_string(),
        );

        adapter.on_start(5000000);
        adapter.on_progress(2500000);
        adapter.on_complete();
    }

    #[test]
    fn test_progress_reporter_trait_impl() {
        // Verify it implements the ProgressReporter trait
        fn accepts_reporter(_reporter: Box<dyn ProgressReporter>) {}

        let adapter = DownloadProgressAdapter::for_jdk_download(false, "zulu@21");
        accepts_reporter(Box::new(adapter));
    }

    #[test]
    fn test_incremental_progress_updates() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download(false, "graalvm@21");

        adapter.on_start(1000);

        // Simulate incremental download
        for i in 1..=10 {
            adapter.on_progress(i * 100);
        }

        adapter.on_complete();
    }
}
