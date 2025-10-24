use std::ffi::OsString;
use std::sync::Arc;

use kopi::config::LockingConfig;
use kopi::error::KopiError;
use kopi::locking::{
    LockBackend, LockController, LockTimeoutValue, ScopedPackageLockGuard,
    installation_lock_scope_from_package,
};
use kopi::models::api::{Links, Package};
use kopi::platform::{AdvisorySupport, FilesystemInfo, FilesystemInspector, FilesystemKind};
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
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    }
}

#[test]
fn installation_lock_serializes_writers() {
    let temp = TempDir::new().unwrap();
    let locking_home = temp.path().to_path_buf();

    let config = LockingConfig::default();
    let controller = LockController::with_default_inspector(locking_home.clone(), &config);

    let package = sample_package();
    let scope = installation_lock_scope_from_package(&package).unwrap();

    let primary = controller.acquire(scope.clone()).unwrap();
    let guard = ScopedPackageLockGuard::new(&controller, primary);

    let second = controller.try_acquire(scope.clone()).unwrap();
    assert!(
        second.is_none(),
        "secondary acquisition should block while lock held"
    );

    drop(guard);

    let reacquired = controller.try_acquire(scope.clone()).unwrap();
    assert!(
        reacquired.is_some(),
        "lock should be available after release"
    );

    if let Some(handle) = reacquired {
        ScopedPackageLockGuard::new(&controller, handle)
            .release()
            .unwrap();
    }
}

#[test]
fn installation_lock_honours_timeout() {
    let temp = TempDir::new().unwrap();
    let locking_home = temp.path().to_path_buf();

    let mut config = LockingConfig::default();
    config.set_timeout_value(LockTimeoutValue::from_secs(0));
    let controller = LockController::with_default_inspector(locking_home, &config);

    let package = sample_package();
    let scope = installation_lock_scope_from_package(&package).unwrap();

    let primary = controller.acquire(scope.clone()).unwrap();
    let guard = ScopedPackageLockGuard::new(&controller, primary);

    let err = controller.acquire(scope.clone()).unwrap_err();
    match err {
        KopiError::LockingTimeout { scope: label, .. } => {
            assert!(label.contains("installation"));
        }
        other => panic!("expected timeout error, got {other:?}"),
    }

    drop(guard);
}

#[derive(Debug)]
struct FallbackInspector;

impl FilesystemInspector for FallbackInspector {
    fn classify(&self, _path: &std::path::Path) -> kopi::error::Result<FilesystemInfo> {
        Ok(FilesystemInfo {
            kind: FilesystemKind::Nfs,
            advisory_support: AdvisorySupport::RequiresFallback,
            is_network_share: true,
        })
    }
}

#[test]
fn fallback_lock_creates_and_cleans_marker() {
    let temp = TempDir::new().unwrap();
    let locking_home = temp.path().to_path_buf();
    let config = LockingConfig::default();

    let inspector: Arc<dyn FilesystemInspector> = Arc::new(FallbackInspector);
    let controller = LockController::new(locking_home.clone(), &config, inspector);

    let package = sample_package();
    let scope = installation_lock_scope_from_package(&package).unwrap();
    let lock_path = scope.lock_path(&locking_home);
    let marker_path = {
        let mut name = OsString::from(lock_path.file_name().unwrap());
        name.push(".marker");
        lock_path.with_file_name(name)
    };

    let acquisition = controller.acquire(scope.clone()).unwrap();
    let guard = ScopedPackageLockGuard::new(&controller, acquisition);
    assert_eq!(guard.backend(), LockBackend::Fallback);
    assert!(lock_path.exists());
    assert!(marker_path.exists());

    guard.release().unwrap();

    assert!(
        !lock_path.exists() && !marker_path.exists(),
        "fallback artifacts for {lock_path:?} should be removed after release"
    );
}
