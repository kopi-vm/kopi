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

//! Integration tests for HTTP metadata source
//!
//! These tests run against the real metadata server at https://kopi-vm.github.io/metadata

use kopi::indicator::SilentProgress;
use kopi::metadata::{HttpMetadataSource, MetadataSource};

/// Default test URL - change this to point to your metadata server
const TEST_METADATA_URL: &str = "https://kopi-vm.github.io/metadata";

#[test]
fn test_real_github_pages_fetch_index() {
    let source = HttpMetadataSource::new(TEST_METADATA_URL.to_string());

    // Test availability
    match source.is_available() {
        Ok(available) => {
            if !available {
                eprintln!("Metadata server at {TEST_METADATA_URL} is not available");
                return;
            }
        }
        Err(e) => {
            eprintln!("Failed to check availability: {e}");
            return;
        }
    }

    // Test fetch_all to verify index is working
    let mut progress = SilentProgress;
    match source.fetch_all(&mut progress) {
        Ok(metadata) => {
            println!("Successfully fetched metadata");
            println!("  Total packages: {}", metadata.len());

            // Group by distribution to show what's available
            let mut distributions = std::collections::HashSet::new();
            for jdk in &metadata {
                distributions.insert(&jdk.distribution);
            }

            println!("  Available distributions: {distributions:?}");
            assert!(!metadata.is_empty());
        }
        Err(e) => {
            eprintln!("Failed to fetch metadata: {e}");
            panic!("Test failed");
        }
    }
}

#[test]
fn test_real_github_pages_fetch_all() {
    let source = HttpMetadataSource::new(TEST_METADATA_URL.to_string());

    let mut progress = SilentProgress;
    match source.fetch_all(&mut progress) {
        Ok(metadata) => {
            println!("Successfully fetched {} metadata entries", metadata.len());

            // Print first few entries
            for (i, jdk) in metadata.iter().take(3).enumerate() {
                println!("\nJDK {}:", i + 1);
                println!("  Distribution: {}", jdk.distribution);
                println!("  Version: {}", jdk.version);
                println!("  Architecture: {:?}", jdk.architecture);
                println!("  OS: {:?}", jdk.operating_system);
                println!("  Package Type: {:?}", jdk.package_type);
                println!("  Is Complete: {}", jdk.is_complete());
            }

            // Verify all metadata is marked as complete
            assert!(metadata.iter().all(|m| m.is_complete()));

            // Verify we got platform-specific metadata
            let current_arch = kopi::platform::get_current_architecture();
            let current_os = kopi::platform::get_current_os();

            for jdk in &metadata {
                assert_eq!(jdk.architecture.to_string(), current_arch);
                assert_eq!(jdk.operating_system.to_string(), current_os);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch metadata: {e}");
            eprintln!("Note: This test requires a metadata server at {TEST_METADATA_URL}");
            panic!("Test failed");
        }
    }
}

#[test]
fn test_real_github_pages_fetch_distribution() {
    let source = HttpMetadataSource::new(TEST_METADATA_URL.to_string());

    // Try to fetch temurin distribution
    let mut progress = SilentProgress;
    match source.fetch_distribution("temurin", &mut progress) {
        Ok(metadata) => {
            println!("Successfully fetched {} temurin entries", metadata.len());

            // Verify all are temurin
            assert!(metadata.iter().all(|m| m.distribution == "temurin"));
            assert!(metadata.iter().all(|m| m.is_complete()));

            if !metadata.is_empty() {
                println!("Sample temurin JDK:");
                println!("  Version: {}", metadata[0].version);
                println!("  Download URL: {:?}", metadata[0].download_url);
                println!("  Checksum: {:?}", metadata[0].checksum);
                println!("  Size: {} bytes", metadata[0].size);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch temurin distribution: {e}");
            eprintln!("This might be expected if no temurin metadata exists for your platform");
        }
    }
}

/// Test with a custom metadata server URL
/// Run with: TEST_METADATA_URL=https://your-server.com/metadata cargo test test_custom_server
#[test]
fn test_custom_server() {
    let url = std::env::var("TEST_METADATA_URL").unwrap_or_else(|_| TEST_METADATA_URL.to_string());

    println!("Testing against: {url}");

    let source = HttpMetadataSource::new(url.clone());

    match source.last_updated() {
        Ok(Some(updated)) => {
            println!("Metadata last updated: {updated}");
        }
        Ok(None) => {
            println!("No last updated timestamp available");
        }
        Err(e) => {
            eprintln!("Failed to get last updated: {e}");
        }
    }
}
