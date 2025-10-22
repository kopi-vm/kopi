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

use crate::config::{LockingConfig, LockingMode};
use crate::error::{KopiError, Result};
use crate::locking::fallback::{self, FallbackAcquire};
use crate::locking::handle::{FallbackHandle, LockBackend, LockHandle};
use crate::locking::scope::{LockKind, LockScope};
use crate::locking::{
    AcquireMode, LockAcquisitionRequest, LockStatusSink, LockTimeoutSource, LockTimeoutValue,
    LockWaitObserver, PollingBackoff, StatusReporterObserver, global_token,
};
use crate::platform::{AdvisorySupport, DefaultFilesystemInspector, FilesystemInspector};
use log::{debug, info};
use std::fs::TryLockError;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Result of a lock attempt when considering fallback downgraded paths.
#[derive(Debug)]
pub enum LockAcquisition {
    Advisory(LockHandle),
    Fallback(Box<FallbackHandle>),
}

impl LockAcquisition {
    pub fn backend(&self) -> LockBackend {
        match self {
            LockAcquisition::Advisory(_) => LockBackend::Advisory,
            LockAcquisition::Fallback(_) => LockBackend::Fallback,
        }
    }

    pub fn scope(&self) -> &LockScope {
        match self {
            LockAcquisition::Advisory(handle) => handle.scope(),
            LockAcquisition::Fallback(handle) => handle.scope(),
        }
    }

    pub fn release(self) -> Result<()> {
        match self {
            LockAcquisition::Advisory(handle) => handle.release(),
            LockAcquisition::Fallback(handle) => (*handle).release(),
        }
    }
}

/// Coordinates advisory locking and fallback behaviour across filesystems.
pub struct LockController {
    kopi_home: PathBuf,
    inspector: Arc<dyn FilesystemInspector>,
    preferred_mode: LockingMode,
    timeout: LockTimeoutValue,
    backoff_config: BackoffConfig,
    timeout_source: LockTimeoutSource,
}

#[derive(Debug, Clone, Copy)]
struct BackoffConfig {
    initial: Duration,
    factor: u32,
    cap: Duration,
}

impl BackoffConfig {
    fn polling_backoff(self) -> PollingBackoff {
        PollingBackoff::new(self.initial, self.factor, self.cap)
    }
}

impl LockController {
    pub fn with_default_inspector<P: Into<PathBuf>>(kopi_home: P, config: &LockingConfig) -> Self {
        Self::new(
            kopi_home,
            config,
            Arc::new(DefaultFilesystemInspector::new()),
        )
    }

    pub fn new<P: Into<PathBuf>>(
        kopi_home: P,
        config: &LockingConfig,
        inspector: Arc<dyn FilesystemInspector>,
    ) -> Self {
        Self {
            kopi_home: kopi_home.into(),
            inspector,
            preferred_mode: config.mode,
            timeout: config.timeout_value(),
            backoff_config: BackoffConfig {
                initial: Duration::from_millis(10),
                factor: 2,
                cap: Duration::from_secs(1),
            },
            timeout_source: config.timeout_source(),
        }
    }

    pub fn acquire(&self, scope: LockScope) -> Result<LockAcquisition> {
        let request = self.build_request(scope, AcquireMode::Blocking, None);
        self.acquire_with(request)
    }

    pub fn try_acquire(&self, scope: LockScope) -> Result<Option<LockAcquisition>> {
        let request = self.build_request(scope, AcquireMode::NonBlocking, None);
        match self.acquire_internal(request)? {
            AcquireDisposition::Acquired(acquired) => Ok(Some(acquired)),
            AcquireDisposition::NotAcquired => Ok(None),
        }
    }

    pub fn acquire_with_observer(
        &self,
        scope: LockScope,
        observer: Option<&dyn LockWaitObserver>,
    ) -> Result<LockAcquisition> {
        let request = self.build_request(scope, AcquireMode::Blocking, observer);
        self.acquire_with(request)
    }

    pub fn acquire_with_status_sink(
        &self,
        scope: LockScope,
        sink: &dyn LockStatusSink,
    ) -> Result<LockAcquisition> {
        let observer = StatusReporterObserver::new(sink, self.timeout_source);
        let request = self.build_request(scope, AcquireMode::Blocking, Some(&observer));
        self.acquire_with(request)
    }

    pub fn acquire_with<'a>(&self, request: LockAcquisitionRequest<'a>) -> Result<LockAcquisition> {
        let scope_label = request.scope().to_string();
        match self.acquire_internal(request)? {
            AcquireDisposition::Acquired(acquired) => Ok(acquired),
            AcquireDisposition::NotAcquired => Err(KopiError::LockingAcquire {
                scope: scope_label,
                details: "Lock acquisition unexpectedly returned without handle".to_string(),
            }),
        }
    }

    pub fn release(&self, acquisition: LockAcquisition) -> Result<()> {
        acquisition.release()
    }

    fn build_request<'a>(
        &self,
        scope: LockScope,
        mode: AcquireMode,
        observer: Option<&'a dyn LockWaitObserver>,
    ) -> LockAcquisitionRequest<'a> {
        LockAcquisitionRequest::new(scope, self.timeout)
            .with_mode(mode)
            .with_backoff(self.backoff_config.polling_backoff())
            .with_cancellation(global_token())
            .with_timeout_source(self.timeout_source)
            .with_observer(observer)
    }

    fn acquire_internal<'a>(
        &self,
        mut request: LockAcquisitionRequest<'a>,
    ) -> Result<AcquireDisposition> {
        let scope = request.scope().clone();
        let mode = request.mode();
        debug!(
            "Attempting {:?} lock for {} with timeout {:?}",
            mode,
            scope,
            request.timeout_value()
        );
        debug!(
            "Lock timeout source for {}: {:?}",
            scope, self.timeout_source
        );
        let lock_path = scope.lock_path(&self.kopi_home);
        let parent = lock_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.kopi_home.clone());

        fs::create_dir_all(&parent).map_err(|err| KopiError::LockingAcquire {
            scope: scope.to_string(),
            details: format!(
                "Failed to ensure parent directory {} exists: {err}",
                parent.display()
            ),
        })?;

        let support = self.determine_support(&lock_path, &scope)?;
        debug!("Lock backend for {scope} selected as {support:?}");

        match support {
            LockBackend::Fallback => self.acquire_fallback(lock_path, &mut request),
            LockBackend::Advisory => self.acquire_advisory(lock_path, &mut request),
        }
    }

    fn determine_support(&self, lock_path: &Path, scope: &LockScope) -> Result<LockBackend> {
        match self.preferred_mode {
            LockingMode::Fallback => {
                info!(
                    "Locking mode forced to fallback for {} ({}).",
                    scope,
                    lock_path.display()
                );
                return Ok(LockBackend::Fallback);
            }
            LockingMode::Advisory => {
                debug!(
                    "Locking mode forced to advisory for {} ({}).",
                    scope,
                    lock_path.display()
                );
                return Ok(LockBackend::Advisory);
            }
            LockingMode::Auto => {}
        }

        let info = self.inspector.classify(lock_path)?;
        debug!(
            "Filesystem classification for {}: {:?}",
            lock_path.display(),
            info
        );
        match info.advisory_support {
            AdvisorySupport::Native => Ok(LockBackend::Advisory),
            AdvisorySupport::RequiresFallback => {
                info!(
                    "Downgrading {} lock to fallback because filesystem {:?} requires it",
                    scope, info.kind
                );
                Ok(LockBackend::Fallback)
            }
            AdvisorySupport::Unknown => Ok(LockBackend::Advisory),
        }
    }

    fn acquire_advisory(
        &self,
        lock_path: PathBuf,
        request: &mut LockAcquisitionRequest<'_>,
    ) -> Result<AcquireDisposition> {
        let scope = request.scope().clone();
        let is_shared = matches!(scope.lock_kind(), LockKind::Shared);
        let file = self
            .prepare_lock_file(&lock_path)
            .map_err(|err| KopiError::LockingAcquire {
                scope: scope.to_string(),
                details: format!("Failed to open lock file {}: {err}", lock_path.display()),
            })?;

        let mut last_detail: Option<String> = None;

        loop {
            if request.cancellation().is_cancelled() {
                request.notify_cancelled();
                return Err(KopiError::LockingCancelled {
                    scope: scope.to_string(),
                    waited_secs: request.elapsed().as_secs_f64(),
                });
            }

            match self.try_lock(&file, is_shared) {
                Ok(()) => {
                    debug!(
                        "Acquired advisory lock for {} after {:.3}s",
                        scope,
                        request.elapsed().as_secs_f64()
                    );
                    request.notify_acquired();
                    let handle =
                        LockHandle::new(scope, lock_path, file, request.budget().started_at());
                    return Ok(AcquireDisposition::Acquired(LockAcquisition::Advisory(
                        handle,
                    )));
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    let err_message = err.to_string();
                    if request.mode().is_non_blocking() {
                        return Ok(AcquireDisposition::NotAcquired);
                    }

                    request.record_wait_start();

                    if request.budget().is_expired() {
                        request.notify_timeout();
                        let detail = last_detail.take().unwrap_or_else(|| err_message.clone());
                        return Err(KopiError::LockingTimeout {
                            scope: scope.to_string(),
                            waited_secs: request.elapsed().as_secs_f64(),
                            timeout_value: request.timeout_value(),
                            timeout_source: request.timeout_source(),
                            details: detail,
                        });
                    }

                    last_detail = Some(err_message);
                    request.record_retry();
                    if let Some(sleep_for) = request.next_sleep_interval() {
                        thread::sleep(sleep_for);
                        continue;
                    }

                    request.notify_timeout();
                    let detail = last_detail.take().unwrap_or_else(|| {
                        "lock contention persisted without remaining timeout".to_string()
                    });
                    return Err(KopiError::LockingTimeout {
                        scope: scope.to_string(),
                        waited_secs: request.elapsed().as_secs_f64(),
                        timeout_value: request.timeout_value(),
                        timeout_source: request.timeout_source(),
                        details: detail,
                    });
                }
                Err(err) if err.kind() == io::ErrorKind::Unsupported => {
                    info!(
                        "Advisory locking unsupported for {} at {}; downgrading to fallback",
                        scope,
                        lock_path.display()
                    );
                    drop(file);
                    return self.acquire_fallback(lock_path, request);
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                    last_detail = Some(err.to_string());
                    continue;
                }
                Err(err) => {
                    return Err(KopiError::LockingAcquire {
                        scope: scope.to_string(),
                        details: err.to_string(),
                    });
                }
            }
        }
    }

    fn acquire_fallback(
        &self,
        lock_path: PathBuf,
        request: &mut LockAcquisitionRequest<'_>,
    ) -> Result<AcquireDisposition> {
        let scope = request.scope().clone();
        info!(
            "Acquiring fallback lock for {} at {}",
            scope,
            lock_path.display()
        );

        match fallback::acquire(lock_path, request)? {
            FallbackAcquire::Acquired(handle) => Ok(AcquireDisposition::Acquired(
                LockAcquisition::Fallback(handle),
            )),
            FallbackAcquire::NotAcquired => Ok(AcquireDisposition::NotAcquired),
        }
    }

    fn prepare_lock_file(&self, lock_path: &Path) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(lock_path, permissions)?;
        }

        Ok(file)
    }

    fn try_lock(&self, file: &File, shared: bool) -> io::Result<()> {
        let result = if shared {
            file.try_lock_shared()
        } else {
            file.try_lock()
        };

        match result {
            Ok(()) => Ok(()),
            Err(TryLockError::WouldBlock) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "lock would block",
            )),
            Err(TryLockError::Error(err)) => Err(err),
        }
    }
}

#[derive(Debug)]
enum AcquireDisposition {
    Acquired(LockAcquisition),
    NotAcquired,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::package_coordinate::PackageCoordinate;
    use crate::locking::{
        CancellationToken, LockAcquisitionRequest, LockKind, LockScope, LockStatusSink,
        LockTimeoutValue, LockWaitObserver, PackageKind, PollingBackoff,
    };
    use crate::platform::{FilesystemInfo, FilesystemInspector, FilesystemKind};
    use std::sync::Mutex;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    struct TestInspector {
        responses: Mutex<Vec<FilesystemInfo>>,
    }

    impl TestInspector {
        fn new(mut responses: Vec<FilesystemInfo>) -> Self {
            responses.reverse();
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl FilesystemInspector for TestInspector {
        fn classify(&self, _path: &Path) -> Result<FilesystemInfo> {
            let mut guard = self.responses.lock().unwrap();
            let info = guard.pop().unwrap_or_else(|| FilesystemInfo {
                kind: FilesystemKind::Other("test".to_string()),
                advisory_support: AdvisorySupport::Native,
                is_network_share: false,
            });
            Ok(info)
        }
    }

    #[derive(Default)]
    struct ObserverRecorder {
        events: Mutex<Vec<String>>,
    }

    impl LockWaitObserver for ObserverRecorder {
        fn on_wait_start(&self, scope: &LockScope, _timeout: LockTimeoutValue) {
            self.events.lock().unwrap().push(format!("start:{scope}"));
        }

        fn on_retry(
            &self,
            _scope: &LockScope,
            attempt: usize,
            _elapsed: Duration,
            _remaining: Option<Duration>,
        ) {
            self.events.lock().unwrap().push(format!("retry:{attempt}"));
        }

        fn on_cancelled(&self, scope: &LockScope, _waited: Duration) {
            self.events
                .lock()
                .unwrap()
                .push(format!("cancelled:{scope}"));
        }
    }

    fn native_fs() -> FilesystemInfo {
        FilesystemInfo {
            kind: FilesystemKind::Ext4,
            advisory_support: AdvisorySupport::Native,
            is_network_share: false,
        }
    }

    fn fallback_fs() -> FilesystemInfo {
        FilesystemInfo {
            kind: FilesystemKind::Nfs,
            advisory_support: AdvisorySupport::RequiresFallback,
            is_network_share: true,
        }
    }

    #[test]
    fn acquire_installation_lock_advisory() {
        let temp = TempDir::new().unwrap();
        let config = LockingConfig::default();
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let acquisition = controller.acquire(scope.clone()).unwrap();
        assert_eq!(acquisition.backend(), LockBackend::Advisory);
        assert_eq!(acquisition.scope(), &scope);
        controller.release(acquisition).unwrap();
    }

    #[test]
    fn try_acquire_returns_none_when_contended() {
        let temp = TempDir::new().unwrap();
        let config = LockingConfig::default();
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let first = controller.acquire(scope.clone()).unwrap();
        let second = controller.try_acquire(scope.clone()).unwrap();
        assert!(second.is_none());
        controller.release(first).unwrap();
    }

    #[test]
    fn blocking_acquire_times_out() {
        let temp = TempDir::new().unwrap();
        let mut config = LockingConfig::default();
        config.set_timeout_value(LockTimeoutValue::from_secs(1));
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs(), native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let first = controller.acquire(scope.clone()).unwrap();
        let start = Instant::now();
        let err = controller.acquire(scope.clone()).unwrap_err();
        match err {
            KopiError::LockingTimeout { scope: s, .. } => {
                assert!(s.contains("installation"));
            }
            other => panic!("Expected timeout error, got {other:?}"),
        }
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_secs(1));
        assert!(elapsed <= Duration::from_secs(2));
        controller.release(first).unwrap();
    }

    #[test]
    fn inspector_requires_fallback() {
        let temp = TempDir::new().unwrap();
        let config = LockingConfig::default();
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![fallback_fs()])),
        );
        let scope = LockScope::CacheWriter;

        let acquisition = controller.acquire(scope.clone()).unwrap();
        assert_eq!(acquisition.backend(), LockBackend::Fallback);
        assert_eq!(acquisition.scope(), &scope);
        controller.release(acquisition).unwrap();
    }

    #[test]
    fn blocking_acquire_cancels_when_token_triggered() {
        let temp = TempDir::new().unwrap();
        let mut config = LockingConfig::default();
        config.set_timeout_value(LockTimeoutValue::Infinite);
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs(), native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let first = controller.acquire(scope.clone()).unwrap();

        let token = CancellationToken::new();
        let cancel_token = token.clone();
        let observer = ObserverRecorder::default();

        let request = LockAcquisitionRequest::new(scope.clone(), LockTimeoutValue::Infinite)
            .with_backoff(PollingBackoff::new(
                Duration::from_millis(5),
                2,
                Duration::from_millis(40),
            ))
            .with_cancellation(token)
            .with_observer(Some(&observer));

        let cancel_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            cancel_token.cancel();
        });

        let err = controller.acquire_with(request).unwrap_err();
        match err {
            KopiError::LockingCancelled { scope: s, .. } => {
                assert!(s.contains("installation"));
            }
            other => panic!("Expected cancellation error, got {other:?}"),
        }

        cancel_handle.join().unwrap();

        let events = observer.events.lock().unwrap();
        assert!(
            events.iter().any(|e| e.starts_with("start")),
            "expected wait_start event, got {events:?}"
        );
        assert!(
            events.iter().any(|e| e.starts_with("retry")),
            "expected retry event, got {events:?}"
        );
        assert!(
            matches!(events.last(), Some(value) if value.starts_with("cancelled")),
            "expected cancellation event at end, got {events:?}"
        );
        drop(events);
        controller.release(first).unwrap();
    }

    #[test]
    fn acquire_with_status_sink_records_timeout_feedback() {
        let temp = TempDir::new().unwrap();
        let mut config = LockingConfig::default();
        config.set_timeout_value(LockTimeoutValue::from_secs(0));
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs(), native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let first = controller.acquire(scope.clone()).unwrap();

        let sink = RecordingSink::default();
        let err = controller
            .acquire_with_status_sink(scope.clone(), &sink)
            .unwrap_err();
        assert!(matches!(err, KopiError::LockingTimeout { .. }));

        let messages = sink.messages();
        assert!(
            messages
                .iter()
                .any(|line| line.contains("Waiting for installation"))
        );
        assert!(
            messages
                .iter()
                .any(|line| line.contains("Timed out waiting for installation"))
        );

        controller.release(first).unwrap();
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<String>>,
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

    #[test]
    fn forced_fallback_mode_bypasses_inspector() {
        let temp = TempDir::new().unwrap();
        let mut config = LockingConfig::default();
        config.mode = LockingMode::Fallback;
        let controller = LockController::with_default_inspector(temp.path().to_path_buf(), &config);
        let scope = LockScope::CacheWriter;

        let acquisition = controller.acquire(scope.clone()).unwrap();
        assert_eq!(acquisition.backend(), LockBackend::Fallback);
        controller.release(acquisition).unwrap();
    }

    #[test]
    fn forced_advisory_mode_attempts_lock() {
        let temp = TempDir::new().unwrap();
        let mut config = LockingConfig::default();
        config.mode = LockingMode::Advisory;
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![fallback_fs()])),
        );
        let scope = LockScope::CacheWriter;

        let acquisition = controller.acquire(scope.clone()).unwrap();
        assert_eq!(acquisition.backend(), LockBackend::Advisory);
        assert_eq!(scope.lock_kind(), LockKind::Exclusive);
        controller.release(acquisition).unwrap();
    }
}
