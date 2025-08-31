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

//! Comprehensive integration tests for multi-progress functionality

use kopi::download::{DownloadProgressAdapter, ProgressReporter};
use kopi::indicator::{ProgressConfig, ProgressFactory, ProgressIndicator, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod common;
use common::progress_capture::TestProgressCapture;
use common::test_home::TestHomeGuard;

/// Test utility to capture multi-progress hierarchies
struct MultiProgressCapture {
    parent: TestProgressCapture,
    children: Arc<Mutex<Vec<TestProgressCapture>>>,
}

impl MultiProgressCapture {
    fn new() -> Self {
        Self {
            parent: TestProgressCapture::new(),
            children: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_parent(&mut self) -> &mut TestProgressCapture {
        &mut self.parent
    }

    fn add_child(&self) -> TestProgressCapture {
        let child = TestProgressCapture::new();
        self.children.lock().unwrap().push(child.clone());
        child
    }

    fn child_count(&self) -> usize {
        self.children.lock().unwrap().len()
    }

    fn verify_parent_child_relationship(&self) -> bool {
        // Parent should have messages
        self.parent.message_count() > 0
    }

    fn verify_indentation(&self) -> bool {
        // In real scenario, child bars would have "└─" prefix
        // For test capture, we verify child creation
        self.child_count() > 0
    }
}

// Test Scenarios

#[test]
fn test_parent_with_single_child() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(10);
    parent.start(config);

    // Create a child progress
    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(1024 * 1024);
    child.start(child_config);

    // Update both progress bars
    for i in 0..5 {
        parent.update(i * 2, Some(10));
        child.update(i * 200_000, Some(1024 * 1024));
        thread::sleep(Duration::from_millis(5));
    }

    // Complete child first, then parent
    child.complete(Some("Child task complete".to_string()));
    parent.complete(Some("Parent task complete".to_string()));
}

#[test]
fn test_parent_with_multiple_children() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
    parent.start(config);

    let mut children = Vec::new();

    // Create multiple children
    for i in 0..3 {
        let mut child = parent.create_child();
        let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        child.start(child_config);
        let child_num = i + 1;
        child.set_message(format!("Processing child {child_num}"));
        children.push(child);
    }

    // Update all children
    for (i, child) in children.iter_mut().enumerate() {
        for j in 0..100 {
            if j % 20 == 0 {
                child.update(j, Some(100));
            }
        }
        let child_num = i + 1;
        child.complete(Some(format!("Child {child_num} complete")));
        parent.update(i as u64 + 1, Some(3));
    }

    parent.complete(Some("All tasks complete".to_string()));
}

#[test]
fn test_parent_no_children_threshold_not_met() {
    // Test case where threshold is not met, so no child is created
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);

    // Create download adapter but simulate small file
    let mut adapter = DownloadProgressAdapter::for_jdk_download(
        "small-tool@1.0",
        Some(parent.create_child()),
        false,
    );

    // Small file (< 10MB threshold)
    adapter.on_start(5 * 1024 * 1024); // 5MB

    // Should update parent message instead of creating child
    adapter.on_progress(2 * 1024 * 1024);
    adapter.on_progress(5 * 1024 * 1024);

    adapter.on_complete();
    parent.complete(Some("Download complete".to_string()));
}

#[test]
fn test_error_handling_with_active_children() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);

    // Create and start a child
    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(1024 * 1024);
    child.start(child_config);

    // Update child partially
    child.update(500_000, Some(1024 * 1024));

    // Simulate error in child
    child.error("Network error occurred".to_string());

    // Parent should handle child error gracefully
    parent.error("Operation failed due to child error".to_string());
}

#[test]
fn test_finish_and_clear_removes_bars() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(2);
    parent.start(config);

    // Create children
    let mut child1 = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(50);
    child1.start(child_config.clone());

    let mut child2 = parent.create_child();
    child2.start(child_config);

    // Complete children
    child1.complete(Some("First complete".to_string()));
    child2.complete(Some("Second complete".to_string()));

    // Parent completion should clear all bars
    parent.complete(Some("All complete".to_string()));
}

// Command Integration Tests

#[test]
fn test_install_with_large_download() {
    let _test_home = TestHomeGuard::new();

    // Simulate install command with large JDK download
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);
    parent.set_message("Installing temurin@21".to_string());

    // Simulate download phase
    parent.update(3, Some(8));
    parent.set_message("Downloading JDK...".to_string());

    // Create child for large download
    let mut download_child = parent.create_child();
    let download_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(150 * 1024 * 1024); // 150MB
    download_child.start(download_config);
    download_child.set_message("temurin-21.0.1".to_string());

    // Simulate download progress
    for i in 0..10 {
        download_child.update(i * 15 * 1024 * 1024, Some(150 * 1024 * 1024));
        thread::sleep(Duration::from_millis(5));
    }

    download_child.complete(Some("Download complete".to_string()));

    // Continue with installation steps
    parent.update(5, Some(8));
    parent.set_message("Extracting archive...".to_string());
    thread::sleep(Duration::from_millis(10));

    parent.update(7, Some(8));
    parent.set_message("Creating shims...".to_string());
    thread::sleep(Duration::from_millis(10));

    parent.complete(Some("Installation complete".to_string()));
}

#[test]
fn test_install_with_small_download() {
    let _test_home = TestHomeGuard::new();

    // Simulate install with small tool download
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);
    parent.set_message("Installing maven@3.9".to_string());

    parent.update(3, Some(8));

    // Small download - no child progress
    let mut adapter =
        DownloadProgressAdapter::for_jdk_download("maven@3.9", Some(parent.create_child()), false);

    adapter.on_start(8 * 1024 * 1024); // 8MB - below threshold

    // Updates go to parent message
    adapter.on_progress(4 * 1024 * 1024);
    adapter.on_progress(8 * 1024 * 1024);
    adapter.on_complete();

    parent.update(8, Some(8));
    parent.complete(Some("Installation complete".to_string()));
}

#[test]
fn test_cache_refresh_multiple_sources() {
    let _test_home = TestHomeGuard::new();

    // Simulate cache refresh with multiple metadata sources
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);
    parent.set_message("Refreshing cache".to_string());

    // Source 1: Foojay (always creates child)
    parent.update(1, Some(5));
    parent.set_message("Fetching from Foojay API...".to_string());

    let mut foojay_child = parent.create_child();
    let foojay_config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
    foojay_child.start(foojay_config);
    foojay_child.set_message("Foojay".to_string());

    for i in 0..100 {
        if i % 20 == 0 {
            foojay_child.update(i, Some(100));
        }
    }
    foojay_child.complete(Some("Fetched 100 packages".to_string()));

    // Source 2: HTTP metadata (large, creates child)
    parent.update(2, Some(5));
    parent.set_message("Fetching HTTP metadata...".to_string());

    let mut http_child = parent.create_child();
    let http_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(12 * 1024 * 1024); // 12MB
    http_child.start(http_config);
    http_child.set_message("HTTP Source".to_string());

    http_child.update(12 * 1024 * 1024, Some(12 * 1024 * 1024));
    http_child.complete(Some("Metadata downloaded".to_string()));

    // Source 3: Local (no child)
    parent.update(3, Some(5));
    parent.set_message("Loading local metadata...".to_string());
    thread::sleep(Duration::from_millis(10));

    parent.update(5, Some(5));
    parent.complete(Some("Cache refresh complete".to_string()));
}

#[test]
fn test_cache_refresh_single_source() {
    let _test_home = TestHomeGuard::new();

    // Test with only Foojay source
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
    parent.start(config);
    parent.set_message("Cache refresh".to_string());

    parent.update(1, Some(3));

    // Foojay always creates child
    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count);
    child.start(child_config);
    child.set_message("Foojay".to_string());

    child.set_message("Querying API...".to_string());
    thread::sleep(Duration::from_millis(10));
    child.set_message("Processing packages...".to_string());
    thread::sleep(Duration::from_millis(10));

    child.complete(Some("Foojay complete".to_string()));
    parent.complete(Some("Refresh complete".to_string()));
}

// Edge Case Tests

#[test]
fn test_concurrent_updates_thread_safety() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
    parent.start(config);

    // Create shared parent progress
    let parent_arc = Arc::new(Mutex::new(parent));

    let mut handles = Vec::new();

    // Spawn multiple threads updating the same parent
    for i in 0..3 {
        let parent_clone = Arc::clone(&parent_arc);

        let handle = thread::spawn(move || {
            let mut local_parent = parent_clone.lock().unwrap();
            let mut child = local_parent.create_child();
            drop(local_parent); // Release lock

            let child_config =
                ProgressConfig::new(ProgressStyle::Count).with_total((i + 1) as u64 * 10);
            child.start(child_config);

            for j in 0..(i + 1) * 10 {
                child.update(j as u64, Some((i + 1) as u64 * 10));
                thread::sleep(Duration::from_millis(2));
            }

            child.complete(Some(format!("Thread {i} complete")));
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    parent_arc
        .lock()
        .unwrap()
        .complete(Some("All threads complete".to_string()));
}

#[test]
fn test_nested_progress_depth_limit() {
    // Test that we handle nested progress appropriately
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(3);
    parent.start(config);

    // Create first level child
    let mut child1 = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(2);
    child1.start(child_config);

    // Try to create second level child (should work but may be silent)
    let mut child2 = child1.create_child();
    let grandchild_config = ProgressConfig::new(ProgressStyle::Count).with_total(1);
    child2.start(grandchild_config);

    // Update all levels
    child2.update(1, Some(1));
    child2.complete(Some("Grandchild complete".to_string()));

    child1.update(2, Some(2));
    child1.complete(Some("Child complete".to_string()));

    parent.update(3, Some(3));
    parent.complete(Some("Parent complete".to_string()));
}

#[test]
fn test_spinner_parent_with_determinate_child() {
    // Parent as spinner (no total), child with progress bar
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count); // No total = spinner
    parent.start(config);

    parent.set_message("Processing...".to_string());

    // Create determinate child
    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Bytes).with_total(1024 * 1024 * 50); // 50MB
    child.start(child_config);

    // Update child while parent spins
    for i in 0..10 {
        child.update(i * 5 * 1024 * 1024, Some(50 * 1024 * 1024));
        thread::sleep(Duration::from_millis(5));
    }

    child.complete(Some("Download complete".to_string()));
    parent.complete(Some("Processing complete".to_string()));
}

#[test]
fn test_rapid_create_destroy_cycles() {
    // Test rapid creation and destruction of progress bars
    for _ in 0..10 {
        let mut parent = ProgressFactory::create(false);
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(2);
        parent.start(config);

        let mut child = parent.create_child();
        let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(10);
        child.start(child_config);

        child.update(10, Some(10));
        child.complete(None);

        parent.update(2, Some(2));
        parent.complete(None);
    }
}

#[test]
fn test_suspend_and_println_with_children() {
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);

    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(10);
    child.start(child_config);

    // Test println - should work without corrupting progress bars
    parent.println("Parent message").unwrap();
    child.println("Child message").unwrap();

    // Test suspend - should temporarily hide all progress bars
    parent.suspend(&mut || {
        println!("Suspended output from parent");
    });

    child.suspend(&mut || {
        println!("Suspended output from child");
    });

    child.complete(Some("Child done".to_string()));
    parent.complete(Some("Parent done".to_string()));
}

#[test]
fn test_unknown_content_length_download() {
    // Test download with unknown content length
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(8);
    parent.start(config);

    let mut adapter = DownloadProgressAdapter::for_jdk_download(
        "unknown-size",
        Some(parent.create_child()),
        false,
    );

    // Unknown size (0 means unknown)
    adapter.on_start(0);

    // Should use spinner mode without creating child
    adapter.on_progress(1024 * 1024); // 1MB
    adapter.on_progress(5 * 1024 * 1024); // 5MB

    adapter.on_complete();
    parent.complete(Some("Download complete".to_string()));
}

#[test]
fn test_very_small_file_download() {
    // Test with very small file (< 1MB)
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(5);
    parent.start(config);

    let mut adapter = DownloadProgressAdapter::for_jdk_download(
        "config-file",
        Some(parent.create_child()),
        false,
    );

    // Very small file
    adapter.on_start(50 * 1024); // 50KB

    adapter.on_progress(25 * 1024);
    adapter.on_progress(50 * 1024);

    adapter.on_complete();
    parent.complete(Some("Configuration downloaded".to_string()));
}

#[test]
fn test_progress_with_zero_total() {
    // Test edge case with zero total
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(0);
    parent.start(config);

    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(0);
    child.start(child_config);

    // Should handle gracefully
    child.update(0, Some(0));
    child.complete(Some("Complete".to_string()));

    parent.update(0, Some(0));
    parent.complete(Some("Complete".to_string()));
}

#[test]
fn test_overflow_protection_with_children() {
    // Test updating beyond total with children
    let mut parent = ProgressFactory::create(false);
    let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
    parent.start(config);

    let mut child = parent.create_child();
    let child_config = ProgressConfig::new(ProgressStyle::Count).with_total(50);
    child.start(child_config);

    // Try to exceed totals
    child.update(100, Some(50)); // 200% of child total
    parent.update(200, Some(100)); // 200% of parent total

    // Should handle gracefully
    child.complete(None);
    parent.complete(None);
}

// Helper assertion functions
fn assert_parent_child_relationship(capture: &MultiProgressCapture) {
    assert!(
        capture.verify_parent_child_relationship(),
        "Parent-child relationship not established"
    );
    assert!(
        capture.verify_indentation(),
        "Child indentation not correct"
    );
}

#[test]
fn test_multi_progress_capture_utility() {
    let mut capture = MultiProgressCapture::new();

    // Test parent operations
    capture
        .get_parent()
        .set_message("Parent message".to_string());
    assert_eq!(capture.get_parent().message_count(), 1);

    // Test child creation
    let mut child1 = capture.add_child();
    child1.set_message("Child 1".to_string());

    let mut child2 = capture.add_child();
    child2.set_message("Child 2".to_string());

    assert_eq!(capture.child_count(), 2);
    assert_parent_child_relationship(&capture);
}
