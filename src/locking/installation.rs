use crate::error::{KopiError, Result};
use crate::locking::{LockScope, PackageCoordinate, PackageKind};
use crate::models::api::Package;
use crate::storage::{
    InstallationMetadata, InstalledJdk, InstalledMetadataSnapshot, JdkRepository,
};
use log::warn;
use std::path::Path;
pub fn installation_lock_scope_from_package(package: &Package) -> Result<LockScope> {
    let coordinate = PackageCoordinate::try_from_package(package)?;
    let mut tags = coordinate.variant_tags().to_vec();
    tags.push(package.distribution_version.clone());
    let coordinate = coordinate.with_variant_tags(tags);
    Ok(LockScope::installation(coordinate))
}

pub struct InstalledScopeResolver<'repo, 'config> {
    repository: &'repo JdkRepository<'config>,
}

impl<'repo, 'config> InstalledScopeResolver<'repo, 'config> {
    pub fn new(repository: &'repo JdkRepository<'config>) -> Self {
        Self { repository }
    }

    pub fn resolve(&self, installed: &InstalledJdk) -> Result<LockScope> {
        let InstalledMetadataSnapshot {
            metadata,
            installation_metadata,
        } = self.repository.load_installed_metadata(installed)?;

        if let Some(metadata) = metadata {
            return installation_lock_scope_from_package(&metadata.package);
        }

        warn!(
            "Falling back to slug-derived lock scope for {} due to missing or unreadable metadata",
            installed.path.display()
        );

        self.fallback_scope(installed, installation_metadata.as_ref())
    }

    fn fallback_scope(
        &self,
        installed: &InstalledJdk,
        installation_metadata: Option<&InstallationMetadata>,
    ) -> Result<LockScope> {
        let slug = installation_slug(&installed.path).ok_or_else(|| {
            KopiError::LockingScopeUnavailable {
                slug: installed.path.display().to_string(),
                reason: "installation directory name is missing".to_string(),
            }
        })?;

        let distribution =
            resolved_distribution(&installed.distribution, &slug).ok_or_else(|| {
                KopiError::LockingScopeUnavailable {
                    slug: slug.clone(),
                    reason: "unable to determine distribution".to_string(),
                }
            })?;

        let mut coordinate =
            PackageCoordinate::new(distribution, installed.version.major(), PackageKind::Jdk)
                .with_javafx(installed.javafx_bundled);

        if let Some(metadata) = installation_metadata {
            let split = split_platform_tokens(&metadata.platform);
            coordinate = coordinate
                .with_operating_system(split.operating_system)
                .with_architecture(split.architecture)
                .with_libc_variant(split.libc_variant);
        }

        let mut variant_tags = Vec::new();
        variant_tags.push(installed.version.to_string());
        variant_tags.push(slug.clone());

        if let Some(metadata) = installation_metadata
            && !metadata.platform.trim().is_empty()
        {
            variant_tags.push(metadata.platform.clone());
        }

        coordinate = coordinate.with_variant_tags(variant_tags);

        Ok(LockScope::installation(coordinate))
    }
}

struct PlatformTokens {
    architecture: Option<String>,
    operating_system: Option<String>,
    libc_variant: Option<String>,
}

fn split_platform_tokens(platform: &str) -> PlatformTokens {
    let mut parts = platform
        .split('_')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_ascii_lowercase())
            }
        })
        .collect::<Vec<_>>();

    if parts.len() > 3 {
        parts.truncate(3);
    }

    PlatformTokens {
        operating_system: parts.first().cloned(),
        architecture: parts.get(1).cloned(),
        libc_variant: parts.get(2).cloned(),
    }
}

fn installation_slug(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|value| value.to_string())
}

fn resolved_distribution<'a>(distribution: &'a str, slug: &'a str) -> Option<String> {
    if !distribution.trim().is_empty() {
        return Some(distribution.trim().to_string());
    }

    slug.split('-')
        .next()
        .map(|segment| segment.trim().to_string())
        .filter(|segment| !segment.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::locking::LockScope;
    use crate::models::api::{Links, Package};
    use crate::storage::{InstalledJdk, JdkMetadataWithInstallation, JdkRepository};
    use crate::version::Version;
    use std::fs;
    use std::path::Path;
    use std::str::FromStr;
    use tempfile::TempDir;
    fn sample_package() -> Package {
        Package {
            id: "pkg-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "Temurin".to_string(),
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
            javafx_bundled: true,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        }
    }

    #[test]
    fn installation_scope_uses_distribution_version_tag() {
        let package = sample_package();
        let scope = installation_lock_scope_from_package(&package).unwrap();
        let locks_root = Path::new("/tmp/kopi");
        let path = scope.lock_path(locks_root);
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("temurin"));
        assert!(path_str.contains("21-0-2"));
        assert!(path_str.ends_with(".lock"));
    }

    #[test]
    fn resolver_uses_metadata_when_available() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let resolver = InstalledScopeResolver::new(&repository);

        let slug = "temurin-21.0.2";
        let install_path = config.jdks_dir().unwrap().join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let package = sample_package();
        let installation_metadata = InstallationMetadata {
            java_home_suffix: String::new(),
            structure_type: "direct".to_string(),
            platform: "linux_x64".to_string(),
            metadata_version: 1,
        };

        let metadata_path = crate::paths::install::metadata_file(config.kopi_home(), slug);
        fs::write(
            &metadata_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&JdkMetadataWithInstallation {
                    package: package.clone(),
                    installation_metadata: installation_metadata.clone(),
                })
                .unwrap()
            ),
        )
        .unwrap();

        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.2").unwrap(),
            install_path,
            true,
        );

        let scope = resolver.resolve(&installed).unwrap();
        let expected = installation_lock_scope_from_package(&package).unwrap();
        assert_eq!(scope, expected);
    }

    #[test]
    fn resolver_falls_back_with_platform_tokens() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let resolver = InstalledScopeResolver::new(&repository);

        let slug = "temurin-21.0.3";
        let install_path = config.jdks_dir().unwrap().join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let metadata_path = crate::paths::install::metadata_file(config.kopi_home(), slug);
        fs::write(
            &metadata_path,
            r#"{
    "installation_metadata": {
        "java_home_suffix": "",
        "structure_type": "direct",
        "platform": "linux_x64_musl",
        "metadata_version": 1
    },
    "package": "invalid"
}
"#,
        )
        .unwrap();

        let installed = InstalledJdk::new(
            "temurin".to_string(),
            Version::from_str("21.0.3").unwrap(),
            install_path,
            false,
        );

        let scope = resolver.resolve(&installed).unwrap();
        if let LockScope::Installation { coordinate } = scope {
            assert_eq!(coordinate.operating_system(), Some("linux"));
            assert_eq!(coordinate.architecture(), Some("x64"));
            assert_eq!(coordinate.libc_variant(), Some("musl"));
            assert!(coordinate.variant_tags().iter().any(|tag| tag == "21.0.3"));
            assert!(
                coordinate
                    .variant_tags()
                    .iter()
                    .any(|tag| tag == "temurin-21.0.3")
            );
            assert!(coordinate.slug().contains("21-0-3"));
        } else {
            panic!("expected installation scope");
        }
    }

    #[test]
    fn resolver_falls_back_without_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let resolver = InstalledScopeResolver::new(&repository);

        let slug = "corretto-21.0.4";
        let install_path = config.jdks_dir().unwrap().join(slug);
        fs::create_dir_all(&install_path).unwrap();

        let installed = InstalledJdk::new(
            "corretto".to_string(),
            Version::from_str("21.0.4").unwrap(),
            install_path,
            false,
        );

        let scope = resolver.resolve(&installed).unwrap();
        if let LockScope::Installation { coordinate } = scope {
            assert_eq!(coordinate.distribution(), "corretto");
            assert_eq!(coordinate.architecture(), None);
            assert!(coordinate.variant_tags().iter().any(|tag| tag == "21.0.4"));
            assert!(
                coordinate
                    .variant_tags()
                    .iter()
                    .any(|tag| tag == "corretto-21.0.4")
            );
            assert!(coordinate.slug().contains("21-0-4"));
        } else {
            panic!("expected installation scope");
        }
    }
}
