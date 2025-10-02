use kopi::config::LockingConfig;
use kopi::locking::{
    LockAcquisition, LockController, LockHygieneRunner, LockScope, PackageCoordinate, PackageKind,
};
use kopi::paths::locking::{cache_lock_path, locks_root};
use std::time::Duration;
use tempfile::TempDir;

fn installation_scope() -> LockScope {
    let coordinate = PackageCoordinate::new("temurin", 21, PackageKind::Jdk);
    LockScope::installation(coordinate)
}

#[test]
fn advisory_and_fallback_lifecycle() {
    let temp = TempDir::new().unwrap();
    let mut config = LockingConfig::default();

    // Advisory path
    config.mode = kopi::config::LockingMode::Advisory;
    let controller = LockController::with_default_inspector(temp.path().to_path_buf(), &config);
    let scope = installation_scope();
    let acquisition = controller.acquire(scope.clone()).unwrap();
    assert!(matches!(acquisition, LockAcquisition::Advisory(_)));
    let contended = controller.try_acquire(scope.clone()).unwrap();
    assert!(contended.is_none());
    controller.release(acquisition).unwrap();

    // Fallback path
    config.mode = kopi::config::LockingMode::Fallback;
    let controller = LockController::with_default_inspector(temp.path().to_path_buf(), &config);
    let scope = LockScope::CacheWriter;
    let acquisition = controller.acquire(scope.clone()).unwrap();
    assert!(matches!(acquisition, LockAcquisition::Fallback(_)));
    let lock_path = scope.lock_path(temp.path());
    assert!(lock_path.exists());
    assert!(lock_path.with_file_name("cache.lock.marker").exists());
    controller.release(acquisition).unwrap();
    assert!(!lock_path.exists());
    assert!(!lock_path.with_file_name("cache.lock.marker").exists());
}

#[test]
fn hygiene_cleans_stale_artifacts() {
    let temp = TempDir::new().unwrap();
    let root = locks_root(temp.path());
    std::fs::create_dir_all(&root).unwrap();

    let lock_path = cache_lock_path(temp.path());
    let marker_path = lock_path.with_file_name("cache.lock.marker");
    write_file(&lock_path, b"fallback");
    write_file(&marker_path, b"marker");

    let runner = LockHygieneRunner::new(root, Duration::from_secs(0));
    let report = runner.run().unwrap();

    assert_eq!(report.removed_locks, 1);
    assert_eq!(report.removed_markers, 1);
    assert!(!lock_path.exists());
    assert!(!marker_path.exists());
}

#[test]
#[ignore]
fn fallback_crash_simulation_cleanup() {
    let temp = TempDir::new().unwrap();
    let root = locks_root(temp.path());
    std::fs::create_dir_all(&root).unwrap();
    let runner = LockHygieneRunner::new(root.clone(), Duration::from_secs(0));

    for idx in 0..1000 {
        let lock_path = root.join(format!("install-{idx}.lock"));
        let marker_path = lock_path.with_file_name(format!("install-{idx}.lock.marker"));
        write_file(&lock_path, b"fallback");
        write_file(&marker_path, b"marker");

        let report = runner.run().unwrap();
        assert_eq!(report.removed_locks, 1);
        assert_eq!(report.removed_markers, 1);
        assert!(!lock_path.exists());
        assert!(!marker_path.exists());
    }
}

fn write_file(path: &std::path::Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}
