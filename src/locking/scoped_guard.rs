use crate::error::Result;
use crate::locking::{LockAcquisition, LockBackend, LockController};
use log::warn;

/// RAII guard that releases a package-scoped lock when dropped.
pub struct ScopedPackageLockGuard<'a> {
    controller: &'a LockController,
    acquisition: Option<LockAcquisition>,
    backend: LockBackend,
    scope_label: String,
}

impl<'a> ScopedPackageLockGuard<'a> {
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

impl Drop for ScopedPackageLockGuard<'_> {
    fn drop(&mut self) {
        if let Some(acquisition) = self.acquisition.take()
            && let Err(err) = self.controller.release(acquisition)
        {
            warn!(
                "Failed to release package lock for {}: {err}",
                self.scope_label
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LockingConfig;
    use crate::locking::{LockBackend, LockController, LockKind, LockScope};
    use tempfile::TempDir;

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
            let guard = ScopedPackageLockGuard::new(&controller, acquisition);
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
        let guard = ScopedPackageLockGuard::new(&controller, acquisition);
        guard.release().unwrap();
        let reacquired = controller.acquire(scope).unwrap();
        drop(reacquired);
    }
}
