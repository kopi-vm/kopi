use kopi::config::LockingConfig;
use kopi::error::KopiError;
use kopi::locking::{
    CancellationToken, LockAcquisitionRequest, LockController, LockScope, LockTimeoutValue,
    PollingBackoff,
};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn timeout_smoke_respects_zero_config() {
    let temp = TempDir::new().unwrap();
    let scope = LockScope::CacheWriter;

    let holder_config = LockingConfig::default();
    let holder = LockController::with_default_inspector(temp.path().to_path_buf(), &holder_config);
    let held = holder
        .acquire(scope.clone())
        .expect("initial lock acquisition should succeed");

    let mut timeout_config = LockingConfig::default();
    timeout_config.set_timeout_value(LockTimeoutValue::from_secs(0));
    let waiter = LockController::with_default_inspector(temp.path().to_path_buf(), &timeout_config);

    let err = waiter
        .acquire(scope.clone())
        .expect_err("zero-second timeout should fail when lock is held");
    match err {
        KopiError::LockingTimeout { timeout_value, .. } => {
            assert_eq!(timeout_value, LockTimeoutValue::from_secs(0))
        }
        other => panic!("expected timeout error, got {other:?}"),
    }

    holder.release(held).expect("should release held lock");
}

#[test]
fn cancellation_smoke_respects_token() {
    let temp = TempDir::new().unwrap();
    let scope = LockScope::CacheWriter;

    let holder_config = LockingConfig::default();
    let holder = LockController::with_default_inspector(temp.path().to_path_buf(), &holder_config);
    let held = holder
        .acquire(scope.clone())
        .expect("initial lock acquisition should succeed");

    let mut waiter_config = LockingConfig::default();
    waiter_config.set_timeout_value(LockTimeoutValue::from_secs(5));
    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();
    let scope_for_thread = scope.clone();
    let path = temp.path().to_path_buf();

    let handle = thread::spawn(move || {
        let waiter = LockController::with_default_inspector(path, &waiter_config);
        let request = LockAcquisitionRequest::new(scope_for_thread, LockTimeoutValue::from_secs(5))
            .with_cancellation(cancel_clone)
            .with_backoff(PollingBackoff::default());
        waiter.acquire_with(request)
    });

    thread::sleep(Duration::from_millis(150));
    cancel_token.cancel();

    match handle.join().expect("thread should finish acquiring") {
        Err(KopiError::LockingCancelled { .. }) => {}
        other => panic!("expected cancellation error, got {other:?}"),
    }

    holder.release(held).expect("should release held lock");
}
