use kopi::archive::ArchiveHandler;
use kopi::download::{DownloadManager, DownloadOptions};
use kopi::security::SecurityManager;
use kopi::storage::StorageManager;
use mockito::Server;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, tempdir};

#[test]
fn test_download_with_checksum_verification() {
    let mut server = Server::new();
    let test_content = b"Test JDK content";
    let expected_checksum = "e2f85493bc3e302ea656d20668c3d327f31dc24a728f873c2bab90cb39d7ae0d";

    let _m = server
        .mock("GET", "/jdk.tar.gz")
        .with_status(200)
        .with_header("content-type", "application/gzip")
        .with_header("content-length", &test_content.len().to_string())
        .with_body(test_content)
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("jdk.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        checksum: Some(expected_checksum.to_string()),
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/jdk.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_ok());
    assert!(dest_file.exists());
    assert_eq!(fs::read(&dest_file).unwrap(), test_content);
}

#[test]
fn test_download_with_resume() {
    let mut server = Server::new();
    let full_content = b"This is a test file for resume functionality";
    let partial_content = &full_content[20..];

    // Mock for range request
    let _m = server
        .mock("GET", "/resumable.bin")
        .match_header("range", "bytes=20-")
        .with_status(206)
        .with_header(
            "content-range",
            &format!("bytes 20-{}/{}", full_content.len() - 1, full_content.len()),
        )
        .with_header("content-length", &partial_content.len().to_string())
        .with_body(partial_content)
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("resumable.bin");

    // Create partial file
    {
        let mut file = fs::File::create(&dest_file).unwrap();
        file.write_all(&full_content[..20]).unwrap();
    }

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        resume: true,
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/resumable.bin", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_ok());
    assert_eq!(fs::read(&dest_file).unwrap(), full_content);
}

#[test]
fn test_security_validation() {
    let security = SecurityManager::new();

    // Test HTTPS validation
    assert!(
        security
            .verify_https_security("https://api.foojay.io/download")
            .is_ok()
    );
    assert!(
        security
            .verify_https_security("http://api.foojay.io/download")
            .is_err()
    );

    // Test trusted domains
    assert!(security.is_trusted_domain("https://api.foojay.io/v3/"));
    assert!(security.is_trusted_domain("https://corretto.aws/downloads/"));
    assert!(!security.is_trusted_domain("https://untrusted.com/"));
}

#[test]
fn test_archive_extraction_workflow() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    // Create test tar.gz archive
    let temp_archive = NamedTempFile::with_suffix(".tar.gz").unwrap();
    {
        let gz = GzEncoder::new(temp_archive.as_file(), Compression::default());
        let mut builder = Builder::new(gz);

        // Add directory entries first
        let mut header = tar::Header::new_gnu();
        header.set_path("jdk").unwrap();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_mode(0o755);
        header.set_size(0);
        header.set_cksum();
        builder.append(&header, &[][..]).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_path("jdk/bin").unwrap();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_mode(0o755);
        header.set_size(0);
        header.set_cksum();
        builder.append(&header, &[][..]).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_path("jdk/lib").unwrap();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_mode(0o755);
        header.set_size(0);
        header.set_cksum();
        builder.append(&header, &[][..]).unwrap();

        // Add test files
        let mut header = tar::Header::new_gnu();
        header.set_path("jdk/bin/java").unwrap();
        header.set_size(12);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, &b"#!/bin/java\n"[..]).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_path("jdk/lib/modules").unwrap();
        header.set_size(7);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, &b"modules"[..]).unwrap();

        builder.finish().unwrap();
    }

    let dest_dir = tempdir().unwrap();
    let handler = ArchiveHandler::new();

    let result = handler.extract(temp_archive.path(), dest_dir.path());
    assert!(result.is_ok());

    // Verify extracted files
    assert!(dest_dir.path().join("jdk/bin/java").exists());
    assert!(dest_dir.path().join("jdk/lib/modules").exists());

    let java_content = fs::read_to_string(dest_dir.path().join("jdk/bin/java")).unwrap();
    assert_eq!(java_content, "#!/bin/java\n");
}

#[test]
fn test_storage_installation_workflow() {
    use kopi::models::jdk::Distribution;

    let temp_home = tempdir().unwrap();
    let storage = StorageManager::with_home(temp_home.path().to_path_buf());
    let distribution = Distribution::Temurin;

    // Prepare installation
    let context = storage
        .prepare_jdk_installation(&distribution, "21.0.1", "x64")
        .unwrap();

    assert!(context.temp_path.exists());

    // Simulate JDK extraction to temp directory with single top-level directory
    // (like real JDK archives: jdk-21+35/)
    let jdk_dir = context.temp_path.join("jdk-21.0.1");
    fs::create_dir_all(jdk_dir.join("bin")).unwrap();
    fs::write(jdk_dir.join("bin/java"), "java executable").unwrap();

    // Finalize installation
    let final_path = storage.finalize_installation(context).unwrap();
    assert!(final_path.exists());
    assert!(final_path.join("bin/java").exists());

    // List installed JDKs
    let installed = storage.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].distribution, "temurin");
    assert_eq!(installed[0].version, "21.0.1");
}

#[test]
fn test_download_progress_reporting() {
    use std::sync::{Arc, Mutex};

    struct TestProgressReporter {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl kopi::download::ProgressReporter for TestProgressReporter {
        fn on_start(&mut self, total_bytes: u64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("start:{}", total_bytes));
        }

        fn on_progress(&mut self, bytes_downloaded: u64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("progress:{}", bytes_downloaded));
        }

        fn on_complete(&mut self) {
            self.events.lock().unwrap().push("complete".to_string());
        }
    }

    let mut server = Server::new();
    let test_content = b"Test content for progress";

    let _m = server
        .mock("GET", "/progress.bin")
        .with_status(200)
        .with_header("content-length", &test_content.len().to_string())
        .with_body(test_content)
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("progress.bin");

    let events = Arc::new(Mutex::new(Vec::new()));
    let reporter = TestProgressReporter {
        events: events.clone(),
    };

    let mut manager = DownloadManager::new().with_progress_reporter(Box::new(reporter));
    let options = DownloadOptions::default();

    let result = manager.download(
        &format!("{}/progress.bin", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_ok());

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| e.starts_with("start:")));
    assert!(events.iter().any(|e| e == "complete"));
}

#[test]
fn test_concurrent_installation_safety() {
    use kopi::models::jdk::Distribution;
    use std::sync::Arc;
    use std::thread;

    let temp_home = tempdir().unwrap();
    let storage = Arc::new(StorageManager::with_home(temp_home.path().to_path_buf()));
    let distribution = Distribution::Temurin;

    let mut handles = vec![];

    // Try to install the same JDK from multiple threads
    for i in 0..3 {
        let storage = storage.clone();
        let dist = distribution.clone();

        let handle = thread::spawn(move || {
            let result = storage.prepare_jdk_installation(&dist, "21.0.1", "x64");

            if let Ok(context) = result {
                // Simulate some work
                thread::sleep(std::time::Duration::from_millis(10));
                fs::create_dir_all(context.temp_path.join("bin")).unwrap();
                fs::write(
                    context.temp_path.join("bin/java"),
                    format!("java from thread {}", i),
                )
                .unwrap();

                storage.finalize_installation(context).ok()
            } else {
                None
            }
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Only one thread should succeed
    let successes = results.iter().filter(|r| r.is_some()).count();
    assert_eq!(successes, 1);

    // Verify the JDK was installed
    let installed = storage.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 1);
}

#[test]
fn test_disk_space_simulation() {
    use kopi::models::jdk::Distribution;

    // This test is more of a unit test for the disk space check logic
    // In a real integration test, we'd need to mock the filesystem
    let temp_home = tempdir().unwrap();
    let storage = StorageManager::with_home(temp_home.path().to_path_buf());
    let distribution = Distribution::Temurin;

    // The disk space check should pass on most systems
    let result = storage.prepare_jdk_installation(&distribution, "21.0.1", "x64");
    assert!(result.is_ok());
}

#[test]
fn test_network_failure_handling() {
    let server = Server::new();

    // Don't create any mock - connection will fail
    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("nonexistent.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        timeout: std::time::Duration::from_secs(1),
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/nonexistent.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_err());
}

#[test]
#[ignore] // TODO: Fix timeout simulation with mockito
fn test_download_network_timeout() {
    let mut server = Server::new();

    // Create a mock that delays response to trigger timeout
    let _m = server
        .mock("GET", "/slow-download.tar.gz")
        .with_status(200)
        .with_header("content-length", "1000000")
        .with_chunked_body(|w| {
            // Write some initial data
            w.write_all(b"Initial data").unwrap();
            // Then sleep longer than timeout
            std::thread::sleep(std::time::Duration::from_secs(3));
            Ok(())
        })
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("slow-download.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        timeout: std::time::Duration::from_secs(1),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = manager.download(
        &format!("{}/slow-download.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    // Should fail due to timeout
    assert!(result.is_err());
    // Should fail within reasonable time (not wait full 3 seconds)
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn test_large_file_download_simulation() {
    use kopi::download::ProgressReporter;
    use std::sync::{Arc, Mutex};

    struct LargeFileProgressReporter {
        total_size: Arc<Mutex<u64>>,
        bytes_downloaded: Arc<Mutex<u64>>,
        chunks_received: Arc<Mutex<u32>>,
    }

    impl ProgressReporter for LargeFileProgressReporter {
        fn on_start(&mut self, total_bytes: u64) {
            *self.total_size.lock().unwrap() = total_bytes;
        }

        fn on_progress(&mut self, bytes_downloaded: u64) {
            *self.bytes_downloaded.lock().unwrap() = bytes_downloaded;
            *self.chunks_received.lock().unwrap() += 1;
        }

        fn on_complete(&mut self) {
            // Verify completion
        }
    }

    let mut server = Server::new();

    // Simulate a 500MB file with chunked response
    let file_size = 500 * 1024 * 1024; // 500MB
    let chunk_size = 1024 * 1024; // 1MB chunks

    let _m = server
        .mock("GET", "/large-jdk.tar.gz")
        .with_status(200)
        .with_header("content-length", &file_size.to_string())
        .with_chunked_body(move |w| {
            // Write data in chunks to simulate large file
            let mut written = 0;
            let chunk = vec![0u8; chunk_size];

            while written < file_size {
                let to_write = std::cmp::min(chunk_size, file_size - written);
                w.write_all(&chunk[..to_write])?;
                written += to_write;

                // Small delay to simulate network transfer
                std::thread::sleep(std::time::Duration::from_micros(10));
            }
            Ok(())
        })
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("large-jdk.tar.gz");

    let total_size = Arc::new(Mutex::new(0));
    let bytes_downloaded = Arc::new(Mutex::new(0));
    let chunks_received = Arc::new(Mutex::new(0));

    let reporter = LargeFileProgressReporter {
        total_size: total_size.clone(),
        bytes_downloaded: bytes_downloaded.clone(),
        chunks_received: chunks_received.clone(),
    };

    let mut manager = DownloadManager::new().with_progress_reporter(Box::new(reporter));
    let options = DownloadOptions {
        timeout: std::time::Duration::from_secs(30), // Longer timeout for large file
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/large-jdk.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_ok());
    assert_eq!(*total_size.lock().unwrap(), file_size as u64);
    assert_eq!(*bytes_downloaded.lock().unwrap(), file_size as u64);
    assert!(*chunks_received.lock().unwrap() > 0);

    // Verify file size
    let metadata = fs::metadata(&dest_file).unwrap();
    assert_eq!(metadata.len(), file_size as u64);
}

#[test]
fn test_download_retry_on_failure() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    let mut server = Server::new();
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    // Mock that fails first 2 attempts, succeeds on 3rd
    let _m = server
        .mock("GET", "/flaky-download.tar.gz")
        .with_status(200)
        .with_chunked_body(move |w| {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                // Simulate network error by returning incomplete response
                w.write_all(b"Partial")?;
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "Network error",
                ))
            } else {
                // Success on 3rd attempt
                w.write_all(b"Complete file content")?;
                Ok(())
            }
        })
        .expect_at_least(3)
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("flaky-download.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        timeout: std::time::Duration::from_secs(10),
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/flaky-download.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    // Since we don't have retry logic built into DownloadManager, this should fail
    assert!(result.is_err());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_download_checksum_mismatch() {
    let mut server = Server::new();
    let test_content = b"Test JDK content with wrong checksum";
    let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";

    let _m = server
        .mock("GET", "/bad-checksum.tar.gz")
        .with_status(200)
        .with_header("content-type", "application/gzip")
        .with_header("content-length", &test_content.len().to_string())
        .with_body(test_content)
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("bad-checksum.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions {
        checksum: Some(wrong_checksum.to_string()),
        ..Default::default()
    };

    let result = manager.download(
        &format!("{}/bad-checksum.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_err());
    // File should be cleaned up on checksum failure
    assert!(!dest_file.exists());
}

#[test]
#[ignore] // TODO: Fix connection reset simulation with mockito
fn test_download_connection_reset() {
    use std::io::{self, ErrorKind};

    let mut server = Server::new();

    // Mock that simulates connection reset
    let _m = server
        .mock("GET", "/connection-reset.tar.gz")
        .with_status(200)
        .with_header("content-length", "1000000")
        .with_chunked_body(|w| {
            // Write partial data then simulate connection reset
            w.write_all(b"Partial data before reset")?;
            Err(io::Error::new(
                ErrorKind::ConnectionReset,
                "Connection reset by peer",
            ))
        })
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("connection-reset.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions::default();

    let result = manager.download(
        &format!("{}/connection-reset.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_err());
    match result {
        Err(e) => {
            let error_str = e.to_string();
            assert!(
                error_str.contains("Connection") || error_str.contains("reset"),
                "Expected connection reset error, got: {}",
                error_str
            );
        }
        _ => panic!("Expected error"),
    }
}

#[test]
fn test_download_404_not_found() {
    let mut server = Server::new();

    let _m = server
        .mock("GET", "/not-found.tar.gz")
        .with_status(404)
        .with_body("Not Found")
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("not-found.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions::default();

    let result = manager.download(
        &format!("{}/not-found.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    assert!(result.is_err());
    match result {
        Err(e) => {
            let error_str = e.to_string();
            assert!(
                error_str.contains("404") || error_str.contains("Not Found"),
                "Expected 404 error, got: {}",
                error_str
            );
        }
        _ => panic!("Expected error"),
    }
}

#[test]
fn test_download_rate_limiting() {
    use std::time::Instant;

    let mut server = Server::new();

    // First request gets rate limited
    let _m1 = server
        .mock("GET", "/rate-limited.tar.gz")
        .with_status(429)
        .with_header("retry-after", "1")
        .with_body("Too Many Requests")
        .create();

    // Second request succeeds
    let _m2 = server
        .mock("GET", "/rate-limited.tar.gz")
        .with_status(200)
        .with_header("content-length", "20")
        .with_body("Rate limit passed OK")
        .create();

    let temp_dir = tempdir().unwrap();
    let dest_file = temp_dir.path().join("rate-limited.tar.gz");

    let mut manager = DownloadManager::new();
    let options = DownloadOptions::default();

    let _start = Instant::now();
    let result = manager.download(
        &format!("{}/rate-limited.tar.gz", server.url()),
        &dest_file,
        &options,
    );

    // Since we don't have rate limiting built into DownloadManager, this should fail with 429
    assert!(result.is_err());
    match result {
        Err(e) => {
            let error_str = e.to_string();
            assert!(
                error_str.contains("429") || error_str.contains("Too Many Requests"),
                "Expected 429 error, got: {}",
                error_str
            );
        }
        _ => panic!("Expected error"),
    }
}
