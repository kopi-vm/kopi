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

mod disk_space;
pub mod formatting;
mod installation;
mod listing;
mod repository;

use crate::error::Result;
use crate::models::api::Package;
use crate::models::distribution::Distribution;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub use installation::InstallationContext;
pub use listing::{InstalledJdk, JdkLister};
pub use repository::JdkRepository;

/// Installation metadata containing platform-specific JDK structure information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallationMetadata {
    /// The suffix to append to the installation directory to get JAVA_HOME
    /// For example: "Contents/Home" for macOS bundle structure, or "" for direct structure
    pub java_home_suffix: String,

    /// The type of JDK structure (bundle, direct, or hybrid)
    pub structure_type: String,

    /// Platform information (e.g., "macos_aarch64", "linux_x64", "windows_x64")
    pub platform: String,

    /// Metadata version for future compatibility
    #[serde(default = "default_metadata_version")]
    pub metadata_version: u32,
}

fn default_metadata_version() -> u32 {
    1
}

/// Complete JDK metadata including API data and installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdkMetadataWithInstallation {
    /// All fields from the API Package
    #[serde(flatten)]
    pub package: Package,

    /// Installation-specific metadata
    pub installation_metadata: InstallationMetadata,
}

pub fn save_jdk_metadata(
    jdks_dir: &Path,
    distribution: &Distribution,
    distribution_version: &str,
    metadata: &Package,
) -> Result<()> {
    let dir_name = format!("{}-{distribution_version}", distribution.id());
    let metadata_filename = format!("{dir_name}.meta.json");
    let metadata_path = jdks_dir.join(metadata_filename);

    let json_content = serde_json::to_string_pretty(metadata)?;
    let json_content_with_newline = format!("{json_content}\n");

    fs::write(&metadata_path, json_content_with_newline)?;

    log::debug!("Saved JDK metadata to {metadata_path:?}");

    Ok(())
}

/// Save JDK metadata with installation information
/// This function saves both the API metadata and platform-specific installation details
pub fn save_jdk_metadata_with_installation(
    jdks_dir: &Path,
    distribution: &Distribution,
    distribution_version: &str,
    metadata: &Package,
    installation_metadata: &InstallationMetadata,
) -> Result<()> {
    let dir_name = format!("{}-{distribution_version}", distribution.id());
    let metadata_filename = format!("{dir_name}.meta.json");
    let metadata_path = jdks_dir.join(metadata_filename);

    let complete_metadata = JdkMetadataWithInstallation {
        package: metadata.clone(),
        installation_metadata: installation_metadata.clone(),
    };

    let json_content = serde_json::to_string_pretty(&complete_metadata)?;
    let json_content_with_newline = format!("{json_content}\n");

    fs::write(&metadata_path, json_content_with_newline)?;

    log::debug!("Saved JDK metadata with installation info to {metadata_path:?}");

    Ok(())
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use crate::models::api::Links;
    use tempfile::TempDir;

    #[test]
    fn test_save_jdk_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let distribution = Distribution::Temurin;

        let package = Package {
            id: "test-package-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.1".to_string(),
            distribution_version: "21.0.1+35.1".to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_35.1.tar.gz".to_string(),
            links: Links {
                pkg_download_redirect: "https://example.com/download".to_string(),
                pkg_info_uri: Some("https://example.com/info".to_string()),
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 190000000,
            operating_system: "linux".to_string(),
            architecture: Some("x64".to_string()),
            lib_c_type: Some("glibc".to_string()),
            package_type: "jdk".to_string(),
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        let result = save_jdk_metadata(&jdks_dir, &distribution, "21.0.1+35.1", &package);
        assert!(result.is_ok());

        let metadata_path = jdks_dir.join("temurin-21.0.1+35.1.meta.json");
        assert!(metadata_path.exists());

        let content = fs::read_to_string(&metadata_path).unwrap();
        assert!(
            content.ends_with('\n'),
            "Metadata file should end with a newline"
        );

        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["id"], "test-package-id");
        assert_eq!(parsed["distribution"], "temurin");
        assert_eq!(parsed["java_version"], "21.0.1");
        assert_eq!(parsed["architecture"], "x64"); // Verify architecture is saved
        assert_eq!(
            parsed["links"]["pkg_download_redirect"],
            "https://example.com/download"
        );
    }

    #[test]
    fn test_installation_metadata_serialization() {
        let metadata = InstallationMetadata {
            java_home_suffix: "Contents/Home".to_string(),
            structure_type: "bundle".to_string(),
            platform: "macos_aarch64".to_string(),
            metadata_version: 1,
        };

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        let parsed: InstallationMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.java_home_suffix, "Contents/Home");
        assert_eq!(parsed.structure_type, "bundle");
        assert_eq!(parsed.platform, "macos_aarch64");
        assert_eq!(parsed.metadata_version, 1);
    }

    #[test]
    fn test_installation_metadata_backward_compatibility() {
        // JSON without metadata_version field (simulating old format)
        let json = r#"{
            "java_home_suffix": "",
            "structure_type": "direct",
            "platform": "linux_x64"
        }"#;

        let parsed: InstallationMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.metadata_version, 1); // Should use default value
        assert_eq!(parsed.java_home_suffix, "");
        assert_eq!(parsed.structure_type, "direct");
        assert_eq!(parsed.platform, "linux_x64");
    }

    #[test]
    fn test_installation_metadata_forward_compatibility() {
        // JSON with extra fields (simulating future format)
        let json = r#"{
            "java_home_suffix": "Contents/Home",
            "structure_type": "bundle",
            "platform": "macos_aarch64",
            "metadata_version": 2,
            "future_field": "some_value",
            "another_future_field": 42
        }"#;

        // Should successfully parse, ignoring unknown fields
        let parsed: InstallationMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.java_home_suffix, "Contents/Home");
        assert_eq!(parsed.structure_type, "bundle");
        assert_eq!(parsed.platform, "macos_aarch64");
        assert_eq!(parsed.metadata_version, 2);
    }

    #[test]
    fn test_save_jdk_metadata_with_installation() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let distribution = Distribution::Temurin;

        let package = Package {
            id: "test-package-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "temurin".to_string(),
            major_version: 21,
            java_version: "21.0.1".to_string(),
            distribution_version: "21.0.1+35.1".to_string(),
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

        let installation_metadata = InstallationMetadata {
            java_home_suffix: "Contents/Home".to_string(),
            structure_type: "bundle".to_string(),
            platform: "macos_aarch64".to_string(),
            metadata_version: 1,
        };

        let result = save_jdk_metadata_with_installation(
            &jdks_dir,
            &distribution,
            "21.0.1+35.1",
            &package,
            &installation_metadata,
        );
        assert!(result.is_ok());

        let metadata_path = jdks_dir.join("temurin-21.0.1+35.1.meta.json");
        assert!(metadata_path.exists());

        let content = fs::read_to_string(&metadata_path).unwrap();
        assert!(
            content.ends_with('\n'),
            "Metadata file should end with a newline"
        );

        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Check API metadata fields
        assert_eq!(parsed["id"], "test-package-id");
        assert_eq!(parsed["distribution"], "temurin");
        assert_eq!(parsed["java_version"], "21.0.1");
        assert_eq!(parsed["architecture"], "aarch64");

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
    fn test_jdk_metadata_with_installation_deserialization() {
        let json = r#"{
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

        let parsed: JdkMetadataWithInstallation = serde_json::from_str(json).unwrap();

        // Verify API fields
        assert_eq!(parsed.package.id, "test-id");
        assert_eq!(parsed.package.distribution, "temurin");

        // Verify installation metadata
        assert_eq!(
            parsed.installation_metadata.java_home_suffix,
            "Contents/Home"
        );
        assert_eq!(parsed.installation_metadata.structure_type, "bundle");
        assert_eq!(parsed.installation_metadata.platform, "macos_aarch64");
        assert_eq!(parsed.installation_metadata.metadata_version, 1);
    }

    #[test]
    fn test_invalid_json_handling() {
        // Test with invalid JSON
        let invalid_json = r#"{ invalid json"#;
        let result: std::result::Result<InstallationMetadata, serde_json::Error> =
            serde_json::from_str(invalid_json);
        assert!(result.is_err());

        // Test with missing required fields
        let missing_fields = r#"{
            "java_home_suffix": "Contents/Home"
        }"#;
        let result: std::result::Result<InstallationMetadata, serde_json::Error> =
            serde_json::from_str(missing_fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_version_default() {
        assert_eq!(default_metadata_version(), 1);
    }
}
