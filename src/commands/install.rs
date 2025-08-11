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

use crate::archive::{JdkStructureType, detect_jdk_root, extract_archive};
use crate::cache::{self, MetadataCache};
use crate::config::KopiConfig;
use crate::download::download_jdk;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution;
use crate::models::metadata::JdkMetadata;
use crate::platform::{
    get_current_architecture, get_current_os, get_platform_description, matches_foojay_libc_type,
};
use crate::security::verify_checksum;
use crate::shim::discovery::{discover_distribution_tools, discover_jdk_tools};
use crate::shim::installer::ShimInstaller;
use crate::storage::JdkRepository;
use crate::version::parser::VersionParser;
use log::{debug, info, trace, warn};
use std::str::FromStr;
use std::time::Duration;

pub struct InstallCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> InstallCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Ensure we have a fresh cache, refreshing if necessary
    fn ensure_fresh_cache(&self, javafx_bundled: bool) -> Result<MetadataCache> {
        let cache_path = self.config.metadata_cache_path()?;
        let max_age = Duration::from_secs(self.config.cache.max_age_hours * 3600);

        // Check if cache needs refresh
        let should_refresh = if cache_path.exists() {
            match cache::load_cache(&cache_path) {
                Ok(cache) => {
                    if self.config.cache.auto_refresh {
                        cache.is_stale(max_age)
                    } else {
                        false
                    }
                }
                Err(e) => {
                    warn!("Failed to load existing cache: {e}");
                    true
                }
            }
        } else {
            debug!("No cache found, will fetch from API");
            true
        };

        // Refresh if needed
        if should_refresh && self.config.cache.auto_refresh {
            info!("Refreshing package cache...");
            match cache::fetch_and_cache_metadata(javafx_bundled, self.config) {
                Ok(cache) => Ok(cache),
                Err(e) => {
                    // If refresh fails and we have an existing cache, use it with warning
                    if cache_path.exists()
                        && let Ok(cache) = cache::load_cache(&cache_path)
                    {
                        warn!("Failed to refresh cache: {e}. Using existing cache.");
                        return Ok(cache);
                    }
                    Err(KopiError::MetadataFetch(format!(
                        "Failed to fetch metadata: {e}"
                    )))
                }
            }
        } else {
            cache::load_cache(&cache_path)
        }
    }

    pub fn execute(
        &self,
        version_spec: &str,
        force: bool,
        dry_run: bool,
        no_progress: bool,
        timeout_secs: Option<u64>,
        javafx_bundled: bool,
    ) -> Result<()> {
        info!("Installing JDK {version_spec}");
        debug!(
            "Install options: force={force}, dry_run={dry_run}, no_progress={no_progress}, \
             timeout={timeout_secs:?}, javafx_bundled={javafx_bundled}"
        );

        // Use config to parse version with additional distributions support
        let parser = VersionParser::new(self.config);

        // Parse version specification
        let version_request = parser.parse(version_spec)?;
        trace!("Parsed version request: {version_request:?}");

        // Install command requires a specific version
        let version = version_request.version.as_ref().ok_or_else(|| {
            KopiError::InvalidVersionFormat(
                "Install command requires a specific version. Use 'kopi cache search' to browse \
                 available versions."
                    .to_string(),
            )
        })?;

        // Validate version semantics
        VersionParser::validate_version_semantics(version)?;

        // Use default distribution from config if not specified
        let distribution = if let Some(dist) = version_request.distribution.clone() {
            dist
        } else {
            Distribution::from_str(&self.config.default_distribution)
                .unwrap_or(Distribution::Temurin)
        };

        println!("Installing {} {}...", distribution.name(), version);

        // Find matching JDK package first to get the actual distribution_version
        debug!("Searching for {} version {}", distribution.name(), version);
        let package =
            self.find_matching_package(&distribution, version, &version_request, javafx_bundled)?;
        trace!("Found package: {package:?}");
        let jdk_metadata = self.convert_package_to_metadata(package.clone())?;

        // Create storage manager with config
        let repository = JdkRepository::new(self.config);

        // Check if already installed using the actual distribution_version
        let installation_dir = repository.jdk_install_path(
            &distribution,
            &jdk_metadata.distribution_version.to_string(),
        )?;

        if dry_run {
            println!(
                "Would install {} {} to {}",
                distribution.name(),
                jdk_metadata.distribution_version,
                installation_dir.display()
            );
            return Ok(());
        }

        if installation_dir.exists() && !force {
            return Err(KopiError::AlreadyExists(format!(
                "{} {} is already installed. Use --force to reinstall.",
                distribution.name(),
                jdk_metadata.distribution_version
            )));
        }

        // Show the actual package found (for debugging purposes)
        if jdk_metadata.distribution.to_lowercase() != distribution.id() {
            warn!(
                "Requested {} but found {} package",
                distribution.name(),
                jdk_metadata.distribution
            );
        }
        // Fetch checksum before download
        let mut jdk_metadata_with_checksum = jdk_metadata.clone();
        if jdk_metadata_with_checksum.checksum.is_none() {
            debug!(
                "Fetching checksum for package ID: {}",
                jdk_metadata_with_checksum.id
            );
            match crate::cache::fetch_package_checksum(&jdk_metadata_with_checksum.id, self.config)
            {
                Ok((checksum, checksum_type)) => {
                    info!("Fetched checksum: {checksum} (type: {checksum_type:?})");
                    jdk_metadata_with_checksum.checksum = Some(checksum);
                    jdk_metadata_with_checksum.checksum_type = Some(checksum_type);
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch checksum: {e}. Proceeding without checksum verification."
                    );
                }
            }
        }

        println!(
            "Downloading {} {} (id: {})...",
            jdk_metadata_with_checksum.distribution,
            jdk_metadata_with_checksum.version,
            jdk_metadata_with_checksum.id
        );

        // Download JDK
        info!(
            "Downloading from {}",
            jdk_metadata_with_checksum
                .download_url
                .as_ref()
                .unwrap_or(&"<URL not available>".to_string())
        );
        let download_result = download_jdk(&jdk_metadata_with_checksum, no_progress, timeout_secs)?;
        let download_path = download_result.path();
        debug!("Downloaded to {download_path:?}");

        // Verify checksum
        if let Some(checksum) = &jdk_metadata_with_checksum.checksum
            && let Some(checksum_type) = jdk_metadata_with_checksum.checksum_type
        {
            println!("Verifying checksum...");
            verify_checksum(download_path, checksum, checksum_type)?;
        }

        // Prepare installation context
        let context = if force && installation_dir.exists() {
            // Remove existing installation first
            repository.remove_jdk(&installation_dir)?;
            repository.prepare_jdk_installation(
                &distribution,
                &jdk_metadata_with_checksum.distribution_version.to_string(),
            )?
        } else {
            repository.prepare_jdk_installation(
                &distribution,
                &jdk_metadata_with_checksum.distribution_version.to_string(),
            )?
        };

        // Extract archive to temp directory
        println!("Extracting archive...");
        info!("Extracting archive to {:?}", context.temp_path);
        extract_archive(download_path, &context.temp_path)?;
        debug!("Extraction completed");

        // Detect JDK structure and move the correct directory
        debug!("Detecting JDK structure");
        let structure_info = match detect_jdk_root(&context.temp_path) {
            Ok(info) => info,
            Err(e) => {
                // Clean up the failed installation
                let _ = repository.cleanup_failed_installation(&context);
                return Err(KopiError::ValidationError(format!(
                    "Invalid JDK structure in archive: {e}"
                )));
            }
        };

        info!(
            "Detected JDK structure: {:?} (root: {})",
            structure_info.structure_type,
            structure_info.jdk_root.display()
        );

        // Handle different structure types when moving to final location
        let final_path = self.finalize_with_structure(
            &repository,
            context,
            structure_info.jdk_root,
            structure_info.structure_type.clone(),
        )?;
        info!("JDK installed to {final_path:?}");

        // Create installation metadata based on detected structure
        let installation_metadata =
            self.create_installation_metadata(&structure_info.structure_type)?;

        // Save metadata JSON file with installation information
        repository.save_jdk_metadata_with_installation(
            &distribution,
            &jdk_metadata_with_checksum.distribution_version.to_string(),
            &package,
            &installation_metadata,
        )?;

        // Clean up is automatic when download_result goes out of scope
        // The TempDir will be cleaned up automatically

        println!(
            "Successfully installed {} {} to {}",
            distribution.name(),
            jdk_metadata_with_checksum.distribution_version,
            final_path.display()
        );

        // Create shims if enabled in config
        if self.config.shims.auto_create_shims {
            debug!("Auto-creating shims for newly installed JDK");

            // Discover JDK tools
            let mut tools = discover_jdk_tools(&final_path)?;
            debug!("Discovered {} standard JDK tools", tools.len());

            // Discover distribution-specific tools
            let extra_tools = discover_distribution_tools(&final_path, Some(distribution.id()))?;
            if !extra_tools.is_empty() {
                debug!(
                    "Discovered {} distribution-specific tools",
                    extra_tools.len()
                );
                tools.extend(extra_tools);
            }

            if !tools.is_empty() {
                println!("\nCreating shims...");
                let shim_installer = ShimInstaller::new(self.config.kopi_home());
                let created_shims = shim_installer.create_missing_shims(&tools)?;

                if !created_shims.is_empty() {
                    println!("Created {} new shims:", created_shims.len());
                    for shim in &created_shims {
                        println!("  - {shim}");
                    }
                } else {
                    debug!("All shims already exist");
                }
            }
        }

        // Show hint about using the JDK
        if VersionParser::is_lts_version(version.major()) {
            println!(
                "Note: {} is an LTS (Long Term Support) version.",
                version.major()
            );
        }
        println!("\nTo use this JDK, run: kopi use {version_spec}");

        Ok(())
    }

    fn find_matching_package(
        &self,
        distribution: &Distribution,
        version: &crate::version::Version,
        version_request: &crate::version::parser::ParsedVersionRequest,
        javafx_bundled: bool,
    ) -> Result<crate::models::api::Package> {
        // Build query parameters
        let arch = get_current_architecture();
        let os = get_current_os();

        // Always ensure we have a fresh cache
        let mut cache = self.ensure_fresh_cache(javafx_bundled)?;

        // Search in cache
        // First try exact match
        if let Some(mut jdk_metadata) = cache.lookup(
            distribution,
            &version.to_string(),
            &arch,
            &os,
            version_request.package_type.as_ref(),
            Some(javafx_bundled),
        ) {
            debug!(
                "Found exact package match: {} {}",
                distribution.name(),
                version
            );

            // Ensure metadata is complete before using it
            if !jdk_metadata.is_complete() {
                debug!("Metadata is incomplete, fetching package details...");
                let provider = crate::metadata::MetadataProvider::from_config(self.config)?;
                provider.ensure_complete(&mut jdk_metadata)?;
            }

            return Ok(self.convert_metadata_to_package(&jdk_metadata));
        }

        // If not found and refresh_on_miss is enabled, try refreshing cache once
        if self.config.cache.refresh_on_miss {
            info!("Package not found in cache, refreshing...");
            match cache::fetch_and_cache_metadata(javafx_bundled, self.config) {
                Ok(new_cache) => {
                    cache = new_cache;

                    // Search again in fresh cache
                    if let Some(mut jdk_metadata) = cache.lookup(
                        distribution,
                        &version.to_string(),
                        &arch,
                        &os,
                        version_request.package_type.as_ref(),
                        Some(javafx_bundled),
                    ) {
                        debug!(
                            "Found package after refresh: {} {}",
                            distribution.name(),
                            version
                        );

                        // Ensure metadata is complete before using it
                        if !jdk_metadata.is_complete() {
                            debug!("Metadata is incomplete, fetching package details...");
                            let provider =
                                crate::metadata::MetadataProvider::from_config(self.config)?;
                            provider.ensure_complete(&mut jdk_metadata)?;
                        }

                        return Ok(self.convert_metadata_to_package(&jdk_metadata));
                    }
                }
                Err(e) => {
                    warn!("Failed to refresh cache on miss: {e}");
                }
            }
        }

        // Package not found after all attempts
        // Try to find available versions in cache for helpful error message
        let available_versions = cache
            .distributions
            .get(distribution.id())
            .map(|dist| {
                let mut versions: Vec<String> = dist
                    .packages
                    .iter()
                    .filter(|pkg| {
                        pkg.architecture.to_string() == arch
                            && pkg.operating_system.to_string() == os
                    })
                    .map(|pkg| pkg.version.to_string())
                    .collect();
                versions.sort();
                versions.dedup();
                versions
            })
            .unwrap_or_default();

        Err(KopiError::VersionNotAvailable(format!(
            "{} {} not found. Available versions: {}",
            distribution.name(),
            version,
            if available_versions.is_empty() {
                "none for your platform".to_string()
            } else {
                available_versions.join(", ")
            }
        )))
    }

    fn convert_package_to_metadata(
        &self,
        package: crate::models::api::Package,
    ) -> Result<JdkMetadata> {
        let arch = get_current_architecture();
        let os = get_current_os();

        // Validate lib_c_type compatibility
        if let Some(ref lib_c_type) = package.lib_c_type
            && !matches_foojay_libc_type(lib_c_type)
        {
            return Err(KopiError::VersionNotAvailable(format!(
                "JDK lib_c_type '{}' is not compatible with kopi's platform '{}'",
                lib_c_type,
                get_platform_description()
            )));
        }

        Ok(JdkMetadata {
            id: package.id,
            distribution: package.distribution.clone(),
            version: crate::version::Version::from_str(&package.java_version)?,
            distribution_version: crate::version::Version::from_str(&package.distribution_version)
                .unwrap_or_else(|_| {
                    crate::version::Version::from_str(&package.java_version)
                        .unwrap_or(crate::version::Version::new(package.major_version, 0, 0))
                }),
            architecture: crate::models::platform::Architecture::from_str(&arch)?,
            operating_system: crate::models::platform::OperatingSystem::from_str(&os)?,
            package_type: crate::models::package::PackageType::from_str(&package.package_type)?,
            archive_type: crate::models::package::ArchiveType::from_str(&package.archive_type)?,
            download_url: Some(package.links.pkg_download_redirect),
            checksum: None, // Foojay API doesn't provide checksums directly
            checksum_type: None,
            size: package.size,
            lib_c_type: package.lib_c_type,
            javafx_bundled: package.javafx_bundled,
            term_of_support: package.term_of_support,
            release_status: package.release_status,
            latest_build_available: package.latest_build_available,
        })
    }
    fn finalize_with_structure(
        &self,
        repository: &JdkRepository,
        context: crate::storage::InstallationContext,
        jdk_root: std::path::PathBuf,
        structure_type: JdkStructureType,
    ) -> Result<std::path::PathBuf> {
        use std::fs;

        // Log the structure type for debugging
        match structure_type {
            JdkStructureType::Direct => {
                info!("Installing JDK with direct structure");
            }
            JdkStructureType::Bundle => {
                info!("Installing JDK with macOS bundle structure");
            }
            JdkStructureType::Hybrid => {
                info!("Installing JDK with hybrid structure (symlinks to bundle)");
            }
        }

        // If the JDK root is not the same as the temp path, we need to move it
        if jdk_root != context.temp_path {
            debug!(
                "JDK root ({}) differs from extraction path ({})",
                jdk_root.display(),
                context.temp_path.display()
            );

            // Move the JDK root directly to the final location
            if let Some(parent) = context.final_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Clean up any existing installation at the final path
            if context.final_path.exists() {
                fs::remove_dir_all(&context.final_path)?;
            }

            // Move the JDK root to the final location
            fs::rename(&jdk_root, &context.final_path).map_err(|e| {
                // Try to clean up on error
                let _ = repository.cleanup_failed_installation(&context);
                KopiError::Io(e)
            })?;

            // Clean up the temp directory if it still exists and is different from jdk_root
            if context.temp_path.exists() && context.temp_path != jdk_root {
                let _ = fs::remove_dir_all(&context.temp_path);
            }

            Ok(context.final_path)
        } else {
            // The JDK is directly in the temp path, use standard finalization
            repository.finalize_installation(context)
        }
    }

    fn create_installation_metadata(
        &self,
        structure_type: &JdkStructureType,
    ) -> Result<crate::storage::InstallationMetadata> {
        use crate::platform::{get_current_architecture, get_current_os};

        let java_home_suffix = match structure_type {
            JdkStructureType::Bundle => "Contents/Home".to_string(),
            JdkStructureType::Direct => String::new(),
            JdkStructureType::Hybrid => "Contents/Home".to_string(), // Hybrid also uses bundle path
        };

        let structure_type_str = match structure_type {
            JdkStructureType::Bundle => "bundle",
            JdkStructureType::Direct => "direct",
            JdkStructureType::Hybrid => "hybrid",
        };

        // Create platform string in format "os_arch"
        let arch = get_current_architecture();
        let os = get_current_os();
        let platform = format!("{os}_{arch}");

        Ok(crate::storage::InstallationMetadata {
            java_home_suffix,
            structure_type: structure_type_str.to_string(),
            platform,
            metadata_version: 1,
        })
    }

    fn convert_metadata_to_package(&self, metadata: &JdkMetadata) -> crate::models::api::Package {
        // Convert JdkMetadata to API Package format
        let pkg_info_uri = format!("https://api.foojay.io/disco/v3.0/packages/{}", metadata.id);

        crate::models::api::Package {
            id: metadata.id.clone(),
            archive_type: metadata.archive_type.to_string(),
            distribution: metadata.distribution.clone(),
            major_version: metadata.version.major(),
            java_version: metadata.version.to_string(),
            distribution_version: metadata.distribution_version.to_string(),
            jdk_version: metadata.version.major(),
            directly_downloadable: true,
            filename: format!(
                "{}-{}-{}-{}.{}",
                metadata.distribution,
                metadata.version,
                metadata.operating_system,
                metadata.architecture,
                metadata.archive_type.extension()
            ),
            links: crate::models::api::Links {
                pkg_download_redirect: metadata.download_url.clone().unwrap_or_default(),
                pkg_info_uri: Some(pkg_info_uri),
            },
            free_use_in_production: true,
            tck_tested: "unknown".to_string(),
            size: metadata.size,
            operating_system: metadata.operating_system.to_string(),
            architecture: Some(metadata.architecture.to_string()),
            lib_c_type: metadata.lib_c_type.clone(),
            package_type: metadata.package_type.to_string(),
            javafx_bundled: metadata.javafx_bundled,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::error::KopiError;

    #[test]
    fn test_parse_version_spec() {
        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config);
        assert!(cmd.is_ok());

        // Test version parsing is called correctly
        let parser = VersionParser::new(&config);
        let version_request = parser.parse("21").unwrap();
        assert!(version_request.version.is_some());
        assert_eq!(version_request.version.unwrap().major(), 21);
        assert_eq!(version_request.distribution, None);
    }

    #[test]
    fn test_dry_run_prevents_installation() {
        // This would be a more complex test with mocks
        // For now, just verify the command can be created
        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config);
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_parse_version_with_distribution() {
        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let parser = VersionParser::new(&config);
        let version_request = parser.parse("corretto@17").unwrap();
        assert!(version_request.version.is_some());
        assert_eq!(version_request.version.unwrap().major(), 17);
        assert_eq!(version_request.distribution, Some(Distribution::Corretto));
    }

    #[test]
    fn test_get_current_architecture() {
        let arch = get_current_architecture();
        // Should return a valid architecture string
        assert!(!arch.is_empty());
        assert_ne!(arch, "unknown");
    }

    #[test]
    fn test_get_current_os() {
        let os = get_current_os();
        // Should return a valid OS string
        assert!(!os.is_empty());
        assert_ne!(os, "unknown");
    }

    #[test]
    fn test_convert_metadata_to_package() {
        use crate::models::package::{ArchiveType, ChecksumType, PackageType};
        use crate::models::platform::{Architecture, OperatingSystem};
        use crate::version::Version;
        use std::str::FromStr;

        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();

        let metadata = JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: Version::from_str("21.0.1+12").unwrap(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: Some("https://example.com/download".to_string()),
            checksum: Some("abc123".to_string()),
            checksum_type: Some(ChecksumType::Sha256),
            size: 100000000,
            lib_c_type: None,
            javafx_bundled: false,
            term_of_support: None,
            release_status: None,
            latest_build_available: None,
        };

        let package = cmd.convert_metadata_to_package(&metadata);

        assert_eq!(package.id, "test-id");
        assert_eq!(package.distribution, "temurin");
        assert_eq!(package.major_version, 21);
        assert_eq!(package.java_version, "21.0.1");
        assert_eq!(package.distribution_version, "21.0.1+12");
        assert_eq!(package.archive_type, "tar.gz");
        assert_eq!(package.operating_system, "linux");
        assert_eq!(package.size, 100000000);
        assert!(package.directly_downloadable);
    }

    #[test]
    fn test_invalid_version_format_error() {
        // Test that invalid version format produces appropriate error
        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let parser = VersionParser::new(&config);
        let result = parser.parse("@@@invalid");
        assert!(result.is_err());
        match result {
            Err(KopiError::InvalidVersionFormat(_)) => {}
            _ => panic!("Expected InvalidVersionFormat error"),
        }
    }

    #[test]
    fn test_version_not_available_error() {
        // Mock scenario where version is not found
        let error = KopiError::VersionNotAvailable("temurin 999".to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("not available"));
    }

    #[test]
    fn test_already_exists_error() {
        let error = KopiError::AlreadyExists("temurin 21 is already installed".to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("already installed"));
    }

    #[test]
    fn test_network_error_handling() {
        let error = KopiError::NetworkError("Connection timeout".to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("Network error"));
    }

    #[test]
    fn test_permission_denied_error() {
        let error = KopiError::PermissionDenied("/opt/kopi".to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("Permission denied"));
    }

    #[test]
    fn test_disk_space_error() {
        let error = KopiError::DiskSpaceError("Only 100MB available, need 500MB".to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("disk space"));
    }

    #[test]
    fn test_checksum_mismatch_error() {
        let error = KopiError::ChecksumMismatch;
        let error_str = error.to_string();
        assert!(error_str.contains("Checksum verification failed"));
    }

    #[test]
    fn test_finalize_with_structure_direct() {
        use crate::archive::JdkStructureType;
        use crate::storage::InstallationContext;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();
        let repository = JdkRepository::new(&config);

        // Create a mock installation context
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let temp_path = jdks_dir.join(".tmp/test-install");
        fs::create_dir_all(&temp_path).unwrap();

        // Create a fake JDK structure
        let jdk_root = temp_path.join("jdk-21");
        fs::create_dir_all(jdk_root.join("bin")).unwrap();
        fs::write(jdk_root.join("bin/java"), "mock java").unwrap();

        let context = InstallationContext {
            final_path: jdks_dir.join("temurin-21.0.1"),
            temp_path: temp_path.clone(),
        };

        // Test direct structure finalization
        let result = cmd.finalize_with_structure(
            &repository,
            context,
            jdk_root.clone(),
            JdkStructureType::Direct,
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(final_path.exists());
        assert!(final_path.join("bin/java").exists());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_finalize_with_structure_bundle() {
        use crate::archive::JdkStructureType;
        use crate::storage::InstallationContext;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();
        let repository = JdkRepository::new(&config);

        // Create a mock installation context
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let temp_path = jdks_dir.join(".tmp/test-install");
        fs::create_dir_all(&temp_path).unwrap();

        // Create a fake bundle structure
        let bundle_root = temp_path.join("jdk-21.jdk");
        let contents_home = bundle_root.join("Contents/Home");
        fs::create_dir_all(contents_home.join("bin")).unwrap();
        fs::write(contents_home.join("bin/java"), "mock java").unwrap();

        let context = InstallationContext {
            final_path: jdks_dir.join("temurin-21.0.1"),
            temp_path: temp_path.clone(),
        };

        // Test bundle structure finalization
        // The bundle_root should be what gets moved, not Contents/Home
        let result = cmd.finalize_with_structure(
            &repository,
            context,
            bundle_root.clone(),
            JdkStructureType::Bundle,
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(final_path.exists());
        // After installation, the structure should be preserved
        assert!(final_path.join("Contents/Home/bin/java").exists());
    }

    #[test]
    fn test_finalize_with_structure_logging() {
        use crate::archive::JdkStructureType;
        use crate::storage::InstallationContext;
        use std::fs;
        use tempfile::TempDir;

        // This test verifies that structure types are logged correctly
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();

        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        let temp_path = jdks_dir.join(".tmp/test-install");
        let jdk_root = temp_path.join("jdk");
        fs::create_dir_all(jdk_root.join("bin")).unwrap();
        fs::write(jdk_root.join("bin/java"), "mock").unwrap();

        // Test that each structure type can be processed without errors
        for structure_type in [
            JdkStructureType::Direct,
            JdkStructureType::Bundle,
            JdkStructureType::Hybrid,
        ] {
            let ctx = InstallationContext {
                final_path: jdks_dir.join(format!("test-jdk-{structure_type:?}")),
                temp_path: temp_path.clone(),
            };

            // Re-create the JDK structure for each test
            if !jdk_root.exists() {
                fs::create_dir_all(jdk_root.join("bin")).unwrap();
                fs::write(jdk_root.join("bin/java"), "mock").unwrap();
            }

            let repo = JdkRepository::new(&config);
            let result =
                cmd.finalize_with_structure(&repo, ctx, jdk_root.clone(), structure_type.clone());

            // The function should handle all structure types
            assert!(
                result.is_ok(),
                "Failed for structure type: {structure_type:?}"
            );
        }
    }

    #[test]
    fn test_invalid_jdk_structure_error_handling() {
        use crate::archive::detect_jdk_root;
        use std::fs;
        use tempfile::TempDir;

        // Test that invalid JDK structures are properly rejected
        let temp_dir = TempDir::new().unwrap();
        let invalid_dir = temp_dir.path();

        // Create a directory without valid JDK structure
        fs::create_dir_all(invalid_dir.join("some_dir")).unwrap();
        fs::write(invalid_dir.join("some_file.txt"), "not a JDK").unwrap();

        let result = detect_jdk_root(invalid_dir);
        assert!(result.is_err());

        if let Err(KopiError::ValidationError(msg)) = result {
            assert!(msg.contains("No valid JDK structure found"));
        } else {
            panic!("Expected ValidationError for invalid JDK structure");
        }
    }

    #[test]
    fn test_create_installation_metadata_direct() {
        use crate::archive::JdkStructureType;

        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();

        let metadata = cmd
            .create_installation_metadata(&JdkStructureType::Direct)
            .unwrap();

        assert_eq!(metadata.java_home_suffix, "");
        assert_eq!(metadata.structure_type, "direct");
        assert!(!metadata.platform.is_empty());
        assert_eq!(metadata.metadata_version, 1);
    }

    #[test]
    fn test_create_installation_metadata_bundle() {
        use crate::archive::JdkStructureType;

        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();

        let metadata = cmd
            .create_installation_metadata(&JdkStructureType::Bundle)
            .unwrap();

        assert_eq!(metadata.java_home_suffix, "Contents/Home");
        assert_eq!(metadata.structure_type, "bundle");
        assert!(!metadata.platform.is_empty());
        assert_eq!(metadata.metadata_version, 1);
    }

    #[test]
    fn test_create_installation_metadata_hybrid() {
        use crate::archive::JdkStructureType;

        let config = KopiConfig::new(std::env::temp_dir()).unwrap();
        let cmd = InstallCommand::new(&config).unwrap();

        let metadata = cmd
            .create_installation_metadata(&JdkStructureType::Hybrid)
            .unwrap();

        assert_eq!(metadata.java_home_suffix, "Contents/Home");
        assert_eq!(metadata.structure_type, "hybrid");
        assert!(!metadata.platform.is_empty());
        assert_eq!(metadata.metadata_version, 1);
    }
}
