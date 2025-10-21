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

use crate::error::Result;
use crate::paths::shared::{ensure_child_directory, ensure_directory};
use std::path::{Path, PathBuf};

pub const JDKS_DIR: &str = "jdks";
pub const CACHE_DIR: &str = "cache";
pub const SHIMS_DIR: &str = "shims";
pub const BIN_DIR: &str = "bin";
pub const LOCKS_DIR: &str = "locks";

pub fn kopi_home_root(kopi_home: &Path) -> PathBuf {
    kopi_home.to_path_buf()
}

pub fn jdks_dir(kopi_home: &Path) -> PathBuf {
    kopi_home.join(JDKS_DIR)
}

pub fn cache_dir(kopi_home: &Path) -> PathBuf {
    kopi_home.join(CACHE_DIR)
}

pub fn shims_dir(kopi_home: &Path) -> PathBuf {
    kopi_home.join(SHIMS_DIR)
}

pub fn bin_dir(kopi_home: &Path) -> PathBuf {
    kopi_home.join(BIN_DIR)
}

pub fn locks_dir(kopi_home: &Path) -> PathBuf {
    kopi_home.join(LOCKS_DIR)
}

pub fn ensure_kopi_home(kopi_home: &Path) -> Result<PathBuf> {
    ensure_directory(kopi_home.to_path_buf())
}

pub fn ensure_jdks_dir(kopi_home: &Path) -> Result<PathBuf> {
    ensure_child_directory(kopi_home, JDKS_DIR)
}

pub fn ensure_cache_dir(kopi_home: &Path) -> Result<PathBuf> {
    ensure_child_directory(kopi_home, CACHE_DIR)
}

pub fn ensure_shims_dir(kopi_home: &Path) -> Result<PathBuf> {
    ensure_child_directory(kopi_home, SHIMS_DIR)
}

pub fn ensure_bin_dir(kopi_home: &Path) -> Result<PathBuf> {
    ensure_child_directory(kopi_home, BIN_DIR)
}

pub fn ensure_locks_dir(kopi_home: &Path) -> Result<PathBuf> {
    ensure_child_directory(kopi_home, LOCKS_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn directory_helpers_join_expected_paths() {
        let home = Path::new("/tmp/kopi");
        assert_eq!(jdks_dir(home), PathBuf::from("/tmp/kopi/jdks"));
        assert_eq!(cache_dir(home), PathBuf::from("/tmp/kopi/cache"));
        assert_eq!(shims_dir(home), PathBuf::from("/tmp/kopi/shims"));
        assert_eq!(bin_dir(home), PathBuf::from("/tmp/kopi/bin"));
        assert_eq!(locks_dir(home), PathBuf::from("/tmp/kopi/locks"));
    }

    #[test]
    fn ensure_helpers_create_directories() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();

        let jdks = ensure_jdks_dir(home).unwrap();
        let cache = ensure_cache_dir(home).unwrap();
        let shims = ensure_shims_dir(home).unwrap();
        let bin = ensure_bin_dir(home).unwrap();
        let locks = ensure_locks_dir(home).unwrap();

        assert!(jdks.exists());
        assert!(cache.exists());
        assert!(shims.exists());
        assert!(bin.exists());
        assert!(locks.exists());

        assert_eq!(jdks, home.join(JDKS_DIR));
        assert_eq!(cache, home.join(CACHE_DIR));
        assert_eq!(shims, home.join(SHIMS_DIR));
        assert_eq!(bin, home.join(BIN_DIR));
        assert_eq!(locks, home.join(LOCKS_DIR));
    }
}
