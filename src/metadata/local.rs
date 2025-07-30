//! Local directory metadata source implementation.
//!
//! Reads metadata from pre-extracted JSON files in a directory structure,
//! typically extracted from bundled metadata archives during installation.

use crate::error::{KopiError, Result};
use crate::metadata::{IndexFile, IndexFileEntry, MetadataSource, PackageDetails};
use crate::models::metadata::JdkMetadata;
use crate::platform::{get_current_architecture, get_current_os, get_foojay_libc_type};
use std::fs::File;
use std::path::PathBuf;

/// Get the platform directory name for the current system.
///
/// For Linux systems, includes the libc type (e.g., "linux-x64-glibc").
/// For other systems, omits the libc type (e.g., "windows-x64", "macos-aarch64").
fn get_current_platform_directory() -> String {
    let os = get_current_os();
    let arch = get_current_architecture();

    if os == "linux" {
        let libc = get_foojay_libc_type();
        format!("{os}-{arch}-{libc}")
    } else {
        format!("{os}-{arch}")
    }
}

/// Metadata source that reads from a local directory structure
pub struct LocalDirectorySource {
    directory: PathBuf,
}

impl LocalDirectorySource {
    /// Create a new LocalDirectorySource for the given directory
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// Read metadata from extracted directory structure
    fn read_metadata(&self) -> Result<Vec<JdkMetadata>> {
        // Read index.json
        let index_path = self.directory.join("index.json");
        let index_file = File::open(&index_path).map_err(|e| {
            KopiError::NotFound(format!(
                "Bundled metadata not found at {}: {}",
                index_path.display(),
                e
            ))
        })?;

        let index: IndexFile = serde_json::from_reader(index_file)?;

        // Get platform directory for current system
        let platform_dir = get_current_platform_directory();

        // Filter files for current platform
        let platform_files = self.filter_files_for_platform(index.files, &platform_dir);

        // Read metadata files from the platform directory
        let mut all_metadata = Vec::new();
        for file_info in platform_files {
            let file_path = self.directory.join(&file_info.path);

            if let Ok(file) = File::open(&file_path) {
                match serde_json::from_reader::<_, Vec<JdkMetadata>>(file) {
                    Ok(metadata) => {
                        // Local files should have full metadata
                        all_metadata.extend(metadata);
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse metadata file {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            } else {
                log::warn!("Metadata file not found: {}", file_path.display());
            }
        }

        Ok(all_metadata)
    }

    /// Filter metadata files based on current platform
    fn filter_files_for_platform(
        &self,
        files: Vec<IndexFileEntry>,
        platform_dir: &str,
    ) -> Vec<IndexFileEntry> {
        files
            .into_iter()
            .filter(|entry| {
                // Check if the file path starts with our platform directory
                entry.path.starts_with(&format!("{platform_dir}/"))
            })
            .collect()
    }
}

impl MetadataSource for LocalDirectorySource {
    fn id(&self) -> &str {
        "local"
    }

    fn name(&self) -> &str {
        "Local Directory"
    }

    fn is_available(&self) -> Result<bool> {
        // Check if the bundled metadata directory exists and has index.json
        let index_path = self.directory.join("index.json");
        Ok(index_path.exists())
    }

    fn fetch_all(&self) -> Result<Vec<JdkMetadata>> {
        self.read_metadata()
    }

    fn fetch_distribution(&self, distribution: &str) -> Result<Vec<JdkMetadata>> {
        let all_metadata = self.read_metadata()?;
        Ok(all_metadata
            .into_iter()
            .filter(|m| m.distribution == distribution)
            .collect())
    }

    fn fetch_package_details(&self, package_id: &str) -> Result<PackageDetails> {
        // Local directory source has complete metadata, so we can return details
        let all_metadata = self.read_metadata()?;

        // Find the package with matching ID
        let package = all_metadata
            .into_iter()
            .find(|m| m.id == package_id)
            .ok_or_else(|| KopiError::NotFound(format!("Package '{package_id}' not found")))?;

        // Extract package details
        let download_url = package.download_url.ok_or_else(|| {
            KopiError::NotFound(format!("Download URL not found for package '{package_id}'"))
        })?;

        Ok(PackageDetails {
            download_url,
            checksum: package.checksum,
            checksum_type: package.checksum_type,
        })
    }

    fn last_updated(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        // Try to get the bundle generation time from index.json
        let index_path = self.directory.join("index.json");
        if let Ok(file) = File::open(&index_path) {
            if let Ok(index) = serde_json::from_reader::<_, serde_json::Value>(file) {
                if let Some(updated) = index.get("updated").and_then(|v| v.as_str()) {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated) {
                        return Ok(Some(dt.with_timezone(&chrono::Utc)));
                    }
                }
            }
        }

        // Fallback to index.json modification time
        if let Ok(metadata) = std::fs::metadata(&index_path) {
            if let Ok(modified) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                return Ok(Some(datetime));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::package::{ArchiveType, PackageType};
    use crate::models::platform::{Architecture, OperatingSystem};
    use crate::version::Version;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_metadata() -> Vec<JdkMetadata> {
        vec![
            JdkMetadata {
                distribution: "temurin".to_string(),
                version: Version::new(21, 0, 0),
                distribution_version: Version::new(21, 0, 0),
                id: "temurin-21-linux-x64".to_string(),
                operating_system: OperatingSystem::Linux,
                architecture: Architecture::X64,
                lib_c_type: Some("glibc".to_string()),
                download_url: Some("https://example.com/temurin-21.tar.gz".to_string()),
                size: 100000000,
                checksum: Some("abc123".to_string()),
                checksum_type: Some(crate::models::package::ChecksumType::Sha256),
                package_type: PackageType::Jdk,
                archive_type: ArchiveType::TarGz,
                javafx_bundled: false,
                term_of_support: None,
                release_status: None,
                latest_build_available: None,
            },
            JdkMetadata {
                distribution: "corretto".to_string(),
                version: Version::new(21, 0, 0),
                distribution_version: Version::new(21, 0, 0),
                id: "corretto-21-linux-x64".to_string(),
                operating_system: OperatingSystem::Linux,
                architecture: Architecture::X64,
                lib_c_type: Some("glibc".to_string()),
                download_url: Some("https://example.com/corretto-21.tar.gz".to_string()),
                size: 110000000,
                checksum: Some("def456".to_string()),
                checksum_type: Some(crate::models::package::ChecksumType::Sha256),
                package_type: PackageType::Jdk,
                archive_type: ArchiveType::TarGz,
                javafx_bundled: false,
                term_of_support: None,
                release_status: None,
                latest_build_available: None,
            },
        ]
    }

    fn setup_test_directory(dir: &TempDir) -> PathBuf {
        let metadata_dir = dir.path().join("bundled-metadata");
        fs::create_dir_all(&metadata_dir).unwrap();

        // Create index.json
        let index = IndexFile {
            version: 2,
            updated: "2024-01-15T10:00:00Z".to_string(),
            files: vec![
                IndexFileEntry {
                    path: "linux-x64-glibc/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["linux".to_string()]),
                    lib_c_types: Some(vec!["glibc".to_string()]),
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                IndexFileEntry {
                    path: "linux-x64-glibc/corretto.json".to_string(),
                    distribution: "corretto".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["linux".to_string()]),
                    lib_c_types: Some(vec!["glibc".to_string()]),
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                // This one should be filtered out on non-Windows platforms
                IndexFileEntry {
                    path: "windows-x64/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["windows".to_string()]),
                    lib_c_types: None,
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
            ],
            generator_config: None,
        };

        let index_path = metadata_dir.join("index.json");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        // Create platform directories
        let linux_dir = metadata_dir.join("linux-x64-glibc");
        fs::create_dir_all(&linux_dir).unwrap();

        let windows_dir = metadata_dir.join("windows-x64");
        fs::create_dir_all(&windows_dir).unwrap();

        // Create metadata files
        let test_metadata = create_test_metadata();

        // Temurin Linux x64
        fs::write(
            linux_dir.join("temurin.json"),
            serde_json::to_string_pretty(&vec![test_metadata[0].clone()]).unwrap(),
        )
        .unwrap();

        // Corretto Linux x64
        fs::write(
            linux_dir.join("corretto.json"),
            serde_json::to_string_pretty(&vec![test_metadata[1].clone()]).unwrap(),
        )
        .unwrap();

        // Windows file (for testing filtering)
        fs::write(
            windows_dir.join("temurin.json"),
            serde_json::to_string_pretty(&vec![JdkMetadata {
                operating_system: OperatingSystem::Windows,
                ..test_metadata[0].clone()
            }])
            .unwrap(),
        )
        .unwrap();

        metadata_dir
    }

    #[test]
    fn test_local_directory_source_creation() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir.clone());
        assert_eq!(source.id(), "local");
        assert_eq!(source.name(), "Local Directory");
    }

    #[test]
    fn test_is_available() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir.clone());
        assert!(source.is_available().unwrap());

        // Test with non-existent directory
        let source = LocalDirectorySource::new(dir.path().join("non-existent"));
        assert!(!source.is_available().unwrap());
    }

    #[test]
    fn test_fetch_all() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir);
        let metadata = source.fetch_all().unwrap();

        // Should only get Linux metadata on Linux platforms
        #[cfg(target_os = "linux")]
        {
            assert_eq!(metadata.len(), 2);
            assert!(
                metadata
                    .iter()
                    .all(|m| m.operating_system == OperatingSystem::Linux)
            );
            assert!(metadata.iter().all(|m| m.is_complete()));
        }

        // On Windows, would get only Windows metadata
        #[cfg(target_os = "windows")]
        {
            assert_eq!(metadata.len(), 1);
            assert!(
                metadata
                    .iter()
                    .all(|m| m.operating_system == OperatingSystem::Windows)
            );
        }
    }

    #[test]
    fn test_fetch_distribution() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir);
        let metadata = source.fetch_distribution("temurin").unwrap();
        // Basic check that works on all platforms
        assert!(!metadata.is_empty());

        #[cfg(target_os = "linux")]
        {
            assert_eq!(metadata.len(), 1);
            assert_eq!(metadata[0].distribution, "temurin");
        }
    }

    #[test]
    fn test_last_updated() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir);
        let last_updated = source.last_updated().unwrap();

        assert!(last_updated.is_some());
        // Should parse the date from index.json
        let dt = last_updated.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_missing_metadata_file() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        // Delete one of the metadata files
        fs::remove_file(metadata_dir.join("linux-x64-glibc").join("corretto.json")).unwrap();

        let source = LocalDirectorySource::new(metadata_dir);
        let metadata = source.fetch_all().unwrap();
        // Basic check that works on all platforms
        assert!(!metadata.is_empty());

        // Should still get temurin metadata
        #[cfg(target_os = "linux")]
        {
            assert_eq!(metadata.len(), 1);
            assert_eq!(metadata[0].distribution, "temurin");
        }
    }

    #[test]
    fn test_corrupt_metadata_file() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        // Write corrupt JSON to one file
        fs::write(
            metadata_dir.join("linux-x64-glibc").join("corretto.json"),
            "{ invalid json",
        )
        .unwrap();

        let source = LocalDirectorySource::new(metadata_dir);
        let metadata = source.fetch_all().unwrap();
        // Basic check that works on all platforms
        assert!(!metadata.is_empty());

        // Should still get temurin metadata
        #[cfg(target_os = "linux")]
        {
            assert_eq!(metadata.len(), 1);
            assert_eq!(metadata[0].distribution, "temurin");
        }
    }

    #[test]
    fn test_fetch_package_details() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = setup_test_directory(&dir);

        let source = LocalDirectorySource::new(metadata_dir);

        // Test fetching existing package details
        let details = source
            .fetch_package_details("temurin-21-linux-x64")
            .unwrap();
        assert_eq!(
            details.download_url,
            "https://example.com/temurin-21.tar.gz"
        );
        assert_eq!(details.checksum, Some("abc123".to_string()));
        assert!(matches!(
            details.checksum_type,
            Some(crate::models::package::ChecksumType::Sha256)
        ));

        // Test fetching non-existent package
        let result = source.fetch_package_details("non-existent-package");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::NotFound(_)));
    }

    #[test]
    fn test_platform_filtering() {
        let dir = TempDir::new().unwrap();
        let metadata_dir = dir.path().join("bundled-metadata");
        fs::create_dir_all(&metadata_dir).unwrap();

        // Create index with multiple platform files
        let index = IndexFile {
            version: 2,
            updated: "2024-01-15T10:00:00Z".to_string(),
            files: vec![
                IndexFileEntry {
                    path: "linux-x64-glibc/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["linux".to_string()]),
                    lib_c_types: Some(vec!["glibc".to_string()]),
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                IndexFileEntry {
                    path: "linux-x64-musl/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["linux".to_string()]),
                    lib_c_types: Some(vec!["musl".to_string()]),
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                IndexFileEntry {
                    path: "linux-aarch64-glibc/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["aarch64".to_string()]),
                    operating_systems: Some(vec!["linux".to_string()]),
                    lib_c_types: Some(vec!["glibc".to_string()]),
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                IndexFileEntry {
                    path: "windows-x64/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["windows".to_string()]),
                    lib_c_types: None,
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
                IndexFileEntry {
                    path: "macos-x64/temurin.json".to_string(),
                    distribution: "temurin".to_string(),
                    architectures: Some(vec!["x64".to_string()]),
                    operating_systems: Some(vec!["macos".to_string()]),
                    lib_c_types: None,
                    size: 1024,
                    checksum: None,
                    last_modified: None,
                },
            ],
            generator_config: None,
        };

        fs::write(
            metadata_dir.join("index.json"),
            serde_json::to_string_pretty(&index).unwrap(),
        )
        .unwrap();

        let source = LocalDirectorySource::new(metadata_dir);

        // Get platform directory for current system
        let platform_dir = get_current_platform_directory();

        let filtered = source.filter_files_for_platform(index.files, &platform_dir);

        // Should only get files matching current platform directory
        for entry in &filtered {
            assert!(entry.path.starts_with(&format!("{platform_dir}/")));
        }

        // Verify we get at least some files on known platforms
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            assert!(!filtered.is_empty());
        }
    }
}
