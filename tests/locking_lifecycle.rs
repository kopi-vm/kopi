use kopi::config::LockingConfig;
use kopi::error::KopiError;
use kopi::indicator::{ProgressConfig, ProgressIndicator, ProgressStyle};
use kopi::locking::{
    LockAcquisition, LockController, LockHygieneRunner, LockScope, LockStatusSink,
    LockTimeoutSource, LockTimeoutValue, PackageCoordinate, PackageKind,
};
use kopi::paths::locking::{cache_lock_path, locks_root};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

mod common;
use common::progress_capture::TestProgressCapture;

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

#[test]
fn timeout_override_and_observer_feedback() {
    let temp = TempDir::new().unwrap();
    let mut config = LockingConfig::default();
    config.set_timeout_value(LockTimeoutValue::from_secs(15));

    let resolution = config
        .resolve_timeout(Some("7"), Some("11"))
        .expect("CLI override should resolve");
    assert_eq!(resolution.value, LockTimeoutValue::from_secs(7));
    assert_eq!(resolution.source, LockTimeoutSource::Cli);

    let resolution = config
        .resolve_timeout(None, Some("0"))
        .expect("environment override should resolve");
    assert_eq!(resolution.value, LockTimeoutValue::from_secs(0));
    assert_eq!(resolution.source, LockTimeoutSource::Environment);

    let controller = LockController::with_default_inspector(temp.path().to_path_buf(), &config);
    let scope = installation_scope();

    let first = controller.acquire(scope.clone()).unwrap();

    let sink = RecordingSink::default();
    let err = controller
        .acquire_with_status_sink(scope.clone(), &sink)
        .unwrap_err();
    assert!(matches!(
        err,
        KopiError::LockingTimeout {
            timeout_source: LockTimeoutSource::Environment,
            timeout_value,
            ..
        } if timeout_value == LockTimeoutValue::from_secs(0)
    ));

    let messages = sink.messages();
    assert!(
        messages
            .iter()
            .any(|line| line.contains("source environment variable"))
    );
    assert!(
        messages
            .iter()
            .any(|line| line.contains("Timed out waiting for installation"))
    );

    controller.release(first).unwrap();
}

#[test]
fn acquire_with_feedback_emits_wait_messages() {
    let temp = TempDir::new().unwrap();
    let controller = Arc::new(LockController::with_default_inspector(
        temp.path().to_path_buf(),
        &LockingConfig::default(),
    ));
    let scope = installation_scope();

    let first = controller.acquire(scope.clone()).unwrap();
    let controller_clone = controller.clone();
    let release_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        controller_clone.release(first).unwrap();
    });

    let capture = TestProgressCapture::new();
    let indicator = Arc::new(Mutex::new(
        Box::new(capture.clone()) as Box<dyn ProgressIndicator>
    ));
    {
        let mut handle = indicator.lock().unwrap();
        handle.start(ProgressConfig::new(ProgressStyle::Status));
    }

    let acquisition = controller
        .acquire_with_feedback(scope.clone(), indicator.clone())
        .unwrap();
    controller.release(acquisition).unwrap();
    release_thread.join().unwrap();

    let messages = capture.get_messages();
    assert!(
        messages
            .iter()
            .any(|msg| msg.message.contains("Waiting for lock on installation")),
        "expected wait message, got {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|msg| msg.message.contains("Lock acquired")),
        "expected acquisition message, got {messages:?}"
    );
}

#[derive(Default)]
struct RecordingSink {
    events: std::sync::Mutex<Vec<String>>,
}

impl RecordingSink {
    fn messages(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

impl LockStatusSink for RecordingSink {
    fn step(&self, message: &str) {
        self.events.lock().unwrap().push(message.to_string());
    }

    fn success(&self, message: &str) {
        self.events.lock().unwrap().push(message.to_string());
    }

    fn error(&self, message: &str) {
        self.events.lock().unwrap().push(message.to_string());
    }
}

fn write_file(path: &std::path::Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}
