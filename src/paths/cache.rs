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
use crate::paths::home;
use crate::paths::shared::ensure_nested_directory;
use std::path::{Path, PathBuf};

pub const METADATA_FILE: &str = "metadata.json";
pub const TEMP_DIR: &str = "tmp";

pub fn cache_root(kopi_home: &Path) -> PathBuf {
    home::cache_dir(kopi_home)
}

pub fn ensure_cache_root(kopi_home: &Path) -> Result<PathBuf> {
    home::ensure_cache_dir(kopi_home)
}

pub fn metadata_cache_file(kopi_home: &Path) -> PathBuf {
    cache_root(kopi_home).join(METADATA_FILE)
}

pub fn temp_cache_directory(kopi_home: &Path) -> PathBuf {
    cache_root(kopi_home).join(TEMP_DIR)
}

pub fn ensure_temp_cache_directory(kopi_home: &Path) -> Result<PathBuf> {
    ensure_nested_directory(kopi_home, [home::CACHE_DIR, TEMP_DIR])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cache_paths_align_with_existing_layout() {
        let home = Path::new("/opt/kopi");

        assert_eq!(cache_root(home), PathBuf::from("/opt/kopi/cache"));
        assert_eq!(
            metadata_cache_file(home),
            PathBuf::from("/opt/kopi/cache/metadata.json")
        );
        assert_eq!(
            temp_cache_directory(home),
            PathBuf::from("/opt/kopi/cache/tmp")
        );
    }

    #[test]
    fn ensure_directories_create_expected_structure() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();

        let cache = ensure_cache_root(home).unwrap();
        let tmp = ensure_temp_cache_directory(home).unwrap();

        assert!(cache.exists());
        assert!(tmp.exists());
        assert_eq!(cache, home.join("cache"));
        assert_eq!(tmp, home.join("cache").join(TEMP_DIR));
    }
}
