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
use crate::storage::formatting::format_size;

const CHILD_PROGRESS_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB

pub struct DownloadProgressAdapter {
    indicator: Box<dyn ProgressIndicator>,
    operation: String,
    context: String,
    parent_indicator: Option<Box<dyn ProgressIndicator>>,
    child_indicator: Option<Box<dyn ProgressIndicator>>,
}

impl DownloadProgressAdapter {
    pub fn new(
        operation: String,
        context: String,
        parent: Option<Box<dyn ProgressIndicator>>,
        no_progress: bool,
    ) -> Self {
        Self {
            indicator: if parent.is_none() {
                ProgressFactory::create(no_progress)
            } else {
                ProgressFactory::create(false) // Not used when we have parent
            },
            operation,
            context,
            parent_indicator: parent,
            child_indicator: None,
        }
    }

    pub fn for_jdk_download(
        package_name: &str,
        parent: Option<Box<dyn ProgressIndicator>>,
        no_progress: bool,
    ) -> Self {
        Self::new(
            "Downloading".to_string(),
            package_name.to_string(),
            parent,
            no_progress,
        )
    }
}

impl ProgressReporter for DownloadProgressAdapter {
    fn on_start(&mut self, total_bytes: u64) {
        // Decide whether to create a child progress based on file size
        if let Some(parent) = self.parent_indicator.take() {
            if total_bytes >= CHILD_PROGRESS_THRESHOLD {
                // Create child progress for large downloads
                self.parent_indicator = Some(parent);
                let mut child = self.parent_indicator.as_mut().unwrap().create_child();

                let config =
                    ProgressConfig::new(&self.operation, &self.context, ProgressStyle::Bytes)
                        .with_total(total_bytes);
                child.start(config);

                if total_bytes > 0 {
                    child.set_message(format!("0 / {total_bytes} bytes"));
                } else {
                    child.set_message("Starting download...".to_string());
                }

                self.child_indicator = Some(child);
            } else {
                // Small file - update parent message instead
                self.parent_indicator = Some(parent);
                let msg = if total_bytes == 0 {
                    format!("Downloading {} (unknown size)", self.context)
                } else {
                    format!(
                        "Downloading {} ({})",
                        self.context,
                        format_size(total_bytes)
                    )
                };
                self.parent_indicator.as_mut().unwrap().set_message(msg);
            }
        } else {
            // No parent - use regular indicator
            let config = if total_bytes > 0 {
                ProgressConfig::new(&self.operation, &self.context, ProgressStyle::Bytes)
                    .with_total(total_bytes)
            } else {
                ProgressConfig::new(&self.operation, &self.context, ProgressStyle::Bytes)
            };
            self.indicator.start(config);

            if total_bytes > 0 {
                self.indicator
                    .set_message(format!("0 / {total_bytes} bytes"));
            } else {
                self.indicator
                    .set_message("Starting download...".to_string());
            }
        }
    }

    fn on_progress(&mut self, bytes_downloaded: u64) {
        if let Some(child) = &mut self.child_indicator {
            // Update child progress
            child.update(bytes_downloaded, None);
            let mb_downloaded = bytes_downloaded as f64 / (1024.0 * 1024.0);
            child.set_message(format!("{mb_downloaded:.1} MB downloaded"));
        } else if self.parent_indicator.is_some() {
            // Small file - just update parent message
            let mb_downloaded = bytes_downloaded as f64 / (1024.0 * 1024.0);
            let msg = format!("Downloading {} ({:.1} MB)", self.context, mb_downloaded);
            self.parent_indicator.as_mut().unwrap().set_message(msg);
        } else {
            // No parent - use regular indicator
            self.indicator.update(bytes_downloaded, None);
            let mb_downloaded = bytes_downloaded as f64 / (1024.0 * 1024.0);
            self.indicator
                .set_message(format!("{mb_downloaded:.1} MB downloaded"));
        }
    }

    fn on_complete(&mut self) {
        if let Some(mut child) = self.child_indicator.take() {
            // Complete child progress
            child.complete(Some("Download complete".to_string()));
        } else if self.parent_indicator.is_some() {
            // Small file - update parent message
            let msg = format!("Downloaded {}", self.context);
            self.parent_indicator.as_mut().unwrap().set_message(msg);
        } else {
            // No parent - use regular indicator
            self.indicator
                .complete(Some("Download complete".to_string()));
        }
    }
}

// Keep the old name for backward compatibility during migration
pub type IndicatifProgressReporter = DownloadProgressAdapter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_progress_with_total() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download("temurin@21", None, false);

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
        let mut adapter = DownloadProgressAdapter::for_jdk_download("liberica@17", None, false);

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
        let mut adapter = DownloadProgressAdapter::for_jdk_download("corretto@11", None, true);

        // Should work silently without panicking
        adapter.on_start(2048 * 1024); // 2MB
        adapter.on_progress(1024 * 1024); // 1MB
        adapter.on_complete();
    }

    #[test]
    fn test_custom_operation_context() {
        let mut adapter = DownloadProgressAdapter::new(
            "Fetching".to_string(),
            "archive.tar.gz".to_string(),
            None,
            false,
        );

        adapter.on_start(5000000);
        adapter.on_progress(2500000);
        adapter.on_complete();
    }

    #[test]
    fn test_progress_reporter_trait_impl() {
        // Verify it implements the ProgressReporter trait
        fn accepts_reporter(_reporter: Box<dyn ProgressReporter>) {}

        let adapter = DownloadProgressAdapter::for_jdk_download("zulu@21", None, false);
        accepts_reporter(Box::new(adapter));
    }

    #[test]
    fn test_incremental_progress_updates() {
        let mut adapter = DownloadProgressAdapter::for_jdk_download("graalvm@21", None, false);

        adapter.on_start(1000);

        // Simulate incremental download
        for i in 1..=10 {
            adapter.on_progress(i * 100);
        }

        adapter.on_complete();
    }

    #[test]
    fn test_large_download_creates_child() {
        // Create a parent progress indicator
        let parent = ProgressFactory::create(false);
        let mut adapter =
            DownloadProgressAdapter::for_jdk_download("temurin@21", Some(parent), false);

        // Start with size >= 10MB (should create child)
        adapter.on_start(15 * 1024 * 1024); // 15MB

        // Simulate download progress
        adapter.on_progress(5 * 1024 * 1024); // 5MB
        adapter.on_progress(10 * 1024 * 1024); // 10MB
        adapter.on_progress(15 * 1024 * 1024); // 15MB

        // Complete the download
        adapter.on_complete();
    }

    #[test]
    fn test_small_download_no_child() {
        // Create a parent progress indicator
        let parent = ProgressFactory::create(false);
        let mut adapter =
            DownloadProgressAdapter::for_jdk_download("tool@1.0", Some(parent), false);

        // Start with size < 10MB (should not create child)
        adapter.on_start(5 * 1024 * 1024); // 5MB

        // Simulate download progress
        adapter.on_progress(1024 * 1024); // 1MB
        adapter.on_progress(3 * 1024 * 1024); // 3MB
        adapter.on_progress(5 * 1024 * 1024); // 5MB

        // Complete the download
        adapter.on_complete();
    }

    #[test]
    fn test_unknown_size_no_child() {
        // Create a parent progress indicator
        let parent = ProgressFactory::create(false);
        let mut adapter =
            DownloadProgressAdapter::for_jdk_download("unknown@1.0", Some(parent), false);

        // Start with unknown size (0)
        adapter.on_start(0);

        // Simulate download progress
        adapter.on_progress(100_000);
        adapter.on_progress(200_000);
        adapter.on_progress(300_000);

        // Complete the download
        adapter.on_complete();
    }

    #[test]
    fn test_exact_threshold_creates_child() {
        // Create a parent progress indicator
        let parent = ProgressFactory::create(false);
        let mut adapter =
            DownloadProgressAdapter::for_jdk_download("boundary@1.0", Some(parent), false);

        // Start with exactly 10MB (should create child)
        adapter.on_start(10 * 1024 * 1024); // Exactly 10MB

        adapter.on_progress(5 * 1024 * 1024);
        adapter.on_progress(10 * 1024 * 1024);

        adapter.on_complete();
    }

    #[test]
    fn test_just_below_threshold_no_child() {
        // Create a parent progress indicator
        let parent = ProgressFactory::create(false);
        let mut adapter =
            DownloadProgressAdapter::for_jdk_download("small@1.0", Some(parent), false);

        // Start with just below 10MB (should not create child)
        adapter.on_start(10 * 1024 * 1024 - 1); // 1 byte less than 10MB

        adapter.on_progress(5 * 1024 * 1024);
        adapter.on_progress(10 * 1024 * 1024 - 1);

        adapter.on_complete();
    }
}
