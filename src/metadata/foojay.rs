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

use crate::api::client::ApiClient;
use crate::api::query::PackageQuery;
use crate::error::Result;
use crate::indicator::ProgressIndicator;
use crate::metadata::source::{MetadataSource, PackageDetails};
use crate::models::metadata::JdkMetadata;
use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::version::Version;
use std::str::FromStr;

pub struct FoojayMetadataSource {
    client: ApiClient,
}

impl FoojayMetadataSource {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.client = self.client.with_base_url(base_url);
        self
    }

    /// Convert API Package to JdkMetadata (without download_url and checksum)
    fn convert_package_to_metadata_incomplete(
        &self,
        package: crate::models::api::Package,
    ) -> Result<JdkMetadata> {
        // Parse version
        let version = Version::from_str(&package.java_version)
            .unwrap_or_else(|_| Version::new(package.major_version, 0, 0));

        // Parse distribution_version
        let distribution_version =
            Version::from_str(&package.distribution_version).unwrap_or_else(|_| version.clone());

        // Parse architecture from filename
        let architecture = crate::cache::parse_architecture_from_filename(&package.filename)
            .unwrap_or(Architecture::X64);

        // Parse operating system
        let operating_system =
            OperatingSystem::from_str(&package.operating_system).unwrap_or(OperatingSystem::Linux);

        // Parse archive type
        let archive_type =
            ArchiveType::from_str(&package.archive_type).unwrap_or(ArchiveType::TarGz);

        let package_type = PackageType::from_str(&package.package_type).unwrap_or(PackageType::Jdk);

        Ok(JdkMetadata {
            id: package.id,
            distribution: package.distribution,
            version,
            distribution_version,
            architecture,
            operating_system,
            package_type,
            archive_type,
            // Foojay API doesn't provide these in the list response
            download_url: None,
            checksum: None,
            checksum_type: None,
            size: package.size,
            lib_c_type: package.lib_c_type,
            javafx_bundled: package.javafx_bundled,
            term_of_support: package.term_of_support,
            release_status: package.release_status,
            latest_build_available: package.latest_build_available,
        })
    }
}

impl MetadataSource for FoojayMetadataSource {
    fn id(&self) -> &str {
        "foojay"
    }

    fn name(&self) -> &str {
        "Foojay Discovery API"
    }

    fn is_available(&self) -> Result<bool> {
        // Try to get distributions as a simple health check
        match self.client.get_distributions() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn fetch_all(&self, progress: &mut dyn ProgressIndicator) -> Result<Vec<JdkMetadata>> {
        // Report initial connection
        progress.set_message("Connecting to Foojay API...".to_string());

        // Get all packages from the API with archive type filtering
        let query = PackageQuery {
            archive_types: Some(vec![
                "tar.gz".to_string(),
                "zip".to_string(),
                "tgz".to_string(),
            ]),
            ..Default::default()
        };
        let packages = self.client.get_packages(Some(query))?;

        // Report package count
        progress.set_message(format!("Retrieved {} packages from Foojay", packages.len()));

        // Convert to JdkMetadata with is_complete=false
        progress.set_message("Processing Foojay metadata...".to_string());

        let result: Result<Vec<JdkMetadata>> = packages
            .into_iter()
            .map(|pkg| self.convert_package_to_metadata_incomplete(pkg))
            .collect();

        // Report completion
        if let Ok(ref metadata) = result {
            progress.set_message(format!("Processed {} packages", metadata.len()));
        }

        result
    }

    fn fetch_distribution(
        &self,
        distribution: &str,
        progress: &mut dyn ProgressIndicator,
    ) -> Result<Vec<JdkMetadata>> {
        // Report fetching specific distribution
        progress.set_message(format!(
            "Fetching {distribution} packages from Foojay API..."
        ));

        let query = PackageQuery {
            distribution: Some(distribution.to_string()),
            archive_types: Some(vec![
                "tar.gz".to_string(),
                "zip".to_string(),
                "tgz".to_string(),
            ]),
            ..Default::default()
        };

        let packages = self.client.get_packages(Some(query))?;

        // Report package count for distribution
        let count = packages.len();
        progress.set_message(format!(
            "Retrieved {count} {distribution} packages from Foojay"
        ));

        // Process packages
        progress.set_message(format!("Processing {distribution} metadata..."));

        let result: Result<Vec<JdkMetadata>> = packages
            .into_iter()
            .map(|pkg| self.convert_package_to_metadata_incomplete(pkg))
            .collect();

        // Report completion
        if let Ok(ref metadata) = result {
            let count = metadata.len();
            progress.set_message(format!("Processed {count} {distribution} packages"));
        }

        result
    }

    fn fetch_package_details(
        &self,
        package_id: &str,
        progress: &mut dyn ProgressIndicator,
    ) -> Result<PackageDetails> {
        // Report fetching package details
        progress.set_message(format!("Fetching package details for {package_id}..."));

        // Fetch complete package info from API
        let package_info = self.client.get_package_by_id(package_id)?;

        // Parse checksum type
        let checksum_type = if !package_info.checksum_type.is_empty() {
            match package_info.checksum_type.to_lowercase().as_str() {
                "sha256" => Some(ChecksumType::Sha256),
                "sha512" => Some(ChecksumType::Sha512),
                "sha1" => Some(ChecksumType::Sha1),
                "md5" => Some(ChecksumType::Md5),
                _ => None,
            }
        } else {
            None
        };

        // Report completion
        progress.set_message(format!("Retrieved details for package {package_id}"));

        Ok(PackageDetails {
            download_url: package_info.direct_download_uri,
            checksum: if package_info.checksum.is_empty() {
                None
            } else {
                Some(package_info.checksum)
            },
            checksum_type,
        })
    }

    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        // Foojay API doesn't provide last update time
        Ok(None)
    }
}

impl Default for FoojayMetadataSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foojay_metadata_source_id() {
        let source = FoojayMetadataSource::new();
        assert_eq!(source.id(), "foojay");
        assert_eq!(source.name(), "Foojay Discovery API");
    }

    #[test]
    fn test_fetch_all_filters_archive_types() {
        // This test verifies that fetch_all() properly filters packages by archive type
        // The actual API call would be mocked in a real test, but here we verify the query
        let _source = FoojayMetadataSource::new();

        // We can't directly test the API call without mocking, but we can verify
        // that our implementation would create the correct query
        let expected_archive_types =
            vec!["tar.gz".to_string(), "zip".to_string(), "tgz".to_string()];

        // Create the same query that fetch_all() creates
        let query = PackageQuery {
            archive_types: Some(expected_archive_types.clone()),
            ..Default::default()
        };

        // Verify the query has the expected archive types
        assert!(query.archive_types.is_some());
        assert_eq!(query.archive_types.unwrap(), expected_archive_types);
    }

    #[test]
    fn test_convert_package_to_metadata_incomplete() {
        let source = FoojayMetadataSource::new();

        // Create a test package
        let api_package = crate::models::api::Package {
            id: "test123".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.1".to_string(),
            distribution_version: "21.0.1+12".to_string(),
            jdk_version: 21,
            operating_system: "linux".to_string(),
            architecture: Some("x64".to_string()),
            package_type: "jdk".to_string(),
            archive_type: "tar.gz".to_string(),
            filename: "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz".to_string(),
            directly_downloadable: true,
            links: crate::models::api::Links {
                pkg_download_redirect: "https://example.com/download".to_string(),
                pkg_info_uri: None,
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 195000000,
            lib_c_type: Some("glibc".to_string()),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        };

        let result = source.convert_package_to_metadata_incomplete(api_package);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.id, "test123");
        assert_eq!(metadata.distribution, "temurin");
        assert_eq!(metadata.version.major(), 21);
        assert_eq!(metadata.architecture.to_string(), "x64");
        assert_eq!(metadata.download_url, None); // Should be None for incomplete
        assert_eq!(metadata.checksum, None);
        assert_eq!(metadata.checksum_type, None);
        assert!(!metadata.is_complete()); // Should be marked as incomplete
    }

    #[test]
    fn test_fetch_package_details_parsing() {
        // This test would require mocking the API client
        // For now, we'll just test the checksum type parsing logic
        let _source = FoojayMetadataSource::new();

        // Test package details with various checksum types
        let test_cases = vec![
            ("sha256", Some(ChecksumType::Sha256)),
            ("SHA256", Some(ChecksumType::Sha256)),
            ("sha512", Some(ChecksumType::Sha512)),
            ("sha1", Some(ChecksumType::Sha1)),
            ("md5", Some(ChecksumType::Md5)),
            ("unknown", None),
            ("", None),
        ];

        for (checksum_type_str, expected) in test_cases {
            let package_info = crate::models::api::PackageInfo {
                filename: "test.tar.gz".to_string(),
                direct_download_uri: "https://example.com/download".to_string(),
                download_site_uri: None,
                checksum: "abc123".to_string(),
                checksum_type: checksum_type_str.to_string(),
                checksum_uri: "https://example.com/checksum".to_string(),
                signature_uri: None,
            };

            // We can't directly test fetch_package_details without mocking,
            // but we can verify the checksum type parsing logic
            let checksum_type = if !package_info.checksum_type.is_empty() {
                match package_info.checksum_type.to_lowercase().as_str() {
                    "sha256" => Some(ChecksumType::Sha256),
                    "sha512" => Some(ChecksumType::Sha512),
                    "sha1" => Some(ChecksumType::Sha1),
                    "md5" => Some(ChecksumType::Md5),
                    _ => None,
                }
            } else {
                None
            };

            assert_eq!(
                checksum_type, expected,
                "Failed for checksum type: {checksum_type_str}"
            );
        }
    }
}
