mod disk_space;
mod installation;
mod listing;
mod repository;

use crate::api::Package;
use crate::error::Result;
use crate::models::jdk::Distribution;
use std::fs;
use std::path::Path;

pub use installation::InstallationContext;
pub use listing::InstalledJdk;
pub use repository::JdkRepository;

pub fn save_jdk_metadata(
    jdks_dir: &Path,
    distribution: &Distribution,
    distribution_version: &str,
    metadata: &Package,
) -> Result<()> {
    let dir_name = format!("{}-{}", distribution.id(), distribution_version);
    let metadata_filename = format!("{}.meta.json", dir_name);
    let metadata_path = jdks_dir.join(metadata_filename);

    let json_content = serde_json::to_string_pretty(metadata)?;
    let json_content_with_newline = format!("{}\n", json_content);

    fs::write(&metadata_path, json_content_with_newline)?;

    log::debug!("Saved JDK metadata to {:?}", metadata_path);

    Ok(())
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use crate::api::Links;
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
        assert_eq!(
            parsed["links"]["pkg_download_redirect"],
            "https://example.com/download"
        );
    }
}
