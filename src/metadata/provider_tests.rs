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

#[cfg(test)]
mod tests {
    use crate::config::{MetadataConfig, SourceConfig};
    use crate::error::{KopiError, Result};
    use crate::metadata::{MetadataProvider, MetadataSource, PackageDetails, SourceHealth};
    use crate::models::metadata::JdkMetadata;
    use crate::models::package::{ArchiveType, ChecksumType, PackageType};
    use crate::models::platform::{Architecture, OperatingSystem};
    use crate::version::Version;
    use std::sync::{Arc, Mutex};

    /// Mock metadata source for testing
    struct MockMetadataSource {
        id: String,
        name: String,
        available: Arc<Mutex<bool>>,
        fetch_all_result: Arc<Mutex<Result<Vec<JdkMetadata>>>>,
        fetch_distribution_result: Arc<Mutex<Result<Vec<JdkMetadata>>>>,
        fetch_package_details_result: Arc<Mutex<Result<PackageDetails>>>,
    }

    impl MockMetadataSource {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                available: Arc::new(Mutex::new(true)),
                fetch_all_result: Arc::new(Mutex::new(Ok(vec![]))),
                fetch_distribution_result: Arc::new(Mutex::new(Ok(vec![]))),
                fetch_package_details_result: Arc::new(Mutex::new(Ok(PackageDetails {
                    download_url: "https://example.com/download".to_string(),
                    checksum: Some("abc123".to_string()),
                    checksum_type: Some(ChecksumType::Sha256),
                }))),
            }
        }

        fn set_available(&self, available: bool) {
            *self.available.lock().unwrap() = available;
        }

        fn set_fetch_all_result(&self, result: Result<Vec<JdkMetadata>>) {
            *self.fetch_all_result.lock().unwrap() = result;
        }

        fn set_fetch_distribution_result(&self, result: Result<Vec<JdkMetadata>>) {
            *self.fetch_distribution_result.lock().unwrap() = result;
        }

        fn set_fetch_package_details_result(&self, result: Result<PackageDetails>) {
            *self.fetch_package_details_result.lock().unwrap() = result;
        }
    }

    impl MetadataSource for Arc<MockMetadataSource> {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn is_available(&self) -> Result<bool> {
            Ok(*self.available.lock().unwrap())
        }

        fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
            let locked = self.fetch_all_result.lock().unwrap();
            match &*locked {
                Ok(vec) => Ok(vec.clone()),
                Err(_) => Err(KopiError::MetadataFetch("Mock error".to_string())),
            }
        }

        fn fetch_distribution(&self, _distribution: &str) -> Result<Vec<JdkMetadata>> {
            let locked = self.fetch_distribution_result.lock().unwrap();
            match &*locked {
                Ok(vec) => Ok(vec.clone()),
                Err(_) => Err(KopiError::MetadataFetch("Mock error".to_string())),
            }
        }

        fn fetch_package_details(&self, _package_id: &str) -> Result<PackageDetails> {
            let locked = self.fetch_package_details_result.lock().unwrap();
            match &*locked {
                Ok(details) => Ok(details.clone()),
                Err(_) => Err(KopiError::MetadataFetch("Mock error".to_string())),
            }
        }

        fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
            Ok(None)
        }
    }

    fn create_test_metadata(id: &str, complete: bool) -> JdkMetadata {
        JdkMetadata {
            id: id.to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::new(21, 0, 1),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: if complete {
                Some("https://example.com/download".to_string())
            } else {
                None
            },
            checksum: if complete {
                Some("abc123".to_string())
            } else {
                None
            },
            checksum_type: if complete {
                Some(ChecksumType::Sha256)
            } else {
                None
            },
            size: 100_000_000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        }
    }

    #[test]
    fn test_fallback_when_primary_fails() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set primary to fail
        primary.set_available(false);

        // Set fallback to return test data
        let test_metadata = vec![create_test_metadata("test1", true)];
        fallback.set_fetch_all_result(Ok(test_metadata.clone()));

        // Create provider with both sources in order
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Fetch should use fallback
        let result = provider.fetch_all().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "test1");
    }

    #[test]
    fn test_both_sources_fail() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set both to fail
        primary.set_available(false);
        fallback.set_available(false);

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Fetch should fail
        let result = provider.fetch_all();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KopiError::MetadataFetch(_)));
    }

    #[test]
    fn test_no_fallback_configured() {
        // Create mock source
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        primary.set_available(false);

        // Create provider without fallback
        let provider = MetadataProvider {
            sources: vec![("primary".to_string(), Box::new(primary.clone()))],
        };

        // Fetch should fail with primary error
        let result = provider.fetch_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_for_distribution() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set primary to fail for distribution
        primary.set_fetch_distribution_result(Err(KopiError::MetadataFetch(
            "Primary failed".to_string(),
        )));

        // Set fallback to return test data
        let test_metadata = vec![create_test_metadata("test1", true)];
        fallback.set_fetch_distribution_result(Ok(test_metadata.clone()));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Fetch distribution should use fallback
        let result = provider.fetch_distribution("temurin").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "test1");
    }

    #[test]
    fn test_fallback_for_package_details() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set primary to fail for package details
        primary.set_fetch_package_details_result(Err(KopiError::MetadataFetch(
            "Primary failed".to_string(),
        )));

        // Set fallback to return test data
        let test_details = PackageDetails {
            download_url: "https://fallback.com/download".to_string(),
            checksum: Some("xyz789".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
        };
        fallback.set_fetch_package_details_result(Ok(test_details.clone()));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Ensure complete should use fallback for package details
        let mut metadata = create_test_metadata("test1", false);
        let result = provider.ensure_complete(&mut metadata);
        assert!(result.is_ok());
        assert_eq!(
            metadata.download_url,
            Some("https://fallback.com/download".to_string())
        );
        assert_eq!(metadata.checksum, Some("xyz789".to_string()));
    }

    #[test]
    fn test_source_health_checking() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set primary as unavailable
        primary.set_available(false);

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Check health
        let health = provider.check_sources_health();
        assert_eq!(health.len(), 2);

        match health.get("primary").unwrap() {
            SourceHealth::Unavailable(_) => {}
            _ => panic!("Primary should be unavailable"),
        }

        match health.get("fallback").unwrap() {
            SourceHealth::Available => {}
            _ => panic!("Fallback should be available"),
        }
    }

    #[test]
    fn test_from_config() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();

        // Create metadata config
        let sources = vec![
            SourceConfig::Http {
                name: "primary-http".to_string(),
                enabled: true,
                base_url: "https://example.com/metadata".to_string(),
                cache_locally: true,
                timeout_secs: 30,
            },
            SourceConfig::Local {
                name: "local-backup".to_string(),
                enabled: true,
                directory: "/tmp/metadata".to_string(),
                archive_pattern: "*.tar.gz".to_string(),
                cache_extracted: true,
            },
            SourceConfig::Foojay {
                name: "foojay-api".to_string(),
                enabled: false,
                base_url: "https://api.foojay.io/disco".to_string(),
                timeout_secs: 30,
            },
        ];

        let metadata_config = MetadataConfig {
            cache: Default::default(),
            sources,
        };

        // Create provider from config
        let result = MetadataProvider::from_metadata_config(&metadata_config, kopi_home);
        assert!(result.is_ok());

        let provider = result.unwrap();
        let sources = provider.list_sources();
        assert_eq!(sources.len(), 2); // Only enabled sources
        assert!(sources.contains(&"primary-http"));
        assert!(sources.contains(&"local-backup"));
        assert!(!sources.contains(&"foojay-api")); // Disabled
    }

    #[test]
    fn test_from_config_no_enabled_sources() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();

        // Create config with no enabled sources
        let sources = vec![SourceConfig::Foojay {
            name: "foojay-api".to_string(),
            enabled: false,
            base_url: "https://api.foojay.io/disco".to_string(),
            timeout_secs: 30,
        }];

        let metadata_config = MetadataConfig {
            cache: Default::default(),
            sources,
        };

        // Create provider from config should fail
        let result = MetadataProvider::from_metadata_config(&metadata_config, kopi_home);
        assert!(result.is_err());
        match result {
            Err(KopiError::InvalidConfig(_)) => {}
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_multiple_http_sources() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();

        // Create config with multiple HTTP sources
        let sources = vec![
            SourceConfig::Http {
                name: "primary-cdn".to_string(),
                enabled: true,
                base_url: "https://cdn1.example.com/metadata".to_string(),
                cache_locally: true,
                timeout_secs: 30,
            },
            SourceConfig::Http {
                name: "secondary-cdn".to_string(),
                enabled: true,
                base_url: "https://cdn2.example.com/metadata".to_string(),
                cache_locally: true,
                timeout_secs: 30,
            },
            SourceConfig::Local {
                name: "local-fallback".to_string(),
                enabled: true,
                directory: "/tmp/metadata".to_string(),
                archive_pattern: "*.tar.gz".to_string(),
                cache_extracted: true,
            },
        ];

        let metadata_config = MetadataConfig {
            cache: Default::default(),
            sources,
        };

        // Create provider from config
        let result = MetadataProvider::from_metadata_config(&metadata_config, kopi_home);
        assert!(result.is_ok());

        let provider = result.unwrap();
        let sources = provider.list_sources();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0], "primary-cdn");
        assert_eq!(sources[1], "secondary-cdn");
        assert_eq!(sources[2], "local-fallback");
    }

    #[test]
    fn test_network_error_with_fallback() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Set primary to return network error
        primary.set_fetch_all_result(Err(KopiError::NetworkError(
            "Connection timeout".to_string(),
        )));

        // Set fallback to return test data
        let test_metadata = vec![create_test_metadata("test1", true)];
        fallback.set_fetch_all_result(Ok(test_metadata.clone()));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Should fallback successfully
        let result = provider.fetch_all().unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_partial_fetch_failure() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let fallback = Arc::new(MockMetadataSource::new("fallback", "Fallback Source"));

        // Primary returns data but fails on package details
        let test_metadata = vec![create_test_metadata("test1", false)];
        primary.set_fetch_all_result(Ok(test_metadata.clone()));
        primary.set_fetch_package_details_result(Err(KopiError::MetadataFetch(
            "Details not available".to_string(),
        )));

        // Fallback has the details
        fallback.set_fetch_package_details_result(Ok(PackageDetails {
            download_url: "https://fallback.com/jdk.tar.gz".to_string(),
            checksum: Some("fallback123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
        }));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![
                ("primary".to_string(), Box::new(primary.clone())),
                ("fallback".to_string(), Box::new(fallback.clone())),
            ],
        };

        // Fetch initial data from primary
        let mut result = provider.fetch_all().unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result[0].is_complete());

        // Ensure complete should use fallback for details
        provider.ensure_complete(&mut result[0]).unwrap();
        assert!(result[0].is_complete());
        assert_eq!(
            result[0].download_url,
            Some("https://fallback.com/jdk.tar.gz".to_string())
        );
    }

    #[test]
    fn test_empty_metadata_handling() {
        // Create mock source that returns empty data
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        primary.set_fetch_all_result(Ok(vec![]));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![("primary".to_string(), Box::new(primary.clone()))],
        };

        // Should return empty vector, not error
        let result = provider.fetch_all().unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_batch_ensure_complete_partial_failure() {
        // Create mock sources
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));

        // Set up to fail on specific package ID
        primary.set_fetch_package_details_result(Err(KopiError::MetadataFetch(
            "Package not found".to_string(),
        )));

        // Create provider
        let provider = MetadataProvider {
            sources: vec![("primary".to_string(), Box::new(primary.clone()))],
        };

        // Create batch of incomplete metadata
        let mut metadata_list = vec![
            create_test_metadata("test1", false),
            create_test_metadata("test2", false),
        ];

        // Batch ensure should fail on first error
        let result = provider.ensure_complete_batch(&mut metadata_list);
        assert!(result.is_err());
    }

    #[test]
    fn test_concurrent_source_access() {
        use std::thread;

        // Create thread-safe mock source
        let primary = Arc::new(MockMetadataSource::new("primary", "Primary Source"));
        let test_metadata = vec![create_test_metadata("test1", true)];
        primary.set_fetch_all_result(Ok(test_metadata.clone()));

        // Create provider
        let provider = Arc::new(MetadataProvider {
            sources: vec![("primary".to_string(), Box::new(primary.clone()))],
        });

        // Spawn multiple threads accessing the provider
        let mut handles = vec![];
        for i in 0..5 {
            let provider_clone = provider.clone();
            let handle = thread::spawn(move || {
                let result = provider_clone.fetch_all();
                assert!(result.is_ok());
                let data = result.unwrap();
                assert_eq!(data.len(), 1);
                assert_eq!(data[0].id, "test1");
                i
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            let thread_id = handle.join().unwrap();
            assert!(thread_id < 5);
        }
    }

    #[test]
    fn test_kopi_home_expansion() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kopi_home = temp_dir.path();

        // Create config with ${KOPI_HOME} in path
        let sources = vec![SourceConfig::Local {
            name: "local-bundled".to_string(),
            enabled: true,
            directory: "${KOPI_HOME}/bundled-metadata".to_string(),
            archive_pattern: "*.tar.gz".to_string(),
            cache_extracted: true,
        }];

        let metadata_config = MetadataConfig {
            cache: Default::default(),
            sources,
        };

        // Create provider - should expand ${KOPI_HOME}
        let result = MetadataProvider::from_metadata_config(&metadata_config, kopi_home);
        assert!(result.is_ok());
    }
}
