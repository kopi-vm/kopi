use crate::cache::{DistributionCache, MetadataCache};
use crate::error::Result;
use crate::models::api::ApiMetadata;
use crate::models::distribution::Distribution as JdkDistribution;
use crate::models::metadata::JdkMetadata;
use crate::models::package::{ArchiveType, ChecksumType, PackageType};
use crate::models::platform::{Architecture, OperatingSystem};
use crate::version::Version;
use std::str::FromStr;

/// Parse architecture from filename patterns
pub fn parse_architecture_from_filename(filename: &str) -> Option<Architecture> {
    // Common architecture patterns in filenames
    let patterns = [
        ("x64", "x64"),
        ("x86_64", "x64"),
        ("amd64", "x64"),
        ("aarch64", "aarch64"),
        ("arm64", "aarch64"),
        ("x86", "x86"),
        ("i386", "x86"),
        ("i686", "x86"),
        ("arm32", "arm32"),
        ("ppc64le", "ppc64le"),
        ("ppc64", "ppc64"),
        ("s390x", "s390x"),
        ("sparcv9", "sparcv9"),
    ];

    for (pattern, arch_str) in patterns.iter() {
        if filename.contains(pattern) {
            return Architecture::from_str(arch_str).ok();
        }
    }

    None
}

/// Convert an API package to JDK metadata
pub fn convert_package_to_jdk_metadata(
    api_package: crate::models::api::Package,
) -> Result<JdkMetadata> {
    // Parse version
    let version = Version::from_str(&api_package.java_version)
        .unwrap_or_else(|_| Version::new(api_package.major_version, 0, 0));

    // Parse distribution_version
    let distribution_version =
        Version::from_str(&api_package.distribution_version).unwrap_or_else(|_| version.clone());

    // Parse architecture from filename
    let architecture =
        parse_architecture_from_filename(&api_package.filename).unwrap_or(Architecture::X64);

    // Parse operating system
    let operating_system =
        OperatingSystem::from_str(&api_package.operating_system).unwrap_or(OperatingSystem::Linux);

    // Parse archive type
    let archive_type =
        ArchiveType::from_str(&api_package.archive_type).unwrap_or(ArchiveType::TarGz);

    let package_type = PackageType::from_str(&api_package.package_type).unwrap_or(PackageType::Jdk);

    let jdk_metadata = JdkMetadata {
        id: api_package.id,
        distribution: api_package.distribution,
        version,
        distribution_version,
        architecture,
        operating_system,
        package_type,
        archive_type,
        download_url: Some(api_package.links.pkg_download_redirect),
        checksum: None, // TODO: Fetch from API if available
        checksum_type: Some(ChecksumType::Sha256),
        size: api_package.size,
        lib_c_type: api_package.lib_c_type,
        javafx_bundled: api_package.javafx_bundled,
        term_of_support: api_package.term_of_support,
        release_status: api_package.release_status,
        latest_build_available: api_package.latest_build_available,
    };

    Ok(jdk_metadata)
}

/// Convert API metadata response to cache format
pub fn convert_api_to_cache(api_metadata: ApiMetadata) -> Result<MetadataCache> {
    let mut cache = MetadataCache::new();

    // Convert API format to cache format
    for dist_metadata in api_metadata.distributions {
        let dist_info = dist_metadata.distribution;

        // Build synonym map: each synonym points to the canonical api_parameter
        for synonym in &dist_info.synonyms {
            cache
                .synonym_map
                .insert(synonym.clone(), dist_info.api_parameter.clone());
        }

        // Also add the api_parameter itself as a synonym pointing to itself
        cache.synonym_map.insert(
            dist_info.api_parameter.clone(),
            dist_info.api_parameter.clone(),
        );

        // Parse distribution
        let distribution = JdkDistribution::from_str(&dist_info.api_parameter)
            .unwrap_or(JdkDistribution::Other(dist_info.api_parameter.clone()));

        // Convert API packages to JdkMetadata
        let packages: Vec<JdkMetadata> = dist_metadata
            .packages
            .into_iter()
            .filter_map(|pkg| convert_package_to_jdk_metadata(pkg).ok())
            .collect();

        let dist_cache = DistributionCache {
            distribution,
            display_name: dist_info.name,
            packages,
        };

        cache
            .distributions
            .insert(dist_info.api_parameter, dist_cache);
    }

    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api::{Links, Package};

    #[test]
    fn test_parse_architecture_from_filename() {
        assert_eq!(
            parse_architecture_from_filename("OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz"),
            Some(Architecture::X64)
        );
        assert_eq!(
            parse_architecture_from_filename(
                "OpenJDK21U-jdk_aarch64_linux_hotspot_21.0.1_12.tar.gz"
            ),
            Some(Architecture::Aarch64)
        );
        assert_eq!(
            parse_architecture_from_filename("amazon-corretto-21.0.1.12.1-linux-x86_64.tar.gz"),
            Some(Architecture::X64)
        );
        assert_eq!(
            parse_architecture_from_filename("some_file_without_arch.tar.gz"),
            None
        );
    }

    #[test]
    fn test_convert_package_to_jdk_metadata() {
        let api_package = Package {
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
            links: Links {
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

        let result = convert_package_to_jdk_metadata(api_package);
        assert!(result.is_ok());

        let jdk_metadata = result.unwrap();
        assert_eq!(jdk_metadata.id, "test123");
        assert_eq!(jdk_metadata.distribution, "temurin");
        assert_eq!(jdk_metadata.version.major(), 21);
        // Architecture is parsed from filename
        assert_eq!(jdk_metadata.architecture.to_string(), "x64");
    }
}
