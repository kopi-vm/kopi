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

//! Integration tests for metadata sources with child progress support

mod common;

use common::progress_capture::TestProgressCapture;
use kopi::indicator::{ProgressConfig, ProgressIndicator};
use kopi::metadata::{FoojayMetadataSource, HttpMetadataSource, LocalDirectorySource};
use kopi::metadata::{IndexFile, IndexFileEntry, MetadataSource};
use kopi::models::metadata::JdkMetadata;
use kopi::models::package::{ArchiveType, PackageType};
use kopi::models::platform::{Architecture, OperatingSystem};
use kopi::version::Version;
use mockito::Server;
use tempfile::TempDir;

/// Helper to create test metadata for mock responses
fn create_test_metadata() -> JdkMetadata {
    JdkMetadata {
        id: "test-jdk-1".to_string(),
        distribution: "temurin".to_string(),
        version: Version::new(21, 0, 1),
        distribution_version: Version::new(21, 0, 1),
        architecture: Architecture::X64,
        operating_system: OperatingSystem::Linux,
        package_type: PackageType::Jdk,
        archive_type: ArchiveType::TarGz,
        download_url: Some("https://example.com/download.tar.gz".to_string()),
        checksum: Some("abc123".to_string()),
        checksum_type: Some(kopi::models::package::ChecksumType::Sha256),
        size: 100_000_000,
        lib_c_type: Some("glibc".to_string()),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    }
}

/// Helper to create a test index for HTTP source
fn create_test_index_with_size(total_size: u64) -> IndexFile {
    let current_arch = kopi::platform::get_current_architecture();
    let current_os = kopi::platform::get_current_os();
    let current_libc = kopi::platform::get_foojay_libc_type();

    // Split size between two files
    let file1_size = total_size / 2;
    let file2_size = total_size - file1_size;

    IndexFile {
        version: 2,
        updated: "2024-01-15T10:00:00Z".to_string(),
        files: vec![
            IndexFileEntry {
                path: "jdks/temurin-test.json".to_string(),
                distribution: "temurin".to_string(),
                architectures: Some(vec![current_arch.clone()]),
                operating_systems: Some(vec![current_os.clone()]),
                lib_c_types: if current_os.as_str() == "linux" {
                    Some(vec![current_libc.to_string()])
                } else {
                    None
                },
                size: file1_size,
                checksum: Some("sha256:abc123".to_string()),
                last_modified: Some("2024-01-15T09:00:00Z".to_string()),
            },
            IndexFileEntry {
                path: "jdks/corretto-test.json".to_string(),
                distribution: "corretto".to_string(),
                architectures: Some(vec![current_arch]),
                operating_systems: Some(vec![current_os.clone()]),
                lib_c_types: if current_os == "linux" {
                    Some(vec![current_libc.to_string()])
                } else {
                    None
                },
                size: file2_size,
                checksum: Some("sha256:def456".to_string()),
                last_modified: Some("2024-01-15T09:00:00Z".to_string()),
            },
        ],
        generator_config: None,
    }
}

/// Helper struct to track child progress creation
struct ChildProgressTracker {
    parent: TestProgressCapture,
    child_created: bool,
}

impl ChildProgressTracker {
    fn new() -> Self {
        Self {
            parent: TestProgressCapture::new(),
            child_created: false,
        }
    }
}

impl ProgressIndicator for ChildProgressTracker {
    fn start(&mut self, config: ProgressConfig) {
        self.parent.start(config);
    }

    fn update(&mut self, current: u64, total: Option<u64>) {
        self.parent.update(current, total);
    }

    fn set_message(&mut self, message: String) {
        self.parent.set_message(message);
    }

    fn complete(&mut self, message: Option<String>) {
        self.parent.complete(message);
    }

    fn success(&self, message: &str) -> std::io::Result<()> {
        self.parent.success(message)
    }

    fn error(&mut self, message: String) {
        self.parent.error(message);
    }

    fn create_child(&mut self) -> Box<dyn ProgressIndicator> {
        self.child_created = true;
        let child = TestProgressCapture::new();
        Box::new(child)
    }

    fn suspend(&self, f: &mut dyn FnMut()) {
        self.parent.suspend(f);
    }

    fn println(&self, message: &str) -> std::io::Result<()> {
        self.parent.println(message)
    }
}

#[test]
fn test_foojay_always_creates_child_progress() {
    // Skip this test in CI as it requires network access
    if std::env::var("CI").is_ok() || std::env::var("SKIP_NETWORK_TESTS").is_ok() {
        return;
    }

    let source = FoojayMetadataSource::new();
    let mut tracker = ChildProgressTracker::new();

    // Even though we can't actually fetch from Foojay API in tests,
    // we can verify that it would attempt to create a child
    // by checking if the child creation happens early in fetch_all

    // This will fail due to network, but we can check the child was created
    let _ = source.fetch_all(&mut tracker);

    // Foojay should always create a child progress
    assert!(
        tracker.child_created,
        "Foojay should always create child progress"
    );
}

#[test]
fn test_http_creates_child_for_large_files() {
    let mut server = Server::new();

    // Create index with total size > 10MB
    let large_index = create_test_index_with_size(11 * 1024 * 1024); // 11MB
    let metadata = vec![create_test_metadata()];

    // Mock index.json
    let _m1 = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&large_index).unwrap())
        .create();

    // Mock metadata files
    let _m2 = server
        .mock("GET", "/jdks/temurin-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let _m3 = server
        .mock("GET", "/jdks/corretto-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let source = HttpMetadataSource::new(server.url());
    let mut tracker = ChildProgressTracker::new();

    let result = source.fetch_all(&mut tracker);
    assert!(result.is_ok());

    // HTTP should create child progress for files >= 10MB
    assert!(
        tracker.child_created,
        "HTTP should create child progress for large files"
    );
}

#[test]
fn test_http_always_creates_child_progress() {
    let mut server = Server::new();

    // Create index with any size (HTTP always creates child progress now)
    let small_index = create_test_index_with_size(5 * 1024 * 1024); // 5MB example
    let metadata = vec![create_test_metadata()];

    // Mock index.json
    let _m1 = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&small_index).unwrap())
        .create();

    // Mock metadata files
    let _m2 = server
        .mock("GET", "/jdks/temurin-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let _m3 = server
        .mock("GET", "/jdks/corretto-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let source = HttpMetadataSource::new(server.url());
    let mut tracker = ChildProgressTracker::new();

    let result = source.fetch_all(&mut tracker);
    assert!(result.is_ok());

    // HTTP should always create child progress (regardless of size)
    assert!(
        tracker.child_created,
        "HTTP should always create child progress"
    );
}

#[test]
fn test_local_never_creates_child_progress() {
    let temp_dir = TempDir::new().unwrap();
    let metadata_dir = temp_dir.path().join("metadata");
    std::fs::create_dir_all(&metadata_dir).unwrap();

    // Create a minimal index.json
    let index = IndexFile {
        version: 2,
        updated: "2024-01-15T10:00:00Z".to_string(),
        files: vec![], // Empty for this test
        generator_config: None,
    };

    let index_path = metadata_dir.join("index.json");
    std::fs::write(&index_path, serde_json::to_string(&index).unwrap()).unwrap();

    let source = LocalDirectorySource::new(metadata_dir);
    let mut tracker = ChildProgressTracker::new();

    let result = source.fetch_all(&mut tracker);
    assert!(result.is_ok());

    // Local should NEVER create child progress
    assert!(
        !tracker.child_created,
        "Local should never create child progress"
    );

    // Should update parent message directly
    assert!(tracker.parent.contains_message("local"));
}

#[test]
fn test_http_distribution_with_large_files() {
    let mut server = Server::new();

    // Create index with a single large temurin file (>10MB)
    let current_arch = kopi::platform::get_current_architecture();
    let current_os = kopi::platform::get_current_os();
    let current_libc = kopi::platform::get_foojay_libc_type();

    let large_index = IndexFile {
        version: 2,
        updated: "2024-01-15T10:00:00Z".to_string(),
        files: vec![IndexFileEntry {
            path: "jdks/temurin-test.json".to_string(),
            distribution: "temurin".to_string(),
            architectures: Some(vec![current_arch]),
            operating_systems: Some(vec![current_os.clone()]),
            lib_c_types: if current_os.as_str() == "linux" {
                Some(vec![current_libc.to_string()])
            } else {
                None
            },
            size: 12 * 1024 * 1024, // 12MB - above the 10MB threshold
            checksum: Some("sha256:abc123".to_string()),
            last_modified: Some("2024-01-15T09:00:00Z".to_string()),
        }],
        generator_config: None,
    };

    let metadata = vec![create_test_metadata()];

    // Mock index.json
    let _m1 = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&large_index).unwrap())
        .create();

    // Mock only temurin metadata file
    let _m2 = server
        .mock("GET", "/jdks/temurin-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let source = HttpMetadataSource::new(server.url());
    let mut tracker = ChildProgressTracker::new();

    let result = source.fetch_distribution("temurin", &mut tracker);
    assert!(result.is_ok());

    // Should create child progress for large distribution files
    assert!(
        tracker.child_created,
        "HTTP should create child progress for large distribution files"
    );
}

#[test]
fn test_cache_module_with_child_progress() {
    use kopi::cache::fetch_and_cache_metadata_with_progress;
    use kopi::config::KopiConfig;

    // Skip this test in CI as it requires network access
    if std::env::var("CI").is_ok() || std::env::var("SKIP_NETWORK_TESTS").is_ok() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

    let mut tracker = ChildProgressTracker::new();
    let mut current_step = 0;

    // This will try to fetch from configured sources
    // Default config uses Foojay which always creates child progress
    let _ = fetch_and_cache_metadata_with_progress(&config, &mut tracker, &mut current_step);

    // The cache module should pass through child progress creation
    // from the underlying metadata source (Foojay by default)
    assert!(
        tracker.child_created || tracker.parent.message_count() > 0,
        "Cache module should support child progress from sources"
    );
}

#[test]
fn test_progress_message_flow() {
    let mut server = Server::new();

    // Create a small index (no child progress)
    let small_index = create_test_index_with_size(1024 * 1024); // 1MB
    let metadata = vec![create_test_metadata()];

    // Mock responses
    let _m1 = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&small_index).unwrap())
        .create();

    let _m2 = server
        .mock("GET", "/jdks/temurin-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let _m3 = server
        .mock("GET", "/jdks/corretto-test.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&metadata).unwrap())
        .create();

    let source = HttpMetadataSource::new(server.url());
    let mut capture = TestProgressCapture::new();

    let result = source.fetch_all(&mut capture);
    assert!(result.is_ok());

    // Verify progress messages were sent
    assert!(capture.message_count() > 0, "Should have progress messages");
    assert!(
        capture.contains_message("metadata"),
        "Should mention metadata"
    );
    assert!(
        capture.contains_message("HTTP"),
        "Should mention HTTP source"
    );
}
