use crate::error::Result;
use crate::locking::{LockAcquisition, LockBackend, LockController, LockScope, PackageCoordinate};
use crate::models::api::Package;
use log::warn;

/// RAII guard that releases an installation lock when dropped.
#[cfg_attr(not(test), allow(dead_code))]
pub struct InstallationLockGuard<'a> {
    controller: &'a LockController,
    acquisition: Option<LockAcquisition>,
    backend: LockBackend,
    scope_label: String,
}

#[cfg_attr(not(test), allow(dead_code))]
impl<'a> InstallationLockGuard<'a> {
    pub fn new(controller: &'a LockController, acquisition: LockAcquisition) -> Self {
        let backend = acquisition.backend();
        let scope_label = acquisition.scope().label();
        Self {
            controller,
            acquisition: Some(acquisition),
            backend,
            scope_label,
        }
    }

    pub fn backend(&self) -> LockBackend {
        self.backend
    }

    #[allow(dead_code)]
    pub fn scope_label(&self) -> &str {
        &self.scope_label
    }

    pub fn release(mut self) -> Result<()> {
        if let Some(acquisition) = self.acquisition.take() {
            self.controller.release(acquisition)
        } else {
            Ok(())
        }
    }
}

impl Drop for InstallationLockGuard<'_> {
    fn drop(&mut self) {
        if let Some(acquisition) = self.acquisition.take()
            && let Err(err) = self.controller.release(acquisition)
        {
            warn!(
                "Failed to release installation lock for {}: {err}",
                self.scope_label
            );
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn installation_lock_scope_from_package(package: &Package) -> Result<LockScope> {
    let coordinate = PackageCoordinate::try_from_package(package)?;
    let mut tags = coordinate.variant_tags().to_vec();
    tags.push(package.distribution_version.clone());
    let coordinate = coordinate.with_variant_tags(tags);
    Ok(LockScope::installation(coordinate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LockingConfig;
    use crate::locking::{LockKind, LockScope};
    use crate::models::api::{Links, Package};
    use std::path::Path;
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
    fn guard_release_allows_reacquire() {
        let temp = TempDir::new().unwrap();
        let controller = LockController::with_default_inspector(
            temp.path().to_path_buf(),
            &LockingConfig::default(),
        );
        let scope = LockScope::CacheWriter;
        let acquisition = controller.acquire(scope.clone()).unwrap();
        {
            let guard = InstallationLockGuard::new(&controller, acquisition);
            assert_eq!(guard.backend(), LockBackend::Advisory);
            assert_eq!(scope.lock_kind(), LockKind::Exclusive);
        }
        let reacquired = controller.acquire(scope).unwrap();
        drop(reacquired);
    }

    #[test]
    fn explicit_release_returns_ok() {
        let temp = TempDir::new().unwrap();
        let controller = LockController::with_default_inspector(
            temp.path().to_path_buf(),
            &LockingConfig::default(),
        );
        let scope = LockScope::CacheWriter;
        let acquisition = controller.acquire(scope.clone()).unwrap();
        let guard = InstallationLockGuard::new(&controller, acquisition);
        guard.release().unwrap();
        let reacquired = controller.acquire(scope).unwrap();
        drop(reacquired);
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
}
