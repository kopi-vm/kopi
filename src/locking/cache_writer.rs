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

use crate::config::KopiConfig;
use crate::error::Result;
use crate::indicator::{ProgressIndicator, StatusReporter};
use crate::locking::{LockAcquisition, LockBackend, LockController, LockScope};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// RAII guard that serialises cache writers through the locking controller.
pub struct CacheWriterLockGuard {
    acquisition: Option<LockAcquisition>,
    backend: LockBackend,
    waited: Duration,
}

impl CacheWriterLockGuard {
    fn new(acquisition: LockAcquisition, waited: Duration) -> Self {
        let backend = acquisition.backend();
        Self {
            acquisition: Some(acquisition),
            backend,
            waited,
        }
    }

    /// Acquire the cache writer lock using progress feedback.
    pub fn acquire_with_feedback(
        config: &KopiConfig,
        indicator: Arc<Mutex<Box<dyn ProgressIndicator>>>,
    ) -> Result<Self> {
        let controller = LockController::with_default_inspector(
            config.kopi_home().to_path_buf(),
            &config.locking,
        );
        let started_at = Instant::now();
        let acquisition = controller.acquire_with_feedback(LockScope::CacheWriter, indicator)?;
        Ok(Self::new(acquisition, started_at.elapsed()))
    }

    /// Acquire the cache writer lock using status reporter feedback.
    pub fn acquire_with_status_reporter(
        config: &KopiConfig,
        reporter: &StatusReporter,
    ) -> Result<Self> {
        let controller = LockController::with_default_inspector(
            config.kopi_home().to_path_buf(),
            &config.locking,
        );
        let started_at = Instant::now();
        let acquisition = controller.acquire_with_status_sink(LockScope::CacheWriter, reporter)?;
        Ok(Self::new(acquisition, started_at.elapsed()))
    }

    /// Returns the backend that ultimately satisfied this lock request.
    pub fn backend(&self) -> LockBackend {
        self.backend
    }

    /// Returns the time spent waiting for the lock.
    pub fn waited(&self) -> Duration {
        self.waited
    }
}

impl Drop for CacheWriterLockGuard {
    fn drop(&mut self) {
        if let Some(acquisition) = self.acquisition.take()
            && let Err(err) = acquisition.release()
        {
            log::warn!("Failed to release cache writer lock: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::{SilentProgress, StatusReporter};
    use crate::locking::LockBackend;
    use tempfile::TempDir;

    #[test]
    fn feedback_acquisition_uses_advisory_backend() {
        let temp_home = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
        let indicator = Arc::new(Mutex::new(
            Box::new(SilentProgress::new()) as Box<dyn ProgressIndicator>
        ));

        let guard =
            CacheWriterLockGuard::acquire_with_feedback(&config, indicator.clone()).unwrap();
        assert_eq!(guard.backend(), LockBackend::Advisory);
        assert!(guard.waited() >= Duration::ZERO);
        drop(guard);

        let reacquired =
            CacheWriterLockGuard::acquire_with_feedback(&config, indicator.clone()).unwrap();
        assert_eq!(reacquired.backend(), LockBackend::Advisory);
    }

    #[test]
    fn status_reporter_acquisition_releases_on_drop() {
        let temp_home = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
        let reporter = StatusReporter::new(true);

        let guard = CacheWriterLockGuard::acquire_with_status_reporter(&config, &reporter).unwrap();
        assert_eq!(guard.backend(), LockBackend::Advisory);
        drop(guard);

        // Ensure the lock can be reacquired after drop.
        let reporter = StatusReporter::new(true);
        assert!(CacheWriterLockGuard::acquire_with_status_reporter(&config, &reporter).is_ok());
    }
}
