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

use crate::error::{KopiError, Result};
use crate::locking::scope::LockScope;
use log::{debug, warn};
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Indicates which backend ultimately satisfied the lock request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockBackend {
    Advisory,
    Fallback,
}

/// Handle for an advisory lock backed by `std::fs::File`.
#[derive(Debug)]
pub struct LockHandle {
    scope: LockScope,
    backend_path: PathBuf,
    file: Option<File>,
    acquired_at: Instant,
    released: bool,
}

impl LockHandle {
    pub(crate) fn new(
        scope: LockScope,
        backend_path: PathBuf,
        file: File,
        acquired_at: Instant,
    ) -> Self {
        Self {
            scope,
            backend_path,
            file: Some(file),
            acquired_at,
            released: false,
        }
    }

    pub fn scope(&self) -> &LockScope {
        &self.scope
    }

    pub fn backend(&self) -> LockBackend {
        LockBackend::Advisory
    }

    pub fn path(&self) -> &PathBuf {
        &self.backend_path
    }

    pub fn release(mut self) -> Result<()> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }

        let elapsed = self.acquired_at.elapsed();
        if let Some(file) = self.file.take() {
            if let Err(err) = file.unlock() {
                self.released = true;
                warn!(
                    "Failed to release advisory lock for {} ({}): {err}",
                    self.scope,
                    self.backend_path.display()
                );
                return Err(KopiError::LockingRelease {
                    scope: self.scope.to_string(),
                    details: err.to_string(),
                });
            }
            debug!(
                "Released advisory lock for {} after {:.3}s",
                self.scope,
                duration_to_secs(elapsed)
            );
        }
        self.released = true;
        Ok(())
    }
}

impl Drop for LockHandle {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        if let Some(file) = self.file.take() {
            if let Err(err) = file.unlock() {
                warn!(
                    "Failed to unlock {} during drop: {err}",
                    self.backend_path.display()
                );
            } else {
                debug!(
                    "Released advisory lock for {} on drop after {:.3}s",
                    self.scope,
                    duration_to_secs(self.acquired_at.elapsed())
                );
            }
        }

        self.released = true;
    }
}

/// Handle representing a fallback lock acquisition. The concrete implementation
/// is introduced in Phase 3; for now we simply track scope metadata.
#[derive(Debug)]
pub struct FallbackHandle {
    scope: LockScope,
    backend_path: PathBuf,
    acquired_at: Instant,
    released: bool,
}

impl FallbackHandle {
    pub(crate) fn new(scope: LockScope, backend_path: PathBuf, acquired_at: Instant) -> Self {
        Self {
            scope,
            backend_path,
            acquired_at,
            released: false,
        }
    }

    pub fn scope(&self) -> &LockScope {
        &self.scope
    }

    pub fn backend(&self) -> LockBackend {
        LockBackend::Fallback
    }

    pub fn path(&self) -> &PathBuf {
        &self.backend_path
    }

    pub fn release(mut self) -> Result<()> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }

        debug!(
            "Released fallback lock for {} after {:.3}s",
            self.scope,
            duration_to_secs(self.acquired_at.elapsed())
        );
        self.released = true;
        Ok(())
    }
}

impl Drop for FallbackHandle {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        debug!(
            "Released fallback lock for {} on drop after {:.3}s",
            self.scope,
            duration_to_secs(self.acquired_at.elapsed())
        );
        self.released = true;
    }
}

fn duration_to_secs(duration: Duration) -> f64 {
    duration.as_secs_f64()
}
