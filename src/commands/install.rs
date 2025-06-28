use crate::api::ApiClient;
use crate::archive::extract_archive;
use crate::config::KopiConfig;
use crate::download::download_jdk;
use crate::error::{KopiError, Result};
use crate::models::jdk::{Distribution, JdkMetadata};
use crate::platform::{get_foojay_libc_type, get_platform_description, matches_foojay_libc_type};
use crate::search::PackageSearcher;
use crate::security::verify_checksum;
use crate::storage::JdkRepository;
use crate::version::parser::VersionParser;
use log::{debug, info, trace, warn};
use std::str::FromStr;

pub struct InstallCommand {
    api_client: ApiClient,
    storage_manager: JdkRepository,
    _config: KopiConfig,
}

impl InstallCommand {
    pub fn new() -> Result<Self> {
        let storage_manager = JdkRepository::new()?;
        let config = KopiConfig::load(storage_manager.kopi_home())?;
        let api_client = ApiClient::new();

        Ok(Self {
            api_client,
            storage_manager,
            _config: config,
        })
    }

    fn get_current_architecture(&self) -> String {
        #[cfg(target_arch = "x86_64")]
        return "x64".to_string();

        #[cfg(target_arch = "x86")]
        return "x86".to_string();

        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();

        #[cfg(target_arch = "arm")]
        return "arm32".to_string();

        #[cfg(target_arch = "powerpc64")]
        return if cfg!(target_endian = "little") {
            "ppc64le".to_string()
        } else {
            "ppc64".to_string()
        };

        #[cfg(target_arch = "s390x")]
        return "s390x".to_string();

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "powerpc64",
            target_arch = "s390x"
        )))]
        return "unknown".to_string();
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
        info!("Installing JDK {}", version_spec);
        debug!(
            "Install options: force={}, dry_run={}, no_progress={}, timeout={:?}, javafx_bundled={}",
            force, dry_run, no_progress, timeout_secs, javafx_bundled
        );

        // Parse version specification
        let version_request = VersionParser::parse(version_spec)?;
        trace!("Parsed version request: {:?}", version_request);

        // Install command requires a specific version
        let version = version_request.version.as_ref().ok_or_else(|| {
            KopiError::InvalidVersionFormat(
                "Install command requires a specific version. Use 'kopi cache search' to browse available versions.".to_string()
            )
        })?;

        // Validate version semantics
        VersionParser::validate_version_semantics(version)?;

        // Use default distribution if not specified
        let distribution = version_request
            .distribution
            .clone()
            .unwrap_or(Distribution::Temurin);

        println!("Installing {} {}...", distribution.name(), version);

        // Find matching JDK package first to get the actual distribution_version
        debug!("Searching for {} version {}", distribution.name(), version);
        let package =
            self.find_matching_package(&distribution, version, &version_request, javafx_bundled)?;
        trace!("Found package: {:?}", package);
        let jdk_metadata = self.convert_package_to_metadata(package.clone())?;

        // Check if already installed using the actual distribution_version
        let installation_dir = self
            .storage_manager
            .jdk_install_path(&distribution, &jdk_metadata.distribution_version);

        if installation_dir.exists() && !force {
            return Err(KopiError::AlreadyExists(format!(
                "{} {} is already installed. Use --force to reinstall.",
                distribution.name(),
                jdk_metadata.distribution_version
            )));
        }

        if dry_run {
            println!(
                "Would install {} {} to {}",
                distribution.name(),
                jdk_metadata.distribution_version,
                installation_dir.display()
            );
            return Ok(());
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
            match crate::cache::fetch_package_checksum(&jdk_metadata_with_checksum.id) {
                Ok((checksum, checksum_type)) => {
                    info!("Fetched checksum: {} (type: {:?})", checksum, checksum_type);
                    jdk_metadata_with_checksum.checksum = Some(checksum);
                    jdk_metadata_with_checksum.checksum_type = Some(checksum_type);
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch checksum: {}. Proceeding without checksum verification.",
                        e
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

        // Check disk space (convert bytes to MB)
        let _required_space_mb = if jdk_metadata_with_checksum.size > 0 {
            jdk_metadata_with_checksum.size / 1024 / 1024
        } else {
            500 // Default to 500MB if size is unknown
        };

        // Download JDK
        info!(
            "Downloading from {}",
            jdk_metadata_with_checksum.download_url
        );
        let download_result = download_jdk(&jdk_metadata_with_checksum, no_progress, timeout_secs)?;
        let download_path = download_result.path();
        debug!("Downloaded to {:?}", download_path);

        // Verify checksum
        if let Some(checksum) = &jdk_metadata_with_checksum.checksum {
            println!("Verifying checksum...");
            verify_checksum(download_path, checksum)?;
        }

        // Prepare installation context
        let context = if force && installation_dir.exists() {
            // Remove existing installation first
            self.storage_manager.remove_jdk(&installation_dir)?;
            self.storage_manager.prepare_jdk_installation(
                &distribution,
                &jdk_metadata_with_checksum.distribution_version,
            )?
        } else {
            self.storage_manager.prepare_jdk_installation(
                &distribution,
                &jdk_metadata_with_checksum.distribution_version,
            )?
        };

        // Extract archive to temp directory
        println!("Extracting archive...");
        info!("Extracting archive to {:?}", context.temp_path);
        extract_archive(download_path, &context.temp_path)?;
        debug!("Extraction completed");

        // Finalize installation
        debug!("Finalizing installation");
        let final_path = self.storage_manager.finalize_installation(context)?;
        info!("JDK installed to {:?}", final_path);

        // Save metadata JSON file
        self.storage_manager.save_jdk_metadata(
            &distribution,
            &jdk_metadata_with_checksum.distribution_version,
            &package,
        )?;

        // Clean up is automatic when download_result goes out of scope
        // The TempDir will be cleaned up automatically

        println!(
            "Successfully installed {} {} to {}",
            distribution.name(),
            jdk_metadata_with_checksum.distribution_version,
            final_path.display()
        );

        // Show hint about using the JDK
        if VersionParser::is_lts_version(version.major) {
            println!(
                "Note: {} is an LTS (Long Term Support) version.",
                version.major
            );
        }
        println!("\nTo use this JDK, run: kopi use {}", version_spec);

        Ok(())
    }

    fn find_matching_package(
        &self,
        distribution: &Distribution,
        version: &crate::models::jdk::Version,
        version_request: &crate::version::parser::ParsedVersionRequest,
        javafx_bundled: bool,
    ) -> Result<crate::api::Package> {
        // Build query parameters
        let arch = self.get_current_architecture();
        let os = self.get_current_os();
        let lib_c_type = get_foojay_libc_type();

        // First try to find the package in cache if it exists
        if let Ok(cache) = crate::cache::load_cache_if_exists() {
            let searcher = PackageSearcher::new(Some(&cache));
            if let Some(jdk_metadata) =
                searcher.find_exact_package(distribution, &version.to_string(), &arch, &os)
            {
                // Convert cached JdkMetadata to API Package format
                debug!(
                    "Found package in cache: {} {}",
                    distribution.name(),
                    version
                );
                return Ok(self.convert_metadata_to_package(&jdk_metadata));
            }
        }

        // If not found in cache or cache doesn't exist, fetch directly from API
        debug!("Package not found in cache, fetching directly from API");

        // Archive types to query for (as expected by foojay.io API)
        let archive_types = vec![
            "tar.gz".to_string(),
            "zip".to_string(),
            "tgz".to_string(),
            "tar".to_string(),
        ];

        let package_type_str = version_request
            .package_type
            .as_ref()
            .map(|pt| pt.to_string())
            .unwrap_or_else(|| "jdk".to_string());

        let query = crate::api::PackageQuery {
            version: Some(version.to_string()),
            distribution: Some(distribution.id().to_string()),
            architecture: Some(arch.clone()),
            operating_system: Some(os.clone()),
            package_type: Some(package_type_str),
            archive_types: Some(archive_types),
            latest: Some("per_version".to_string()),
            directly_downloadable: Some(true),
            lib_c_type: Some(lib_c_type.to_string()),
            javafx_bundled: if javafx_bundled { None } else { Some(false) },
        };

        // Get packages from API
        let packages = self.api_client.get_packages(Some(query))?;

        // Debug: Log the response
        debug!("API returned {} packages", packages.len());
        for (i, pkg) in packages.iter().enumerate() {
            trace!(
                "Package[{}]: distribution={}, version={}, filename={}, archive_type={}, os={}, id={}",
                i,
                pkg.distribution,
                pkg.java_version,
                pkg.filename,
                pkg.archive_type,
                pkg.operating_system,
                pkg.id
            );
        }

        if packages.is_empty() {
            // Try to find any packages for this distribution to suggest versions
            let archive_types = vec![
                "tar.gz".to_string(),
                "zip".to_string(),
                "tgz".to_string(),
                "tar".to_string(),
            ];

            let package_type_str_all = version_request
                .package_type
                .as_ref()
                .map(|pt| pt.to_string())
                .unwrap_or_else(|| "jdk".to_string());

            let query_all = crate::api::PackageQuery {
                distribution: Some(distribution.id().to_string()),
                architecture: Some(arch.clone()),
                operating_system: Some(os.clone()),
                package_type: Some(package_type_str_all),
                archive_types: Some(archive_types),
                directly_downloadable: Some(true),
                lib_c_type: Some(lib_c_type.to_string()),
                version: None,
                latest: None,
                javafx_bundled: if javafx_bundled { None } else { Some(false) },
            };

            let all_packages = self
                .api_client
                .get_packages(Some(query_all))
                .unwrap_or_default();
            let version_strings: Vec<String> = all_packages
                .iter()
                .map(|p| p.java_version.clone())
                .collect();

            return Err(KopiError::VersionNotAvailable(format!(
                "{} {} not found. Available versions: {}",
                distribution.name(),
                version,
                if version_strings.is_empty() {
                    "none".to_string()
                } else {
                    version_strings.join(", ")
                }
            )));
        }

        // Find the first package that matches the requested distribution
        let package = packages
            .into_iter()
            .find(|p| p.distribution.to_lowercase() == distribution.id())
            .ok_or_else(|| {
                KopiError::VersionNotAvailable(format!(
                    "{} {} not found in the returned packages",
                    distribution.name(),
                    version
                ))
            })?;

        Ok(package)
    }

    fn get_current_os(&self) -> String {
        #[cfg(target_os = "linux")]
        return "linux".to_string();

        #[cfg(target_os = "windows")]
        return "windows".to_string();

        #[cfg(target_os = "macos")]
        return "macos".to_string();

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        return "unknown".to_string();
    }

    fn convert_package_to_metadata(&self, package: crate::api::Package) -> Result<JdkMetadata> {
        let arch = self.get_current_architecture();
        let os = self.get_current_os();

        // Validate lib_c_type compatibility
        if let Some(ref lib_c_type) = package.lib_c_type {
            if !matches_foojay_libc_type(lib_c_type) {
                return Err(KopiError::VersionNotAvailable(format!(
                    "JDK lib_c_type '{}' is not compatible with kopi's platform '{}'",
                    lib_c_type,
                    get_platform_description()
                )));
            }
        }

        Ok(JdkMetadata {
            id: package.id,
            distribution: package.distribution.clone(),
            version: crate::models::jdk::Version::from_str(&package.java_version)?,
            distribution_version: package.distribution_version,
            architecture: crate::models::jdk::Architecture::from_str(&arch)?,
            operating_system: crate::models::jdk::OperatingSystem::from_str(&os)?,
            package_type: crate::models::jdk::PackageType::Jdk,
            archive_type: crate::models::jdk::ArchiveType::from_str(&package.archive_type)?,
            download_url: package.links.pkg_download_redirect,
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
    fn convert_metadata_to_package(&self, metadata: &JdkMetadata) -> crate::api::Package {
        // Convert JdkMetadata to API Package format
        let pkg_info_uri = format!("https://api.foojay.io/disco/v3.0/packages/{}", metadata.id);

        crate::api::Package {
            id: metadata.id.clone(),
            archive_type: metadata.archive_type.to_string(),
            distribution: metadata.distribution.clone(),
            major_version: metadata.version.major,
            java_version: metadata.version.to_string(),
            distribution_version: metadata.distribution_version.clone(),
            jdk_version: metadata.version.major,
            directly_downloadable: true,
            filename: format!(
                "{}-{}-{}-{}.{}",
                metadata.distribution,
                metadata.version,
                metadata.operating_system,
                metadata.architecture,
                metadata.archive_type.extension()
            ),
            links: crate::api::Links {
                pkg_download_redirect: metadata.download_url.clone(),
                pkg_info_uri: Some(pkg_info_uri),
            },
            free_use_in_production: true,
            tck_tested: "unknown".to_string(),
            size: metadata.size,
            operating_system: metadata.operating_system.to_string(),
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

    #[test]
    fn test_parse_version_spec() {
        let cmd = InstallCommand::new();
        assert!(cmd.is_ok());

        // Test version parsing is called correctly
        let version_request = VersionParser::parse("21").unwrap();
        assert!(version_request.version.is_some());
        assert_eq!(version_request.version.unwrap().major, 21);
        assert_eq!(version_request.distribution, None);
    }

    #[test]
    fn test_dry_run_prevents_installation() {
        // This would be a more complex test with mocks
        // For now, just verify the command can be created
        let cmd = InstallCommand::new();
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_parse_version_with_distribution() {
        let version_request = VersionParser::parse("corretto@17").unwrap();
        assert!(version_request.version.is_some());
        assert_eq!(version_request.version.unwrap().major, 17);
        assert_eq!(version_request.distribution, Some(Distribution::Corretto));
    }

    #[test]
    fn test_get_current_architecture() {
        let cmd = InstallCommand::new().unwrap();
        let arch = cmd.get_current_architecture();
        // Should return a valid architecture string
        assert!(!arch.is_empty());
        assert_ne!(arch, "unknown");
    }

    #[test]
    fn test_get_current_os() {
        let cmd = InstallCommand::new().unwrap();
        let os = cmd.get_current_os();
        // Should return a valid OS string
        assert!(!os.is_empty());
        assert_ne!(os, "unknown");
    }

    #[test]
    fn test_convert_metadata_to_package() {
        use crate::models::jdk::{
            Architecture, ArchiveType, ChecksumType, OperatingSystem, PackageType, Version,
        };

        let cmd = InstallCommand::new().unwrap();

        let metadata = JdkMetadata {
            id: "test-id".to_string(),
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            distribution_version: "21.0.1+12".to_string(),
            architecture: Architecture::X64,
            operating_system: OperatingSystem::Linux,
            package_type: PackageType::Jdk,
            archive_type: ArchiveType::TarGz,
            download_url: "https://example.com/download".to_string(),
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
}
