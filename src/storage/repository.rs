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

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::api::Package;
use crate::models::distribution::Distribution;
use crate::paths::install;
use crate::storage::disk_space::DiskSpaceChecker;
use crate::storage::installation::{InstallationContext, JdkInstaller};
use crate::storage::listing::{InstalledJdk, JdkLister};
use crate::storage::{InstallationMetadata, JdkMetadataWithInstallation};
use crate::version::{Version, VersionRequest};
use log::{debug, warn};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct JdkRepository<'a> {
    config: &'a KopiConfig,
}

impl<'a> JdkRepository<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &KopiConfig {
        self.config
    }

    pub fn jdks_dir(&self) -> Result<PathBuf> {
        self.config.jdks_dir()
    }

    pub fn jdk_install_path(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
        javafx_bundled: bool,
    ) -> Result<PathBuf> {
        let suffix = if javafx_bundled { "-fx" } else { "" };
        let dir_name = format!("{}-{distribution_version}{suffix}", distribution.id());
        Ok(self.config.jdks_dir()?.join(dir_name))
    }

    pub fn prepare_jdk_installation(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
        javafx_bundled: bool,
    ) -> Result<InstallationContext> {
        let install_path =
            self.jdk_install_path(distribution, distribution_version, javafx_bundled)?;

        let disk_checker = DiskSpaceChecker::new(self.config.storage.min_disk_space_mb);
        disk_checker.check_disk_space(&install_path, self.config.kopi_home())?;

        let jdks_dir = self.config.jdks_dir()?;
        JdkInstaller::prepare_installation(&jdks_dir, &install_path)
    }

    pub fn finalize_installation(&self, context: InstallationContext) -> Result<PathBuf> {
        JdkInstaller::finalize_installation(context)
    }

    pub fn cleanup_failed_installation(&self, context: &InstallationContext) -> Result<()> {
        JdkInstaller::cleanup_failed_installation(context)
    }

    pub fn load_installed_metadata(
        &self,
        installed: &InstalledJdk,
    ) -> Result<InstalledMetadataSnapshot> {
        let jdks_dir = self.config.jdks_dir()?;
        if !installed.path.starts_with(&jdks_dir) {
            return Err(KopiError::SecurityError(format!(
                "Refusing to read metadata outside of the JDKs directory: {:?}",
                installed.path
            )));
        }

        let slug = installed
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                KopiError::ValidationError(format!(
                    "Invalid installation path for {} {}",
                    installed.distribution, installed.version
                ))
            })?;

        let metadata_path = install::metadata_file(self.config.kopi_home(), slug);
        if !metadata_path.exists() {
            return Ok(InstalledMetadataSnapshot::missing());
        }

        let contents = fs::read_to_string(&metadata_path).map_err(|error| {
            KopiError::SystemError(format!(
                "Failed to read metadata file {}: {error}",
                metadata_path.display()
            ))
        })?;

        match serde_json::from_str::<JdkMetadataWithInstallation>(&contents) {
            Ok(metadata) => {
                let installation_metadata = metadata.installation_metadata.clone();
                Ok(InstalledMetadataSnapshot::complete(
                    metadata,
                    installation_metadata,
                ))
            }
            Err(parse_error) => {
                warn!(
                    "Failed to parse installed metadata {}: {}",
                    metadata_path.display(),
                    parse_error
                );

                let installation_metadata = serde_json::from_str::<Value>(&contents)
                    .ok()
                    .and_then(|value| value.get("installation_metadata").cloned())
                    .and_then(|value| serde_json::from_value(value).ok());

                Ok(InstalledMetadataSnapshot {
                    metadata: None,
                    installation_metadata,
                })
            }
        }
    }

    pub fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>> {
        let jdks_dir = self.config.jdks_dir()?;
        JdkLister::list_installed_jdks(&jdks_dir)
    }

    /// Check if a specific JDK version is installed
    pub fn check_installation(
        &self,
        distribution: &Distribution,
        version: &Version,
    ) -> Result<bool> {
        debug!(
            "Checking installation for {} version {}",
            distribution.id(),
            version
        );

        // Get list of installed JDKs
        if let Ok(installed_jdks) = self.list_installed_jdks() {
            debug!("Found {} installed JDKs", installed_jdks.len());

            // Look for an exact match
            for jdk in installed_jdks {
                debug!(
                    "Checking installed JDK: {} {} at {:?}",
                    jdk.distribution, jdk.version, jdk.path
                );

                if jdk.distribution == distribution.id() {
                    debug!(
                        "Distribution matches. Checking if search version {} matches installed \
                         version {}",
                        version, jdk.version
                    );

                    // Check if the installed version matches the search pattern
                    // For example: installed "17.0.15" matches search pattern "17"
                    if jdk.version.matches_pattern(&version.to_string()) {
                        debug!(
                            "Found matching JDK: {} {} (matched pattern {})",
                            distribution.name(),
                            jdk.version,
                            version
                        );
                        return Ok(true);
                    } else {
                        debug!(
                            "Version mismatch: installed version {} does not match search pattern \
                             {}",
                            jdk.version, version
                        );
                    }
                }
            }
        }

        debug!(
            "No matching JDK found for {} version {}",
            distribution.id(),
            version
        );
        Ok(false)
    }

    pub fn get_jdk_size(&self, path: &Path) -> Result<u64> {
        JdkLister::get_jdk_size(path)
    }

    pub fn remove_jdk(&self, path: &Path) -> Result<()> {
        let jdks_dir = self.config.jdks_dir()?;
        if !path.starts_with(&jdks_dir) {
            return Err(KopiError::SecurityError(format!(
                "Refusing to remove directory outside of JDKs directory: {path:?}"
            )));
        }

        fs::remove_dir_all(path)?;
        Ok(())
    }

    pub fn save_jdk_metadata(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
        metadata: &Package,
    ) -> Result<()> {
        let jdks_dir = self.config.jdks_dir()?;
        super::save_jdk_metadata(&jdks_dir, distribution, distribution_version, metadata)
    }

    pub fn save_jdk_metadata_with_installation(
        &self,
        distribution: &Distribution,
        distribution_version: &str,
        metadata: &Package,
        installation_metadata: &InstallationMetadata,
        javafx_bundled: bool,
    ) -> Result<()> {
        let jdks_dir = self.config.jdks_dir()?;
        super::save_jdk_metadata_with_installation(
            &jdks_dir,
            distribution,
            distribution_version,
            metadata,
            installation_metadata,
            javafx_bundled,
        )
    }

    /// Find installed JDKs matching a version request and return them sorted by version (oldest first)
    ///
    /// # Arguments
    /// * `request` - Version request containing version pattern, optional distribution, and optional package type
    ///
    /// # Returns
    /// * Vec of InstalledJdk sorted by version (oldest first)
    ///
    /// # Examples
    /// * Version pattern "21" - Returns all 21.x.x.x versions, oldest first
    /// * Version pattern "21.2" - Returns all 21.2.x.x versions, oldest first  
    /// * Version pattern "21.2.13" - Returns all 21.2.13.x versions, oldest first
    /// * With distribution filter - Returns only JDKs from the specified distribution
    pub fn find_matching_jdks(&self, request: &VersionRequest) -> Result<Vec<InstalledJdk>> {
        // Get all installed JDKs
        let all_jdks = self.list_installed_jdks()?;

        // Filter JDKs based on distribution, version pattern, and JavaFX
        let mut matching_jdks: Vec<InstalledJdk> = all_jdks
            .into_iter()
            .filter(|jdk| {
                // Check distribution filter if specified
                if let Some(dist) = &request.distribution
                    && &jdk.distribution != dist
                {
                    return false;
                }

                // Check JavaFX filter if specified
                if let Some(javafx) = request.javafx_bundled
                    && jdk.javafx_bundled != javafx
                {
                    return false;
                }

                // Check version pattern
                jdk.version.matches_pattern(&request.version_pattern)
            })
            .collect();

        // Sort by version (oldest first)
        // When versions are equal, maintain stable sort order (which preserves distribution order from list_installed_jdks)
        matching_jdks.sort_by(|a, b| a.version.cmp(&b.version));

        Ok(matching_jdks)
    }
}

#[derive(Debug, Default)]
pub struct InstalledMetadataSnapshot {
    pub metadata: Option<JdkMetadataWithInstallation>,
    pub installation_metadata: Option<InstallationMetadata>,
}

impl InstalledMetadataSnapshot {
    pub fn missing() -> Self {
        Self::default()
    }

    pub fn complete(
        metadata: JdkMetadataWithInstallation,
        installation: InstallationMetadata,
    ) -> Self {
        Self {
            metadata: Some(metadata),
            installation_metadata: Some(installation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api::{Links, Package};
    use crate::paths::install;
    use std::str::FromStr;
    use tempfile::TempDir;

    struct TestStorage {
        config: KopiConfig,
        _temp_dir: TempDir,
    }

    impl TestStorage {
        fn new() -> Self {
            // Clear any leftover environment variables
            unsafe {
                std::env::remove_var("KOPI_AUTO_INSTALL");
                std::env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
            }

            let temp_dir = TempDir::new().unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
            TestStorage {
                config,
                _temp_dir: temp_dir,
            }
        }

        fn manager(&self) -> JdkRepository<'_> {
            JdkRepository::new(&self.config)
        }
    }

    #[test]
    fn test_jdk_install_path() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let distribution = Distribution::Temurin;

        let path = manager
            .jdk_install_path(&distribution, "21.0.1+35.1", false)
            .unwrap();
        assert!(path.ends_with("jdks/temurin-21.0.1+35.1"));
    }

    #[test]
    fn test_remove_jdk_security() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();

        let result = manager.remove_jdk(Path::new("/etc/passwd"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::SecurityError(_)));
    }

    #[test]
    fn test_min_disk_space_from_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[storage]
min_disk_space_mb = 1024
"#,
        )
        .unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(&config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 1024);
    }

    #[test]
    fn test_min_disk_space_default() {
        let temp_dir = TempDir::new().unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = JdkRepository::new(&config);
        assert_eq!(manager.config.storage.min_disk_space_mb, 500);
    }

    #[test]
    fn test_check_installation_empty_repository() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let distribution = Distribution::Temurin;
        let version = Version::new(21, 0, 0);

        let result = manager.check_installation(&distribution, &version).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_installation_with_partial_version() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        // Create a JDK directory with a full version
        fs::create_dir_all(jdks_dir.join("temurin-17.0.15")).unwrap();

        // Search with just major version "17"
        let distribution = Distribution::Temurin;
        let search_version = Version::from_components(17, None, None);

        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for version 17"
        );

        // Search with major.minor "17.0"
        let search_version = Version::from_components(17, Some(0), None);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for version 17.0"
        );

        // Search with full version "17.0.15"
        let search_version = Version::new(17, 0, 15);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            result,
            "Should find temurin-17.0.15 when searching for exact version"
        );

        // Search with different patch version should not match
        let search_version = Version::new(17, 0, 14);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            !result,
            "Should not find temurin-17.0.15 when searching for 17.0.14"
        );

        // Search with different minor version should not match
        let search_version = Version::new(17, 1, 0);
        let result = manager
            .check_installation(&distribution, &search_version)
            .unwrap();
        assert!(
            !result,
            "Should not find temurin-17.0.15 when searching for 17.1.0"
        );
    }

    #[test]
    fn test_find_matching_jdks() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        // Create test JDK directories as mentioned in the user's example
        fs::create_dir_all(jdks_dir.join("temurin-21.2.13.4")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-21.2.13.5")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-21.2.15.6")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-21.3.17.2")).unwrap();

        // Test case 1: Pattern "21" should return oldest 21.x version first
        let request = VersionRequest::new("21".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 4);
        // Check all elements in ascending order
        assert_eq!(matches[0].distribution, "temurin");
        assert_eq!(matches[0].version.to_string(), "21.2.13.4");
        assert_eq!(matches[1].distribution, "corretto");
        assert_eq!(matches[1].version.to_string(), "21.2.13.5");
        assert_eq!(matches[2].distribution, "temurin");
        assert_eq!(matches[2].version.to_string(), "21.2.15.6");
        assert_eq!(matches[3].distribution, "temurin");
        assert_eq!(matches[3].version.to_string(), "21.3.17.2");

        // Test case 2: Pattern "21.2" should return oldest 21.2.x version first
        let request = VersionRequest::new("21.2".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 3);
        // Check all elements in ascending order
        assert_eq!(matches[0].distribution, "temurin");
        assert_eq!(matches[0].version.to_string(), "21.2.13.4");
        assert_eq!(matches[1].distribution, "corretto");
        assert_eq!(matches[1].version.to_string(), "21.2.13.5");
        assert_eq!(matches[2].distribution, "temurin");
        assert_eq!(matches[2].version.to_string(), "21.2.15.6");

        // Test case 3: Pattern "21.2.13" should return oldest 21.2.13.x version first
        let request = VersionRequest::new("21.2.13".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].distribution, "temurin");
        assert_eq!(matches[0].version.to_string(), "21.2.13.4");
        assert_eq!(matches[1].distribution, "corretto");
        assert_eq!(matches[1].version.to_string(), "21.2.13.5");

        // Test case 4: Pattern "21.2.13.4" should return exact match
        let request = VersionRequest::new("21.2.13.4".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "temurin");
        assert_eq!(matches[0].version.to_string(), "21.2.13.4");

        // Test case 5: Pattern "corretto@21" should return only corretto JDKs
        let request = VersionRequest::new("21".to_string())
            .unwrap()
            .with_distribution("corretto".to_string());
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "corretto");
        assert_eq!(matches[0].version.to_string(), "21.2.13.5");

        // Test case 6: Pattern for non-existent version
        let request = VersionRequest::new("22".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 0);

        // Test case 7: Pattern with distribution that doesn't exist
        let request = VersionRequest::new("21".to_string())
            .unwrap()
            .with_distribution("zulu".to_string());
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_find_matching_jdks_sorting() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        // Create JDKs in random order to test sorting
        fs::create_dir_all(jdks_dir.join("temurin-17.0.5")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-21.0.0")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-17.0.15")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-21.0.7")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-21.0.7")).unwrap();

        // Test that versions are sorted oldest first
        let request = VersionRequest::new("21".to_string()).unwrap();
        let matches = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 3);

        // Check all elements in ascending order
        assert_eq!(matches[0].version.to_string(), "21.0.0");
        assert_eq!(matches[1].version.to_string(), "21.0.7");
        assert_eq!(matches[2].version.to_string(), "21.0.7");

        // When versions are equal, order is stable (depends on distribution sorting from list_installed_jdks)
        let request = VersionRequest::new("21.0.7".to_string()).unwrap();
        let matches_21_0_7 = manager.find_matching_jdks(&request).unwrap();
        assert_eq!(matches_21_0_7.len(), 2);
        // corretto comes before temurin alphabetically
        assert_eq!(matches_21_0_7[0].distribution, "corretto");
        assert_eq!(matches_21_0_7[1].distribution, "temurin");
    }

    #[test]
    fn test_find_matching_jdks_invalid_pattern() {
        // Invalid @ format - testing at VersionRequest level
        let result = VersionRequest::from_str("dist@ver@extra");
        assert!(result.is_err());

        // Invalid version pattern
        let result = VersionRequest::new("invalid.version".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn load_installed_metadata_returns_complete_snapshot() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        let slug = "temurin-21.0.2";
        let install_path = jdks_dir.join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.2").unwrap(),
            install_path,
            false,
        );

        let package = Package {
            id: "pkg-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.2".to_string(),
            distribution_version: "21.0.2".to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: "openjdk.tar.gz".to_string(),
            links: Links {
                pkg_download_redirect: "https://example.com".to_string(),
                pkg_info_uri: Some("https://example.com/info".to_string()),
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 1024,
            operating_system: "linux".to_string(),
            architecture: Some("x64".to_string()),
            lib_c_type: Some("gnu".to_string()),
            package_type: "JDK".to_string(),
            javafx_bundled: false,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        };

        let installation_metadata = InstallationMetadata {
            java_home_suffix: String::new(),
            structure_type: "direct".to_string(),
            platform: "linux_x64".to_string(),
            metadata_version: 1,
        };

        let complete_metadata = JdkMetadataWithInstallation {
            package: package.clone(),
            installation_metadata: installation_metadata.clone(),
        };

        let metadata_path = install::metadata_file(test_storage.config.kopi_home(), slug);
        fs::write(
            &metadata_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&complete_metadata).unwrap()
            ),
        )
        .unwrap();

        let snapshot = manager.load_installed_metadata(&installed).unwrap();
        assert!(snapshot.metadata.is_some());
        assert!(snapshot.installation_metadata.is_some());

        let parsed_package = &snapshot.metadata.as_ref().unwrap().package;
        assert_eq!(parsed_package.distribution, package.distribution);
        assert_eq!(
            parsed_package.distribution_version,
            package.distribution_version
        );

        let parsed_installation = snapshot.installation_metadata.unwrap();
        assert_eq!(parsed_installation.platform, installation_metadata.platform);
    }

    #[test]
    fn load_installed_metadata_handles_corruption() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        let slug = "temurin-21.0.3";
        let install_path = jdks_dir.join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.3").unwrap(),
            install_path,
            false,
        );

        let metadata_path = install::metadata_file(test_storage.config.kopi_home(), slug);
        fs::write(
            &metadata_path,
            r#"{
"installation_metadata": {
    "java_home_suffix": "",
    "structure_type": "direct",
    "platform": "linux_x64",
    "metadata_version": 1
},
"package": "invalid"
}
"#,
        )
        .unwrap();

        let snapshot = manager.load_installed_metadata(&installed).unwrap();
        assert!(snapshot.metadata.is_none());

        let installation = snapshot.installation_metadata.expect("fallback metadata");
        assert_eq!(installation.platform, "linux_x64");
    }

    #[test]
    fn test_save_jdk_metadata_with_installation() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let jdks_dir = config.jdks_dir().unwrap();
        fs::create_dir_all(&jdks_dir).unwrap();

        let distribution = Distribution::Temurin;
        let distribution_version = "21.0.1+35.1";

        let package = Package {
            id: "test-package-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.1".to_string(),
            distribution_version: distribution_version.to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: "OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.1_35.1.tar.gz".to_string(),
            links: Links {
                pkg_download_redirect: "https://example.com/download".to_string(),
                pkg_info_uri: Some("https://example.com/info".to_string()),
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 190000000,
            operating_system: "mac".to_string(),
            architecture: Some("aarch64".to_string()),
            lib_c_type: None,
            package_type: "jdk".to_string(),
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        let installation_metadata = crate::storage::InstallationMetadata {
            java_home_suffix: "Contents/Home".to_string(),
            structure_type: "bundle".to_string(),
            platform: "macos_aarch64".to_string(),
            metadata_version: 1,
        };

        // Save metadata with installation info
        let result = repository.save_jdk_metadata_with_installation(
            &distribution,
            distribution_version,
            &package,
            &installation_metadata,
            false,
        );
        assert!(result.is_ok());

        // Verify the metadata file exists
        let metadata_path = jdks_dir.join(format!(
            "{}-{distribution_version}.meta.json",
            distribution.id()
        ));
        assert!(metadata_path.exists());

        // Read and verify the contents
        let content = fs::read_to_string(&metadata_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Check API metadata fields
        assert_eq!(parsed["id"], "test-package-id");
        assert_eq!(parsed["distribution"], "temurin");
        assert_eq!(parsed["java_version"], "21.0.1");

        // Check installation metadata fields
        assert_eq!(
            parsed["installation_metadata"]["java_home_suffix"],
            "Contents/Home"
        );
        assert_eq!(parsed["installation_metadata"]["structure_type"], "bundle");
        assert_eq!(parsed["installation_metadata"]["platform"], "macos_aarch64");
        assert_eq!(parsed["installation_metadata"]["metadata_version"], 1);
    }

    #[test]
    fn test_javafx_directory_separation() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let jdks_dir = test_storage.config.jdks_dir().unwrap();

        // Create both JavaFX and non-JavaFX versions of the same JDK
        fs::create_dir_all(jdks_dir.join("liberica-21.0.5")).unwrap();
        fs::create_dir_all(jdks_dir.join("liberica-21.0.5-fx")).unwrap();

        // List installed JDKs
        let installed = manager.list_installed_jdks().unwrap();
        assert_eq!(installed.len(), 2);

        // Find the two JDKs and verify their properties
        let non_fx = installed.iter().find(|jdk| !jdk.javafx_bundled).unwrap();
        let with_fx = installed.iter().find(|jdk| jdk.javafx_bundled).unwrap();

        assert_eq!(non_fx.distribution, "liberica");
        assert_eq!(non_fx.version.to_string(), "21.0.5");
        assert!(!non_fx.javafx_bundled);
        assert!(non_fx.path.ends_with("liberica-21.0.5"));

        assert_eq!(with_fx.distribution, "liberica");
        assert_eq!(with_fx.version.to_string(), "21.0.5");
        assert!(with_fx.javafx_bundled);
        assert!(with_fx.path.ends_with("liberica-21.0.5-fx"));

        // Test version request filtering by JavaFX
        let request_no_fx = VersionRequest::new("21.0.5".to_string())
            .unwrap()
            .with_distribution("liberica".to_string())
            .with_javafx_bundled(false);
        let matches_no_fx = manager.find_matching_jdks(&request_no_fx).unwrap();
        assert_eq!(matches_no_fx.len(), 1);
        assert!(!matches_no_fx[0].javafx_bundled);

        let request_with_fx = VersionRequest::new("21.0.5".to_string())
            .unwrap()
            .with_distribution("liberica".to_string())
            .with_javafx_bundled(true);
        let matches_with_fx = manager.find_matching_jdks(&request_with_fx).unwrap();
        assert_eq!(matches_with_fx.len(), 1);
        assert!(matches_with_fx[0].javafx_bundled);

        // Test that without JavaFX filter, both are returned
        let request_all = VersionRequest::new("21.0.5".to_string())
            .unwrap()
            .with_distribution("liberica".to_string());
        let matches_all = manager.find_matching_jdks(&request_all).unwrap();
        assert_eq!(matches_all.len(), 2);
    }

    #[test]
    fn test_jdk_install_path_with_javafx() {
        let test_storage = TestStorage::new();
        let manager = test_storage.manager();
        let distribution = Distribution::Liberica;

        // Test non-JavaFX path
        let path_no_fx = manager
            .jdk_install_path(&distribution, "21.0.5", false)
            .unwrap();
        assert!(path_no_fx.ends_with("jdks/liberica-21.0.5"));

        // Test JavaFX path
        let path_with_fx = manager
            .jdk_install_path(&distribution, "21.0.5", true)
            .unwrap();
        assert!(path_with_fx.ends_with("jdks/liberica-21.0.5-fx"));
    }
}
