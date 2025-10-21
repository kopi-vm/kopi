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

use crate::locking::package_coordinate::PackageCoordinate;
use crate::paths::locking::{cache_lock_path, install_lock_path, locks_root};
use std::fmt;
use std::path::{Path, PathBuf};

/// Indicates whether a lock should allow concurrent readers or enforce exclusivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    Shared,
    Exclusive,
}

/// Describes the scope of a lock to be acquired through the locking controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockScope {
    Installation { coordinate: PackageCoordinate },
    CacheWriter,
    GlobalConfig,
}

impl LockScope {
    pub fn installation(coordinate: PackageCoordinate) -> Self {
        Self::Installation { coordinate }
    }

    /// Returns the on-disk path for this scope relative to the Kopi home directory.
    pub fn lock_path(&self, kopi_home: &Path) -> PathBuf {
        match self {
            LockScope::Installation { coordinate } => install_lock_path(
                kopi_home,
                coordinate.distribution(),
                coordinate.slug().as_ref(),
            ),
            LockScope::CacheWriter => cache_lock_path(kopi_home),
            LockScope::GlobalConfig => locks_root(kopi_home).join("config.lock"),
        }
    }

    /// Indicates whether the scope should use a shared or exclusive advisory lock.
    pub fn lock_kind(&self) -> LockKind {
        match self {
            LockScope::Installation { .. } | LockScope::CacheWriter | LockScope::GlobalConfig => {
                LockKind::Exclusive
            }
        }
    }

    /// Human-readable label used for logging and error reporting.
    pub fn label(&self) -> String {
        match self {
            LockScope::Installation { coordinate } => {
                format!("installation {}", coordinate.slug())
            }
            LockScope::CacheWriter => "cache writer".to_string(),
            LockScope::GlobalConfig => "global configuration".to_string(),
        }
    }
}

impl fmt::Display for LockScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::PackageKind;
    use std::path::Path;

    #[test]
    fn installation_scope_uses_slugged_path() {
        let coordinate =
            PackageCoordinate::new("Temurin", 21, PackageKind::Jdk).with_architecture(Some("x64"));
        let home = Path::new("/tmp/kopi");
        let path = LockScope::installation(coordinate).lock_path(home);
        assert_eq!(
            path,
            Path::new("/tmp/kopi/locks/install/temurin/temurin-21-jdk-x64.lock")
        );
    }

    #[test]
    fn cache_scope_uses_shared_path() {
        let home = Path::new("/tmp/kopi");
        let path = LockScope::CacheWriter.lock_path(home);
        assert_eq!(path, Path::new("/tmp/kopi/locks/cache.lock"));
    }

    #[test]
    fn labels_are_human_readable() {
        let coordinate = PackageCoordinate::new("Temurin", 21, PackageKind::Jdk);
        let install_scope = LockScope::installation(coordinate);
        assert!(install_scope.label().contains("installation"));
        assert_eq!(LockScope::CacheWriter.label(), "cache writer");
        assert_eq!(LockScope::GlobalConfig.label(), "global configuration");
    }
}
