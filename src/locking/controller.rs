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
use crate::locking::handle::{FallbackHandle, LockBackend, LockHandle};
use crate::locking::scope::{LockKind, LockScope};
use crate::platform::{AdvisorySupport, DefaultFilesystemInspector, FilesystemInspector};
use log::{debug, info};
use std::fs::TryLockError;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireMode {
    Blocking,
    NonBlocking,
}

/// Result of a lock attempt when considering fallback downgraded paths.
#[derive(Debug)]
pub enum LockAcquisition {
    Advisory(LockHandle),
    Fallback(FallbackHandle),
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
            LockAcquisition::Fallback(handle) => handle.release(),
        }
    }
}

/// Coordinates advisory locking and fallback behaviour across filesystems.
pub struct LockController {
    kopi_home: PathBuf,
    inspector: Arc<dyn FilesystemInspector>,
    preferred_mode: LockingMode,
    timeout: Duration,
    retry_delay: Duration,
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
        let retry_delay = Duration::from_millis(50);
        Self {
            kopi_home: kopi_home.into(),
            inspector,
            preferred_mode: config.mode,
            timeout: config.timeout(),
            retry_delay,
        }
    }

    pub fn acquire(&self, scope: LockScope) -> Result<LockAcquisition> {
        match self.acquire_with_mode(scope, AcquireMode::Blocking)? {
            AcquireDisposition::Acquired(acquired) => Ok(acquired),
            AcquireDisposition::NotAcquired => Err(KopiError::LockingAcquire {
                scope: "unknown".to_string(),
                details: "Lock acquisition unexpectedly returned without handle".to_string(),
            }),
        }
    }

    pub fn try_acquire(&self, scope: LockScope) -> Result<Option<LockAcquisition>> {
        match self.acquire_with_mode(scope, AcquireMode::NonBlocking)? {
            AcquireDisposition::Acquired(acquired) => Ok(Some(acquired)),
            AcquireDisposition::NotAcquired => Ok(None),
        }
    }

    pub fn release(&self, acquisition: LockAcquisition) -> Result<()> {
        acquisition.release()
    }

    fn acquire_with_mode(&self, scope: LockScope, mode: AcquireMode) -> Result<AcquireDisposition> {
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

        match support {
            LockBackend::Fallback => self.acquire_fallback(scope, lock_path),
            LockBackend::Advisory => self.acquire_advisory(scope, lock_path, mode),
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
        scope: LockScope,
        lock_path: PathBuf,
        mode: AcquireMode,
    ) -> Result<AcquireDisposition> {
        let is_shared = matches!(scope.lock_kind(), LockKind::Shared);
        let file = self
            .prepare_lock_file(&lock_path)
            .map_err(|err| KopiError::LockingAcquire {
                scope: scope.to_string(),
                details: format!("Failed to open lock file {}: {err}", lock_path.display()),
            })?;

        let acquisition_start = Instant::now();
        let mut last_detail: Option<String> = None;

        loop {
            match self.try_lock(&file, is_shared) {
                Ok(()) => {
                    debug!(
                        "Acquired advisory lock for {} after {:.3}s",
                        scope,
                        acquisition_start.elapsed().as_secs_f64()
                    );
                    let handle = LockHandle::new(scope, lock_path, file, acquisition_start);
                    return Ok(AcquireDisposition::Acquired(LockAcquisition::Advisory(
                        handle,
                    )));
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    let err_message = err.to_string();
                    if matches!(mode, AcquireMode::NonBlocking) {
                        return Ok(AcquireDisposition::NotAcquired);
                    }

                    if acquisition_start.elapsed() >= self.timeout {
                        let waited = acquisition_start.elapsed();
                        let detail = last_detail.take().unwrap_or_else(|| err_message.clone());
                        return Err(KopiError::LockingTimeout {
                            scope: scope.to_string(),
                            waited_secs: waited.as_secs_f64(),
                            details: detail,
                        });
                    }

                    last_detail = Some(err_message);
                    thread::sleep(self.retry_delay);
                    continue;
                }
                Err(err) if err.kind() == io::ErrorKind::Unsupported => {
                    info!(
                        "Advisory locking unsupported for {} at {}; downgrading to fallback",
                        scope,
                        lock_path.display()
                    );
                    drop(file);
                    return self.acquire_fallback(scope, lock_path);
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

    fn acquire_fallback(&self, scope: LockScope, lock_path: PathBuf) -> Result<AcquireDisposition> {
        info!(
            "Acquiring fallback lock for {} at {}",
            scope,
            lock_path.display()
        );
        let acquired_at = Instant::now();
        let handle = FallbackHandle::new(scope, lock_path, acquired_at);
        Ok(AcquireDisposition::Acquired(LockAcquisition::Fallback(
            handle,
        )))
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
    use crate::locking::{LockKind, LockScope, PackageKind};
    use crate::platform::{FilesystemInfo, FilesystemInspector, FilesystemKind};
    use std::sync::Mutex;
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
        let config = LockingConfig {
            timeout_secs: 1,
            ..LockingConfig::default()
        };
        let controller = LockController::new(
            temp.path().to_path_buf(),
            &config,
            Arc::new(TestInspector::new(vec![native_fs(), native_fs()])),
        );
        let scope =
            LockScope::installation(PackageCoordinate::new("temurin", 21, PackageKind::Jdk));

        let first = controller.acquire(scope.clone()).unwrap();
        let err = controller.acquire(scope.clone()).unwrap_err();
        match err {
            KopiError::LockingTimeout { scope: s, .. } => {
                assert!(s.contains("installation"));
            }
            other => panic!("Expected timeout error, got {other:?}"),
        }
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
    fn forced_fallback_mode_bypasses_inspector() {
        let temp = TempDir::new().unwrap();
        let config = LockingConfig {
            mode: LockingMode::Fallback,
            ..LockingConfig::default()
        };
        let controller = LockController::with_default_inspector(temp.path().to_path_buf(), &config);
        let scope = LockScope::CacheWriter;

        let acquisition = controller.acquire(scope.clone()).unwrap();
        assert_eq!(acquisition.backend(), LockBackend::Fallback);
        controller.release(acquisition).unwrap();
    }

    #[test]
    fn forced_advisory_mode_attempts_lock() {
        let temp = TempDir::new().unwrap();
        let config = LockingConfig {
            mode: LockingMode::Advisory,
            ..LockingConfig::default()
        };
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
