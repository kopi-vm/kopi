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

//! Fallback locking based on atomic file creation for network filesystems.
//!
//! The fallback path claims the lock by creating the target file with
//! `create_new`, writes metadata about the lease, and records an adjacent
//! marker file. Hygiene sweeps use these artifacts to distinguish fallback
//! acquisitions from advisory locks and to clean up stale state after crashes.

use crate::error::{KopiError, Result};
use crate::locking::LockAcquisitionRequest;
use crate::locking::handle::FallbackHandle;
use crate::locking::scope::LockScope;
use chrono::{DateTime, Utc};
use log::{debug, warn};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;
use uuid::Uuid;

/// File suffix used for marker files accompanying fallback locks.
pub(crate) const MARKER_SUFFIX: &str = ".marker";
/// Substring used for temporary staging artifacts while attempting acquisition.
pub(crate) const STAGING_SEGMENT: &str = ".staging-";

/// Outcome of a fallback acquisition attempt.
#[derive(Debug)]
pub enum FallbackAcquire {
    Acquired(Box<FallbackHandle>),
    NotAcquired,
}

#[derive(Serialize)]
struct FallbackLeaseMetadata<'a> {
    lease_id: &'a str,
    backend: &'static str,
    pid: u32,
    scope: String,
    created_at: DateTime<Utc>,
}

pub(crate) fn acquire(
    lock_path: PathBuf,
    request: &mut LockAcquisitionRequest<'_>,
) -> Result<FallbackAcquire> {
    let scope = request.scope().clone();
    let scope_label = scope.to_string();

    loop {
        if request.cancellation().is_cancelled() {
            request.notify_cancelled();
            return Err(KopiError::LockingCancelled {
                scope: scope_label.clone(),
                waited_secs: request.elapsed().as_secs_f64(),
            });
        }

        let lease_id = generate_lease_id();
        match attempt_once(&scope, &lock_path, &lease_id) {
            Attempt::Acquired(handle) => {
                debug!(
                    "Acquired fallback lock for {} after {:.3}s",
                    scope_label,
                    request.elapsed().as_secs_f64()
                );
                request.notify_acquired();
                return Ok(FallbackAcquire::Acquired(handle));
            }
            Attempt::Busy => {
                if request.mode().is_non_blocking() {
                    return Ok(FallbackAcquire::NotAcquired);
                }

                request.record_wait_start();
                if request.budget().is_expired() {
                    request.notify_timeout();
                    return Err(KopiError::LockingTimeout {
                        scope: scope_label.clone(),
                        waited_secs: request.elapsed().as_secs_f64(),
                        timeout_value: request.timeout_value(),
                        timeout_source: request.timeout_source(),
                        details: "lock file already exists".to_string(),
                    });
                }

                request.record_retry();
                if let Some(sleep_for) = request.next_sleep_interval() {
                    if request.cancellation().is_cancelled() {
                        request.notify_cancelled();
                        return Err(KopiError::LockingCancelled {
                            scope: scope_label.clone(),
                            waited_secs: request.elapsed().as_secs_f64(),
                        });
                    }
                    thread::sleep(sleep_for);
                    continue;
                }

                request.notify_timeout();
                return Err(KopiError::LockingTimeout {
                    scope: scope_label.clone(),
                    waited_secs: request.elapsed().as_secs_f64(),
                    timeout_value: request.timeout_value(),
                    timeout_source: request.timeout_source(),
                    details: "lock file already exists".to_string(),
                });
            }
            Attempt::IoError(err) => {
                return Err(KopiError::LockingAcquire {
                    scope: scope_label.clone(),
                    details: err.to_string(),
                });
            }
        }
    }
}

enum Attempt {
    Acquired(Box<FallbackHandle>),
    Busy,
    IoError(io::Error),
}

fn attempt_once(scope: &LockScope, lock_path: &Path, lease_id: &str) -> Attempt {
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(mut file) => {
            if let Err(err) = write_lock_metadata(&mut file, scope, lease_id) {
                drop(file);
                cleanup_lock_file(lock_path);
                return Attempt::IoError(err);
            }
            drop(file);

            match finalize_acquisition(scope.clone(), lock_path, lease_id) {
                Ok(handle) => Attempt::Acquired(Box::new(handle)),
                Err(err) => {
                    cleanup_lock_file(lock_path);
                    Attempt::IoError(err)
                }
            }
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Attempt::Busy,
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => Attempt::Busy,
        Err(err) => Attempt::IoError(err),
    }
}

fn write_lock_metadata(file: &mut File, scope: &LockScope, lease_id: &str) -> io::Result<()> {
    let metadata = FallbackLeaseMetadata {
        lease_id,
        backend: "fallback",
        pid: std::process::id(),
        scope: scope.to_string(),
        created_at: Utc::now(),
    };
    let payload =
        serde_json::to_vec_pretty(&metadata).map_err(|err| io::Error::other(err.to_string()))?;

    file.write_all(&payload)?;
    file.sync_all()?;
    Ok(())
}

fn finalize_acquisition(
    scope: LockScope,
    lock_path: &Path,
    lease_id: &str,
) -> io::Result<FallbackHandle> {
    let marker_path = marker_path(lock_path);
    write_marker(&marker_path, lease_id, &scope)?;

    Ok(FallbackHandle::new(
        scope,
        lock_path.to_path_buf(),
        marker_path,
        lease_id.to_string(),
        Instant::now(),
    ))
}

fn write_marker(marker_path: &Path, lease_id: &str, scope: &LockScope) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(marker_path)?;

    let metadata = FallbackLeaseMetadata {
        lease_id,
        backend: "fallback",
        pid: std::process::id(),
        scope: scope.to_string(),
        created_at: Utc::now(),
    };
    let payload =
        serde_json::to_vec_pretty(&metadata).map_err(|err| io::Error::other(err.to_string()))?;
    file.write_all(&payload)?;
    file.sync_all()?;
    Ok(())
}

fn cleanup_lock_file(path: &Path) {
    if let Err(err) = fs::remove_file(path) {
        match err.kind() {
            io::ErrorKind::NotFound => {}
            _ => warn!(
                "Failed to remove fallback lock artifact {}: {err}",
                path.display()
            ),
        }
    }
}

fn marker_path(lock_path: &Path) -> PathBuf {
    append_suffix(lock_path, MARKER_SUFFIX)
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(suffix);
    path.with_file_name(file_name)
}

fn generate_lease_id() -> String {
    format!("{}-{}", std::process::id(), Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::LockScope;
    use crate::locking::PackageKind;
    use crate::locking::acquisition::AcquireMode;
    use crate::locking::package_coordinate::PackageCoordinate;
    use crate::locking::{LockAcquisitionRequest, LockTimeoutValue, PollingBackoff};
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn install_scope() -> LockScope {
        let coordinate = PackageCoordinate::new("temurin", 21, PackageKind::Jdk);
        LockScope::installation(coordinate)
    }

    fn make_request(
        scope: LockScope,
        timeout: Duration,
        mode: AcquireMode,
    ) -> LockAcquisitionRequest<'static> {
        LockAcquisitionRequest::new(scope, LockTimeoutValue::Finite(timeout))
            .with_mode(mode)
            .with_backoff(PollingBackoff::new(
                Duration::from_millis(10),
                2,
                Duration::from_millis(10),
            ))
    }

    #[test]
    fn blocking_acquire_and_release_cleanup_files() {
        let temp = TempDir::new().unwrap();
        let lock_path = temp.path().join("cache.lock");
        let scope = LockScope::CacheWriter;
        let mut request =
            make_request(scope.clone(), Duration::from_secs(1), AcquireMode::Blocking);
        let outcome = acquire(lock_path.clone(), &mut request).unwrap();

        let handle = match outcome {
            FallbackAcquire::Acquired(handle) => handle,
            FallbackAcquire::NotAcquired => panic!("Expected acquisition"),
        };
        let handle = *handle;
        assert!(lock_path.exists());
        assert!(lock_path.with_file_name("cache.lock.marker").exists());

        handle.release().unwrap();
        assert!(!lock_path.exists());
        assert!(!lock_path.with_file_name("cache.lock.marker").exists());
    }

    #[test]
    fn non_blocking_returns_not_acquired_when_busy() {
        let temp = TempDir::new().unwrap();
        let lock_path = temp.path().join("install.lock");
        let scope = install_scope();
        let mut first_request =
            make_request(scope.clone(), Duration::from_secs(1), AcquireMode::Blocking);
        let first = acquire(lock_path.clone(), &mut first_request).unwrap();
        let first_handle = match first {
            FallbackAcquire::Acquired(handle) => handle,
            FallbackAcquire::NotAcquired => panic!("First acquisition should succeed"),
        };
        let first_handle = *first_handle;

        let mut second_request = make_request(
            scope.clone(),
            Duration::from_secs(1),
            AcquireMode::NonBlocking,
        );
        let second = acquire(lock_path.clone(), &mut second_request).unwrap();
        assert!(matches!(second, FallbackAcquire::NotAcquired));

        first_handle.release().unwrap();
    }

    #[test]
    fn blocking_acquire_times_out_when_contended() {
        let temp = TempDir::new().unwrap();
        let lock_path = temp.path().join("global.lock");
        let scope = LockScope::GlobalConfig;
        let mut first_request = make_request(
            scope.clone(),
            Duration::from_millis(100),
            AcquireMode::Blocking,
        );
        let first = acquire(lock_path.clone(), &mut first_request).unwrap();
        let first_handle = match first {
            FallbackAcquire::Acquired(handle) => handle,
            FallbackAcquire::NotAcquired => panic!("Expected acquisition"),
        };
        let first_handle = *first_handle;

        let mut timeout_request = make_request(
            scope.clone(),
            Duration::from_millis(100),
            AcquireMode::Blocking,
        );
        let err = acquire(lock_path.clone(), &mut timeout_request).unwrap_err();
        match err {
            KopiError::LockingTimeout { scope: label, .. } => {
                assert!(label.contains("global"));
            }
            other => panic!("Expected timeout error, got {other:?}"),
        }

        first_handle.release().unwrap();
        assert!(!lock_path.exists());
        assert!(fs::read_dir(temp.path()).unwrap().count() <= 1);
    }
}
