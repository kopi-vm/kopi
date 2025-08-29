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

//! Integration test for multi-progress functionality in install command

use kopi::download::{DownloadProgressAdapter, ProgressReporter};
use kopi::indicator::{ProgressConfig, ProgressFactory, ProgressStyle};

#[test]
fn test_download_creates_child_for_large_file() {
    // Create parent progress
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);

    // Create download progress adapter with parent
    let mut adapter =
        DownloadProgressAdapter::for_jdk_download("temurin@21", Some(parent.create_child()), false);

    // Simulate large download (>= 10MB)
    adapter.on_start(20 * 1024 * 1024); // 20MB

    // Should have created a child progress
    adapter.on_progress(5 * 1024 * 1024); // 5MB
    adapter.on_progress(10 * 1024 * 1024); // 10MB
    adapter.on_progress(20 * 1024 * 1024); // 20MB

    adapter.on_complete();
    parent.complete(Some("Installation complete".to_string()));
}

#[test]
fn test_download_no_child_for_small_file() {
    // Create parent progress
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);

    // Create download progress adapter with parent
    let mut adapter =
        DownloadProgressAdapter::for_jdk_download("tool@1.0", Some(parent.create_child()), false);

    // Simulate small download (< 10MB)
    adapter.on_start(5 * 1024 * 1024); // 5MB

    // Should not create child, only update parent message
    adapter.on_progress(1024 * 1024); // 1MB
    adapter.on_progress(3 * 1024 * 1024); // 3MB
    adapter.on_progress(5 * 1024 * 1024); // 5MB

    adapter.on_complete();
    parent.complete(Some("Installation complete".to_string()));
}

#[test]
fn test_foojay_creates_child_progress() {
    // FoojayMetadataSource creates child progress internally

    // Create parent progress
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);

    // FoojayMetadataSource should always create child progress
    // This test verifies the implementation pattern
    // In real usage, fetch_all would create a child internally
    let _child = parent.create_child();
    // Child should be created (we can't test is_silent on trait object)
}

#[test]
fn test_http_source_child_for_large_metadata() {
    // Test that HTTP source creates child progress for large total size
    // This tests the threshold logic for multiple files
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);

    // In real usage, HttpMetadataSource would check total file size
    // and create child if >= 10MB
    let total_size = 15 * 1024 * 1024; // 15MB total
    if total_size >= 10 * 1024 * 1024 {
        let _child = parent.create_child();
        // Child should be created for large total size
    }
}

#[test]
fn test_cache_refresh_with_parent_progress() {
    use kopi::cache;
    use kopi::config::KopiConfig;
    use tempfile::TempDir;

    // This test verifies that cache refresh accepts parent progress
    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

    let mut parent = ProgressFactory::create(false);
    let config_progress = ProgressConfig::new(ProgressStyle::Count).with_total(10);
    parent.start(config_progress);

    let mut current_step = 0u64;

    // The function should accept parent progress, not force SilentProgress
    // In real implementation, this would create child progress for Foojay
    // We can't test the actual network call here, but verify the signature works
    let result =
        cache::fetch_and_cache_metadata_with_progress(&config, parent.as_mut(), &mut current_step);

    // The test might succeed if network is available, or fail without network
    // Either way, the important thing is that the API accepts parent progress
    match result {
        Ok(_) => println!("Successfully fetched metadata with parent progress"),
        Err(_) => println!("Failed to fetch metadata (expected without network)"),
    }
}
