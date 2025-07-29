#[cfg(test)]
mod http_tests {
    use super::super::*;
    use crate::models::metadata::JdkMetadata;
    use crate::models::package::{ArchiveType, PackageType};
    use crate::models::platform::{Architecture, OperatingSystem};
    use crate::version::Version;
    use mockito::Server;

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
            checksum_type: Some(crate::models::package::ChecksumType::Sha256),
            size: 100000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
            is_complete: true,
        }
    }

    fn create_test_index() -> IndexFile {
        let current_arch = crate::platform::get_current_architecture();
        let current_os = crate::platform::get_current_os();
        let current_libc = crate::platform::get_foojay_libc_type();

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
                    size: 45678,
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
                    size: 48900,
                    checksum: Some("sha256:def456".to_string()),
                    last_modified: Some("2024-01-15T09:00:00Z".to_string()),
                },
            ],
            generator_config: None,
        }
    }

    #[test]
    fn test_fetch_index_success() {
        let mut server = Server::new();
        let index = create_test_index();

        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.fetch_index();

        assert!(result.is_ok());
        let fetched_index = result.unwrap();
        assert_eq!(fetched_index.version, 2);
        assert_eq!(fetched_index.files.len(), 2);
    }

    #[test]
    fn test_fetch_index_http_error() {
        let mut server = Server::new();

        let _m = server.mock("GET", "/index.json").with_status(404).create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.fetch_index();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 404"));
    }

    #[test]
    fn test_is_available() {
        let mut server = Server::new();
        let index = create_test_index();

        // Test when available
        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        let source = HttpMetadataSource::new(server.url());
        assert!(source.is_available().unwrap());

        // Test when not available
        let mut server2 = Server::new();
        let _m2 = server2.mock("GET", "/index.json").with_status(500).create();

        let source2 = HttpMetadataSource::new(server2.url());
        assert!(!source2.is_available().unwrap());
    }

    #[test]
    fn test_fetch_all() {
        let mut server = Server::new();
        let index = create_test_index();
        let metadata = vec![create_test_metadata()];

        // Mock index.json
        let _m1 = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
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
        let result = source.fetch_all();

        assert!(result.is_ok());
        let all_metadata = result.unwrap();
        assert_eq!(all_metadata.len(), 2); // Both files fetched
        assert!(all_metadata.iter().all(|m| m.is_complete));
    }

    #[test]
    fn test_fetch_distribution() {
        let mut server = Server::new();
        let index = create_test_index();
        let metadata = vec![create_test_metadata()];

        // Mock index.json
        let _m1 = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        // Mock only temurin metadata file
        let _m2 = server
            .mock("GET", "/jdks/temurin-test.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&metadata).unwrap())
            .create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.fetch_distribution("temurin");

        assert!(result.is_ok());
        let dist_metadata = result.unwrap();
        assert_eq!(dist_metadata.len(), 1);
        assert_eq!(dist_metadata[0].distribution, "temurin");
    }

    #[test]
    fn test_fetch_package_details_not_supported() {
        let source = HttpMetadataSource::new("https://example.com".to_string());
        let result = source.fetch_package_details("test-id");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("complete metadata")
        );
    }

    #[test]
    fn test_last_updated() {
        let mut server = Server::new();
        let index = create_test_index();

        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.last_updated();

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().to_rfc3339(), "2024-01-15T10:00:00+00:00");
    }

    #[test]
    fn test_partial_fetch_failure() {
        let mut server = Server::new();
        let index = create_test_index();
        let metadata = vec![create_test_metadata()];

        // Mock index.json
        let _m1 = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        // Mock one successful and one failed metadata file
        let _m2 = server
            .mock("GET", "/jdks/temurin-test.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&metadata).unwrap())
            .create();

        let _m3 = server
            .mock("GET", "/jdks/corretto-test.json")
            .with_status(500)
            .create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.fetch_all();

        // Should succeed with partial results
        assert!(result.is_ok());
        let all_metadata = result.unwrap();
        assert_eq!(all_metadata.len(), 1); // Only successful file
    }

    #[test]
    fn test_platform_filtering_excludes_wrong_platform() {
        let mut server = Server::new();

        // Create index with wrong platform files
        let mut index = create_test_index();
        index.files = vec![IndexFileEntry {
            path: "jdks/wrong-arch.json".to_string(),
            distribution: "temurin".to_string(),
            architectures: Some(vec!["s390x".to_string()]), // Wrong arch
            operating_systems: Some(vec![crate::platform::get_current_os()]),
            lib_c_types: None,
            size: 45678,
            checksum: None,
            last_modified: None,
        }];

        let _m = server
            .mock("GET", "/index.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&index).unwrap())
            .create();

        let source = HttpMetadataSource::new(server.url());
        let result = source.fetch_all();

        assert!(result.is_ok());
        let all_metadata = result.unwrap();
        assert_eq!(all_metadata.len(), 0); // No files for current platform
    }

    // Test JSON serialization/deserialization format
    #[test]
    fn test_metadata_json_serialization() {
        let metadata = create_test_metadata();

        // Serialize to JSON and back
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: JdkMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.distribution, "temurin");
        assert_eq!(deserialized.version.major(), 21);
        assert!(deserialized.download_url.is_some());
    }
}
