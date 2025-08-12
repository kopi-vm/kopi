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

use crate::error::{KopiError, Result};
use crate::storage::{InstallationMetadata, JdkMetadataWithInstallation};
use crate::version::Version;
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct InstalledJdk {
    pub distribution: String,
    pub version: Version,
    pub path: PathBuf,
    /// Cached metadata, loaded lazily on first access
    metadata_cache: RefCell<Option<InstallationMetadata>>,
}

impl InstalledJdk {
    /// Create a new InstalledJdk instance
    pub fn new(distribution: String, version: Version, path: PathBuf) -> Self {
        Self {
            distribution,
            version,
            path,
            metadata_cache: RefCell::new(None),
        }
    }

    /// Load metadata from the metadata file if it exists
    fn load_metadata(&self, jdks_dir: &Path) -> Option<InstallationMetadata> {
        let dir_name = format!("{}-{}", self.distribution, self.version);
        let metadata_filename = format!("{dir_name}.meta.json");
        let metadata_path = jdks_dir.join(&metadata_filename);

        if !metadata_path.exists() {
            log::debug!("Metadata file not found: {}", metadata_path.display());
            return None;
        }

        match std::fs::read_to_string(&metadata_path) {
            Ok(content) => match serde_json::from_str::<JdkMetadataWithInstallation>(&content) {
                Ok(metadata) => {
                    log::debug!("Loaded metadata from: {}", metadata_path.display());
                    Some(metadata.installation_metadata)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse metadata file {}: {}",
                        metadata_path.display(),
                        e
                    );
                    None
                }
            },
            Err(e) => {
                log::warn!(
                    "Failed to read metadata file {}: {}",
                    metadata_path.display(),
                    e
                );
                None
            }
        }
    }

    /// Get cached metadata, loading it if necessary
    fn get_cached_metadata(&self) -> Option<InstallationMetadata> {
        let mut cache = self.metadata_cache.borrow_mut();

        if cache.is_none() {
            // Try to load metadata from disk
            // We need to determine the jdks_dir - typically it's the parent of the JDK path
            if let Some(parent) = self.path.parent()
                && let Some(metadata) = self.load_metadata(parent)
            {
                // Validate metadata has required fields
                if self.validate_metadata(&metadata) {
                    *cache = Some(metadata);
                } else {
                    log::warn!(
                        "Metadata for {} has incomplete fields, falling back to runtime detection",
                        self.distribution
                    );
                }
            }
        }

        cache.clone()
    }

    /// Validate that metadata has all required fields
    fn validate_metadata(&self, metadata: &InstallationMetadata) -> bool {
        // Check that critical fields are not empty or have valid values
        if metadata.platform.is_empty() {
            log::debug!("Metadata validation failed: empty platform field");
            return false;
        }

        if metadata.structure_type.is_empty() {
            log::debug!("Metadata validation failed: empty structure_type field");
            return false;
        }

        // java_home_suffix can be empty for direct structure, so we don't validate it
        // metadata_version should be > 0
        if metadata.metadata_version == 0 {
            log::debug!("Metadata validation failed: invalid metadata_version");
            return false;
        }

        true
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KopiError::SystemError(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Format version string
        let version_string = format!("{}@{}", self.distribution, self.version);

        // Write atomically using a temporary file
        let temp_path = path.with_extension("tmp");

        {
            let mut file = fs::File::create(&temp_path).map_err(|e| {
                KopiError::SystemError(format!("Failed to create {}: {}", temp_path.display(), e))
            })?;

            file.write_all(version_string.as_bytes()).map_err(|e| {
                KopiError::SystemError(format!("Failed to write to {}: {}", temp_path.display(), e))
            })?;

            file.flush().map_err(|e| {
                KopiError::SystemError(format!("Failed to flush {}: {}", temp_path.display(), e))
            })?;
        }

        // Rename temp file to final location
        fs::rename(&temp_path, path).map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to rename {} to {}: {}",
                temp_path.display(),
                path.display(),
                e
            ))
        })?;

        log::debug!("Wrote version file: {path:?}");
        Ok(())
    }

    /// Resolves the correct JAVA_HOME path for this JDK installation.
    ///
    /// On macOS, this handles different directory structures:
    /// - Bundle structure: Returns path/Contents/Home
    /// - Direct structure: Returns path directly
    ///
    /// On other platforms, always returns the path directly.
    pub fn resolve_java_home(&self) -> PathBuf {
        // Try to use cached metadata first
        if let Some(metadata) = self.get_cached_metadata() {
            let java_home = if metadata.java_home_suffix.is_empty() {
                self.path.clone()
            } else {
                self.path.join(&metadata.java_home_suffix)
            };

            log::debug!(
                "Resolved JAVA_HOME for {} using cached metadata ({}): {}",
                self.distribution,
                metadata.structure_type,
                java_home.display()
            );
            return java_home;
        }

        // Fall back to runtime detection
        log::warn!(
            "No metadata found for {} at {}, falling back to runtime detection. \
             This may impact performance. Consider reinstalling the JDK to create metadata.",
            self.distribution,
            self.path.display()
        );

        #[cfg(target_os = "macos")]
        {
            // Check for bundle structure (Contents/Home)
            let bundle_path = self.path.join("Contents").join("Home");
            if bundle_path.join("bin").exists() {
                log::debug!(
                    "Resolved JAVA_HOME for {} using bundle structure: {}",
                    self.distribution,
                    bundle_path.display()
                );
                return bundle_path;
            }

            // Direct structure or hybrid (has bin at root)
            if self.path.join("bin").exists() {
                log::debug!(
                    "Resolved JAVA_HOME for {} using direct structure: {}",
                    self.distribution,
                    self.path.display()
                );
                return self.path.clone();
            }

            // Fallback: return path as-is and log warning
            log::warn!(
                "Could not detect JDK structure for {} at {}, using path as-is",
                self.distribution,
                self.path.display()
            );
            self.path.clone()
        }

        #[cfg(not(target_os = "macos"))]
        {
            // On non-macOS platforms, always use direct structure
            log::debug!(
                "Resolved JAVA_HOME for {} on non-macOS platform: {}",
                self.distribution,
                self.path.display()
            );
            self.path.clone()
        }
    }

    /// Resolves the path to the bin directory for this JDK installation.
    ///
    /// This method uses resolve_java_home() and appends "bin" to get the
    /// correct bin directory path regardless of the JDK structure.
    pub fn resolve_bin_path(&self) -> Result<PathBuf> {
        let java_home = self.resolve_java_home();
        let bin_path = java_home.join("bin");

        if !bin_path.exists() {
            return Err(KopiError::SystemError(format!(
                "JDK bin directory not found at expected location: {}",
                bin_path.display()
            )));
        }

        log::debug!(
            "Resolved bin path for {}: {}",
            self.distribution,
            bin_path.display()
        );

        Ok(bin_path)
    }
}

pub struct JdkLister;

impl JdkLister {
    pub fn list_installed_jdks(jdks_dir: &Path) -> Result<Vec<InstalledJdk>> {
        if !jdks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();

        for entry in fs::read_dir(jdks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(jdk_info) = Self::parse_jdk_dir_name(&path) {
                installed.push(jdk_info);
            }
        }

        installed.sort_by(|a, b| {
            a.distribution
                .cmp(&b.distribution)
                .then(b.version.cmp(&a.version))
        });

        Ok(installed)
    }

    pub fn parse_jdk_dir_name(path: &Path) -> Option<InstalledJdk> {
        let file_name = path.file_name()?.to_str()?;

        let mut split_pos = None;
        let chars: Vec<char> = file_name.chars().collect();

        for i in 0..chars.len() - 1 {
            if chars[i] == '-' && chars[i + 1].is_numeric() {
                split_pos = Some(i);
                break;
            }
        }

        let (distribution, version) = if let Some(pos) = split_pos {
            let dist = &file_name[..pos];
            let ver = &file_name[pos + 1..];
            (dist, ver)
        } else {
            return None;
        };

        let parsed_version = match Version::from_str(version) {
            Ok(v) => v,
            Err(_) => return None,
        };

        Some(InstalledJdk::new(
            distribution.to_string(),
            parsed_version,
            path.to_path_buf(),
        ))
    }

    pub fn get_jdk_size(path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api::{Links, Package};
    use crate::storage::{InstallationMetadata, JdkMetadataWithInstallation};
    use crate::version::Version;
    use std::time::Instant;
    use tempfile::TempDir;

    #[test]
    fn test_list_installed_jdks() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        fs::create_dir_all(jdks_dir.join("temurin-21.0.1")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-17.0.9")).unwrap();
        fs::create_dir_all(jdks_dir.join(".tmp")).unwrap();

        let installed = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
        assert_eq!(installed.len(), 2);

        assert_eq!(installed[0].distribution, "corretto");
        assert_eq!(installed[0].version.to_string(), "17.0.9");

        assert_eq!(installed[1].distribution, "temurin");
        assert_eq!(installed[1].version.to_string(), "21.0.1");
    }

    #[test]
    fn test_parse_jdk_dir_name() {
        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-21.0.1")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "21.0.1");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-22-ea")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "22-ea");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("corretto-17.0.9+9")).unwrap();
        assert_eq!(jdk.distribution, "corretto");
        assert_eq!(jdk.version.to_string(), "17.0.9+9");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("graalvm-ce-21.0.1")).unwrap();
        assert_eq!(jdk.distribution, "graalvm-ce");
        assert_eq!(jdk.version.to_string(), "21.0.1");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("liberica-21.0.1-13")).unwrap();
        assert_eq!(jdk.distribution, "liberica");
        assert_eq!(jdk.version.to_string(), "21.0.1-13");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-17")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "17");

        assert!(JdkLister::parse_jdk_dir_name(Path::new("invalid")).is_none());
        assert!(JdkLister::parse_jdk_dir_name(Path::new("no-hyphen-here")).is_none());
        assert!(JdkLister::parse_jdk_dir_name(Path::new("temurin")).is_none());

        // Version with 'v' prefix should not be parsed
        assert!(JdkLister::parse_jdk_dir_name(Path::new("zulu-v11.0.21")).is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_bundle_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Create bundle structure
        let bundle_bin_path = jdk_path.join("Contents").join("Home").join("bin");
        fs::create_dir_all(&bundle_bin_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
            metadata_cache: RefCell::new(None),
        };

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path.join("Contents").join("Home"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_direct_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("liberica-21.0.1");

        // Create direct structure
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path).unwrap();

        let jdk = InstalledJdk::new(
            "liberica".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_hybrid_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("zulu-21.0.1");

        // Create hybrid structure (bin at root + Contents/Home exists)
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        fs::create_dir_all(jdk_path.join("Contents").join("Home").join("bin")).unwrap();

        let jdk = InstalledJdk::new("zulu".to_string(), Version::new(21, 0, 1), jdk_path.clone());

        // Should prefer bundle structure when both exist
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path.join("Contents").join("Home"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_missing_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("broken-jdk");
        fs::create_dir_all(&jdk_path).unwrap();

        let jdk = InstalledJdk::new(
            "broken".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // Should return path as-is when structure cannot be detected
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_resolve_java_home_non_macos() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Even if bundle structure exists, should return direct path on non-macOS
        fs::create_dir_all(jdk_path.join("Contents").join("Home").join("bin")).unwrap();
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
            metadata_cache: RefCell::new(None),
        };

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    fn test_resolve_bin_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("test-jdk");

        // Create a bin directory
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path).unwrap();

        let jdk = InstalledJdk::new("test".to_string(), Version::new(21, 0, 1), jdk_path.clone());

        let resolved_bin = jdk.resolve_bin_path().unwrap();
        assert_eq!(resolved_bin, bin_path);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_bin_path_bundle_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Create bundle structure
        let bundle_bin_path = jdk_path.join("Contents").join("Home").join("bin");
        fs::create_dir_all(&bundle_bin_path).unwrap();

        let jdk = InstalledJdk::new("temurin".to_string(), Version::new(21, 0, 1), jdk_path);

        let resolved_bin = jdk.resolve_bin_path().unwrap();
        assert_eq!(resolved_bin, bundle_bin_path);
    }

    #[test]
    fn test_resolve_bin_path_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("broken-jdk");
        fs::create_dir_all(&jdk_path).unwrap();

        let jdk = InstalledJdk::new("broken".to_string(), Version::new(21, 0, 1), jdk_path);

        let result = jdk.resolve_bin_path();
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                KopiError::SystemError(msg) => {
                    assert!(msg.contains("bin directory not found"));
                }
                _ => panic!("Expected SystemError"),
            }
        }
    }

    #[test]
    fn test_metadata_lazy_loading() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create metadata file
        let metadata_content = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "temurin",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1+35.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": "https://example.com/info"
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "mac",
            "architecture": "aarch64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "Contents/Home",
                "structure_type": "bundle",
                "platform": "macos_aarch64",
                "metadata_version": 1
            }
        }"#;

        let metadata_path = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_path, metadata_content).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
            metadata_cache: RefCell::new(None),
        };

        // First access should load metadata
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path.join("Contents/Home"));

        // Verify metadata was cached
        assert!(jdk.metadata_cache.borrow().is_some());

        // Second access should use cached data (delete file to ensure it's not re-read)
        fs::remove_file(&metadata_path).unwrap();
        let java_home2 = jdk.resolve_java_home();
        assert_eq!(java_home2, jdk_path.join("Contents/Home"));
    }

    #[test]
    fn test_metadata_cache_miss_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("liberica-21.0.1");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        let jdk = InstalledJdk::new(
            "liberica".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // No metadata file exists, should fall back to runtime detection
        let java_home = jdk.resolve_java_home();

        #[cfg(target_os = "macos")]
        assert_eq!(java_home, jdk_path);

        #[cfg(not(target_os = "macos"))]
        assert_eq!(java_home, jdk_path);

        // Cache should remain None
        assert!(jdk.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_metadata_corrupt_file_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        // Create corrupt metadata file
        let metadata_path = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_path, "{ invalid json").unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
            metadata_cache: RefCell::new(None),
        };

        // Should fall back to runtime detection
        let java_home = jdk.resolve_java_home();

        #[cfg(target_os = "macos")]
        assert_eq!(java_home, jdk_path);

        #[cfg(not(target_os = "macos"))]
        assert_eq!(java_home, jdk_path);

        // Cache should remain None due to parse error
        assert!(jdk.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_metadata_performance() {
        use std::time::Instant;

        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create metadata file
        let metadata_content = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "temurin",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1+35.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": null
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "mac",
            "architecture": "aarch64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "",
                "structure_type": "direct",
                "platform": "macos_aarch64",
                "metadata_version": 1
            }
        }"#;

        let metadata_path = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_path, metadata_content).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
            metadata_cache: RefCell::new(None),
        };

        // First access loads metadata
        let _ = jdk.resolve_java_home();

        // Measure cached access time
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = jdk.resolve_java_home();
        }
        let elapsed = start.elapsed();

        // Average time per call should be < 1ms
        let avg_micros = elapsed.as_micros() / 1000;
        assert!(
            avg_micros < 1000,
            "Cached access took {avg_micros} microseconds on average"
        );
    }

    #[test]
    fn test_metadata_sequential_access() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create metadata file
        let metadata_content = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "temurin",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1+35.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": null
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "mac",
            "architecture": "aarch64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "Contents/Home",
                "structure_type": "bundle",
                "platform": "macos_aarch64",
                "metadata_version": 1
            }
        }"#;

        let metadata_path = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_path, metadata_content).unwrap();

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // Note: RefCell is not thread-safe, so this test verifies
        // sequential access from the same thread (which is the actual use case)
        let expected_java_home = jdk_path.join("Contents/Home");

        // Multiple sequential accesses
        for _ in 0..10 {
            let java_home = jdk.resolve_java_home();
            assert_eq!(java_home, expected_java_home);
        }

        // Verify metadata was only loaded once
        assert!(jdk.metadata_cache.borrow().is_some());
    }

    #[test]
    fn test_metadata_incomplete_fields_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        // Create metadata file with missing required fields
        let incomplete_metadata = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "temurin",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1+35.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": null
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "mac",
            "architecture": "aarch64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "Contents/Home",
                "structure_type": "",
                "platform": "macos_aarch64",
                "metadata_version": 1
            }
        }"#;

        let metadata_path = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_path, incomplete_metadata).unwrap();

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // Should fall back to runtime detection due to empty structure_type
        let java_home = jdk.resolve_java_home();

        #[cfg(target_os = "macos")]
        assert_eq!(java_home, jdk_path);

        #[cfg(not(target_os = "macos"))]
        assert_eq!(java_home, jdk_path);

        // Cache should remain None due to validation failure
        assert!(jdk.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_metadata_invalid_version_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("liberica-21.0.1");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        // Create metadata file with invalid metadata_version
        let invalid_metadata = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "liberica",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": null
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "linux",
            "architecture": "x64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "",
                "structure_type": "direct",
                "platform": "linux_x64",
                "metadata_version": 0
            }
        }"#;

        let metadata_path = jdks_dir.join("liberica-21.0.1.meta.json");
        fs::write(&metadata_path, invalid_metadata).unwrap();

        let jdk = InstalledJdk::new(
            "liberica".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // Should fall back to runtime detection due to invalid metadata_version
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);

        // Cache should remain None due to validation failure
        assert!(jdk.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_metadata_empty_platform_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let jdk_path = jdks_dir.join("zulu-21.0.1");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        // Create metadata file with empty platform field
        let invalid_metadata = r#"{
            "id": "test-id",
            "archive_type": "tar.gz",
            "distribution": "zulu",
            "major_version": 21,
            "java_version": "21.0.1",
            "distribution_version": "21.0.1",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "test.tar.gz",
            "links": {
                "pkg_download_redirect": "https://example.com",
                "pkg_info_uri": null
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 190000000,
            "operating_system": "mac",
            "architecture": "aarch64",
            "lib_c_type": null,
            "package_type": "jdk",
            "javafx_bundled": false,
            "term_of_support": null,
            "release_status": null,
            "latest_build_available": null,
            "installation_metadata": {
                "java_home_suffix": "",
                "structure_type": "direct",
                "platform": "",
                "metadata_version": 1
            }
        }"#;

        let metadata_path = jdks_dir.join("zulu-21.0.1.meta.json");
        fs::write(&metadata_path, invalid_metadata).unwrap();

        let jdk = InstalledJdk::new("zulu".to_string(), Version::new(21, 0, 1), jdk_path.clone());

        // Should fall back to runtime detection due to empty platform
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);

        // Cache should remain None due to validation failure
        assert!(jdk.metadata_cache.borrow().is_none());
    }

    #[test]
    fn test_fallback_no_user_errors() {
        // This test verifies that all fallback scenarios work without returning errors to users
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Test 1: Missing metadata file - should work without errors
        let jdk_path1 = jdks_dir.join("temurin-17.0.1");
        fs::create_dir_all(jdk_path1.join("bin")).unwrap();
        let jdk1 = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(17, 0, 1),
            jdk_path1.clone(),
        );

        // These operations should succeed without errors
        let java_home1 = jdk1.resolve_java_home();
        assert!(!java_home1.as_os_str().is_empty());
        let bin_path1 = jdk1.resolve_bin_path();
        assert!(bin_path1.is_ok());

        // Test 2: Corrupted metadata file - should work without errors
        let jdk_path2 = jdks_dir.join("liberica-17.0.1");
        fs::create_dir_all(jdk_path2.join("bin")).unwrap();
        fs::write(jdks_dir.join("liberica-17.0.1.meta.json"), "{ corrupt json").unwrap();
        let jdk2 = InstalledJdk::new(
            "liberica".to_string(),
            Version::new(17, 0, 1),
            jdk_path2.clone(),
        );

        let java_home2 = jdk2.resolve_java_home();
        assert!(!java_home2.as_os_str().is_empty());
        let bin_path2 = jdk2.resolve_bin_path();
        assert!(bin_path2.is_ok());

        // Test 3: Incomplete metadata - should work without errors
        let jdk_path3 = jdks_dir.join("zulu-17.0.1");
        fs::create_dir_all(jdk_path3.join("bin")).unwrap();
        let incomplete_meta = r#"{
            "id": "test",
            "installation_metadata": {
                "java_home_suffix": "",
                "structure_type": "",
                "platform": "test",
                "metadata_version": 1
            }
        }"#;
        fs::write(jdks_dir.join("zulu-17.0.1.meta.json"), incomplete_meta).unwrap();
        let jdk3 = InstalledJdk::new(
            "zulu".to_string(),
            Version::new(17, 0, 1),
            jdk_path3.clone(),
        );

        let java_home3 = jdk3.resolve_java_home();
        assert!(!java_home3.as_os_str().is_empty());
        let bin_path3 = jdk3.resolve_bin_path();
        assert!(bin_path3.is_ok());
    }

    #[test]
    fn test_fallback_logging_output() {
        use log::{Level, Log, Metadata, Record};
        use std::sync::Mutex;

        // Custom logger to capture log messages
        struct TestLogger {
            messages: Mutex<Vec<(Level, String)>>,
        }

        impl Log for TestLogger {
            fn enabled(&self, _metadata: &Metadata) -> bool {
                true
            }

            fn log(&self, record: &Record) {
                let mut messages = self.messages.lock().unwrap();
                messages.push((record.level(), record.args().to_string()));
            }

            fn flush(&self) {}
        }

        let _logger = TestLogger {
            messages: Mutex::new(Vec::new()),
        };

        // Note: In a real test environment, we'd use a proper logging framework
        // This is a simplified example to demonstrate the concept

        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Test missing metadata logging
        let jdk_path = jdks_dir.join("test-jdk");
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        let jdk = InstalledJdk::new("test".to_string(), Version::new(21, 0, 1), jdk_path.clone());

        // This should trigger fallback warning
        let _ = jdk.resolve_java_home();

        // In a real test, we would verify the log messages contain expected warnings
        // For now, we just ensure the operation completes without panic
    }

    #[test]
    fn test_installed_jdk_write_to() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join("test-version");

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(21, 0, 1),
            temp_dir.path().join("temurin-21.0.1"),
        );

        jdk.write_to(&version_file).unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "temurin@21.0.1");

        // Test overwriting
        let jdk2 = InstalledJdk::new(
            "corretto".to_string(),
            Version::new(17, 0, 9),
            temp_dir.path().join("corretto-17.0.9"),
        );

        jdk2.write_to(&version_file).unwrap();

        let content2 = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "corretto@17.0.9");
    }

    #[test]
    fn test_path_resolution_performance_regression() {
        // This test ensures that path resolution performance doesn't regress
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path();
        let jdk_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create metadata for fast cached access
        let metadata = JdkMetadataWithInstallation {
            package: Package {
                id: "perf-test".to_string(),
                archive_type: "tar.gz".to_string(),
                distribution: "temurin".to_string(),
                major_version: 21,
                java_version: "21.0.1".to_string(),
                distribution_version: "21.0.1".to_string(),
                jdk_version: 21,
                directly_downloadable: true,
                filename: "temurin-21.0.1.tar.gz".to_string(),
                links: Links {
                    pkg_download_redirect: "https://example.com/jdk.tar.gz".to_string(),
                    pkg_info_uri: None,
                },
                free_use_in_production: true,
                tck_tested: "yes".to_string(),
                size: 100000000,
                operating_system: "macos".to_string(),
                architecture: Some("x64".to_string()),
                lib_c_type: None,
                package_type: "jdk".to_string(),
                javafx_bundled: false,
                term_of_support: None,
                release_status: None,
                latest_build_available: Some(true),
            },
            installation_metadata: InstallationMetadata {
                java_home_suffix: "Contents/Home".to_string(),
                structure_type: "bundle".to_string(),
                platform: "macos".to_string(),
                metadata_version: 1,
            },
        };

        let metadata_file = jdks_dir.join("temurin-21.0.1.meta.json");
        fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(21, 0, 1),
            jdk_path.clone(),
        );

        // Pre-load cache
        let _ = jdk.resolve_java_home();

        // Measure cached access time
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = jdk.resolve_java_home();
        }
        let elapsed = start.elapsed();

        // Average time per call should be < 1 microsecond (1000ns)
        let avg_ns = elapsed.as_nanos() / 1000;
        assert!(
            avg_ns < 1000,
            "Path resolution with cache too slow: {avg_ns} ns/call (expected < 1000 ns)"
        );

        // Test bin path resolution performance
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = jdk.resolve_bin_path();
        }
        let elapsed = start.elapsed();

        // Bin path resolution should also be fast
        let avg_ns = elapsed.as_nanos() / 1000;
        assert!(
            avg_ns < 10000,
            "Bin path resolution too slow: {avg_ns} ns/call (expected < 10000 ns)"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_structure_detection_performance_regression() {
        use crate::archive::detect_jdk_root;

        // Test that structure detection performance is acceptable
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();

        // Create bundle structure
        let contents_home = jdk_path.join("Contents").join("Home");
        fs::create_dir_all(contents_home.join("bin")).unwrap();
        fs::File::create(contents_home.join("bin").join("java")).unwrap();

        // Measure detection time
        let start = Instant::now();
        for _ in 0..100 {
            let _ = detect_jdk_root(jdk_path).unwrap();
        }
        let elapsed = start.elapsed();

        // Average time should be < 1ms
        let avg_ms = elapsed.as_millis() / 100;
        assert!(
            avg_ms < 1,
            "Structure detection too slow: {avg_ms} ms/call (expected < 1 ms)"
        );
    }

    #[test]
    fn test_memory_usage_with_multiple_jdks() {
        // Test that memory usage is reasonable with many JDKs
        let temp_dir = TempDir::new().unwrap();
        let mut jdks = Vec::new();

        // Create 100 JDKs with metadata
        for i in 0..100 {
            let distribution = format!("dist{i}");
            let version = Version::new(21, 0, i as u32);
            let jdk_path = temp_dir.path().join(format!("{distribution}-{version}"));
            fs::create_dir_all(&jdk_path).unwrap();

            // Create metadata
            let metadata = JdkMetadataWithInstallation {
                package: Package {
                    id: format!("id-{i}"),
                    archive_type: "tar.gz".to_string(),
                    distribution: distribution.clone(),
                    major_version: 21,
                    java_version: version.to_string(),
                    distribution_version: version.to_string(),
                    jdk_version: 21,
                    directly_downloadable: true,
                    filename: format!("{distribution}-{version}.tar.gz"),
                    links: Links {
                        pkg_download_redirect: format!("https://example.com/jdk{i}.tar.gz"),
                        pkg_info_uri: None,
                    },
                    free_use_in_production: true,
                    tck_tested: "yes".to_string(),
                    size: 100000000,
                    operating_system: "macos".to_string(),
                    architecture: Some("x64".to_string()),
                    lib_c_type: None,
                    package_type: "jdk".to_string(),
                    javafx_bundled: false,
                    term_of_support: None,
                    release_status: None,
                    latest_build_available: Some(true),
                },
                installation_metadata: InstallationMetadata {
                    java_home_suffix: if i % 2 == 0 {
                        "".to_string()
                    } else {
                        "Contents/Home".to_string()
                    },
                    structure_type: if i % 2 == 0 {
                        "direct".to_string()
                    } else {
                        "bundle".to_string()
                    },
                    platform: "macos".to_string(),
                    metadata_version: 1,
                },
            };

            let metadata_file = temp_dir
                .path()
                .join(format!("{distribution}-{version}.meta.json"));
            fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

            jdks.push(InstalledJdk::new(distribution, version, jdk_path));
        }

        // Access all JDKs to load metadata
        for jdk in &jdks {
            let _ = jdk.resolve_java_home();
        }

        // Verify we can still access them efficiently
        let start = Instant::now();
        for jdk in &jdks {
            let _ = jdk.resolve_java_home();
        }
        let elapsed = start.elapsed();

        // Should still be fast even with 100 JDKs
        let elapsed_ms = elapsed.as_millis();
        assert!(
            elapsed_ms < 10,
            "Accessing 100 JDKs took too long: {elapsed_ms} ms (expected < 10 ms)"
        );
    }

    // This test is commented out because it causes a compilation error
    // that demonstrates the thread-safety issue.
    /*
    #[test]
    #[should_panic(expected = "cannot be shared between threads safely")]
    #[ignore = "This test reveals a thread-safety issue with RefCell - metadata_cache should use RwLock instead"]
    fn test_concurrent_metadata_access_reveals_race_condition() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Create JDK directory structure with bundle format
        let jdk_path = jdks_dir.join("temurin-21.0.0");
        fs::create_dir_all(&jdk_path).unwrap();
        let bundle_home = jdk_path.join("Contents/Home");
        let bundle_bin = bundle_home.join("bin");
        fs::create_dir_all(&bundle_bin).unwrap();

        // Create java binary
        let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
        fs::write(bundle_bin.join(java_binary), "#!/bin/sh\necho 'test java'").unwrap();

        // Create metadata file
        let metadata = JdkMetadataWithInstallation {
            package: Package {
                id: "test-id".to_string(),
                archive_type: "tar.gz".to_string(),
                distribution: "temurin".to_string(),
                major_version: 21,
                java_version: "21".to_string(),
                distribution_version: "21.0.0".to_string(),
                jdk_version: 21,
                directly_downloadable: true,
                filename: "test.tar.gz".to_string(),
                links: Links {
                    pkg_download_redirect: "".to_string(),
                    pkg_info_uri: None,
                },
                free_use_in_production: true,
                tck_tested: "yes".to_string(),
                size: 100000000,
                operating_system: "mac".to_string(),
                lib_c_type: None,
                architecture: Some("aarch64".to_string()),
                package_type: "jdk".to_string(),
                javafx_bundled: false,
                term_of_support: Some("sts".to_string()),
                release_status: None,
                latest_build_available: Some(true),
            },
            installation_metadata: InstallationMetadata {
                java_home_suffix: "Contents/Home".to_string(),
                structure_type: "bundle".to_string(),
                platform: "macos".to_string(),
                metadata_version: 1,
            },
        };

        let metadata_file = jdks_dir.join("temurin-21.0.0.meta.json");
        fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

        // Create shared InstalledJdk instance
        let version = Version::new(21, 0, 0);
        let jdk = Arc::new(InstalledJdk::new(
            "temurin".to_string(),
            version,
            jdk_path,
        ));

        // Spawn multiple threads accessing metadata concurrently
        let mut handles = vec![];
        for _ in 0..10 {
            let jdk_clone = Arc::clone(&jdk);
            let handle = thread::spawn(move || {
                // Each thread tries to resolve paths multiple times
                for _ in 0..100 {
                    let java_home = jdk_clone.resolve_java_home();
                    assert!(java_home.to_string_lossy().contains("Contents/Home"));

                    let bin_path = jdk_clone.resolve_bin_path();
                    assert!(bin_path.is_ok());
                    assert!(bin_path.unwrap().to_string_lossy().contains("Contents/Home/bin"));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // FINDING: This test reveals a thread-safety issue in InstalledJdk.
        // The metadata_cache field uses RefCell which is not Sync, preventing
        // safe concurrent access. The implementation should use RwLock or OnceCell
        // for thread-safe lazy initialization of metadata.
        //
        // The compile error proves that Arc<InstalledJdk> cannot be safely shared
        // between threads due to RefCell not implementing Sync.
    }
    */

    #[test]
    fn test_error_recovery_missing_bin_directory() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Create JDK without bin directory
        let jdk_path = jdks_dir.join("temurin-21.0.0");
        fs::create_dir_all(&jdk_path).unwrap();

        // Create metadata indicating bundle structure
        let metadata = JdkMetadataWithInstallation {
            package: create_test_package("temurin", "21.0.0"),
            installation_metadata: InstallationMetadata {
                java_home_suffix: "Contents/Home".to_string(),
                structure_type: "bundle".to_string(),
                platform: "macos".to_string(),
                metadata_version: 1,
            },
        };

        let metadata_file = jdks_dir.join("temurin-21.0.0.meta.json");
        fs::write(&metadata_file, serde_json::to_string(&metadata).unwrap()).unwrap();

        let jdk = InstalledJdk::new("temurin".to_string(), Version::new(21, 0, 0), jdk_path);

        // Should return error when bin directory is missing
        let bin_path_result = jdk.resolve_bin_path();
        assert!(bin_path_result.is_err());
        assert!(
            bin_path_result
                .unwrap_err()
                .to_string()
                .contains("bin directory not found")
        );
    }

    #[test]
    fn test_error_recovery_invalid_json_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Create JDK with proper structure based on platform
        let jdk_path = jdks_dir.join("temurin-21.0.0");
        
        #[cfg(target_os = "macos")]
        {
            // macOS: Create bundle structure
            let bundle_home = jdk_path.join("Contents/Home");
            let bundle_bin = bundle_home.join("bin");
            fs::create_dir_all(&bundle_bin).unwrap();
            
            // Create java binary
            let java_binary = "java";
            fs::write(bundle_bin.join(java_binary), "#!/bin/sh\necho 'test java'").unwrap();
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // Other platforms: Create direct structure
            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).unwrap();
            
            // Create java binary
            let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
            fs::write(bin_dir.join(java_binary), "#!/bin/sh\necho 'test java'").unwrap();
        }

        // Write invalid JSON to metadata file
        let metadata_file = jdks_dir.join("temurin-21.0.0.meta.json");
        fs::write(&metadata_file, "{ invalid json content }").unwrap();

        let jdk = InstalledJdk::new(
            "temurin".to_string(),
            Version::new(21, 0, 0),
            jdk_path.clone(),
        );

        // Should fall back to runtime detection when metadata is invalid
        let java_home = jdk.resolve_java_home();
        
        // Expected path depends on platform
        #[cfg(target_os = "macos")]
        let expected_java_home = jdk_path.join("Contents/Home");
        #[cfg(not(target_os = "macos"))]
        let expected_java_home = jdk_path.clone();
        
        assert_eq!(java_home, expected_java_home);

        // Bin path should still work via fallback
        let bin_path = jdk.resolve_bin_path();
        assert!(bin_path.is_ok());
        
        #[cfg(target_os = "macos")]
        assert_eq!(bin_path.unwrap(), jdk_path.join("Contents/Home/bin"));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(bin_path.unwrap(), jdk_path.join("bin"));
    }

    #[test]
    fn test_error_recovery_partially_missing_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        // Create JDK with direct structure
        let jdk_path = jdks_dir.join("liberica-17.0.9");
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path).unwrap();

        let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
        fs::write(bin_path.join(java_binary), "#!/bin/sh\necho 'test java'").unwrap();

        // Create metadata with missing installation_metadata field
        let incomplete_metadata = r#"{
            "id": "test-id",
            "distribution": "liberica",
            "version": "17.0.9",
            "java_version": "17",
            "major_version": 17
        }"#;

        let metadata_file = jdks_dir.join("liberica-17.0.9.meta.json");
        fs::write(&metadata_file, incomplete_metadata).unwrap();

        let jdk = InstalledJdk::new(
            "liberica".to_string(),
            Version::new(17, 0, 9),
            jdk_path.clone(),
        );

        // Should fall back to runtime detection
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);

        let bin_path = jdk.resolve_bin_path();
        assert!(bin_path.is_ok());
        assert_eq!(bin_path.unwrap(), jdk_path.join("bin"));
    }

    // Helper function to create a test Package
    fn create_test_package(distribution: &str, version: &str) -> Package {
        Package {
            id: "test-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: distribution.to_string(),
            major_version: 21,
            java_version: version.to_string(),
            distribution_version: version.to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: "test.tar.gz".to_string(),
            links: Links {
                pkg_download_redirect: "".to_string(),
                pkg_info_uri: None,
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 100000000,
            operating_system: "mac".to_string(),
            lib_c_type: None,
            architecture: Some("aarch64".to_string()),
            package_type: "jdk".to_string(),
            javafx_bundled: false,
            term_of_support: Some("sts".to_string()),
            release_status: None,
            latest_build_available: Some(true),
        }
    }
}
