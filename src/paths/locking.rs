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

use crate::paths::home;
use crate::paths::shared::sanitize_segment;
use std::path::{Path, PathBuf};

const INSTALL_DIR: &str = "install";
const CACHE_LOCK_FILE: &str = "cache.lock";

pub fn locks_root(kopi_home: &Path) -> PathBuf {
    home::locks_dir(kopi_home)
}

pub fn install_lock_directory(kopi_home: &Path, distribution: &str) -> PathBuf {
    let normalized = sanitize_segment(distribution).unwrap_or_else(|| "default".to_string());
    locks_root(kopi_home).join(INSTALL_DIR).join(normalized)
}

pub fn install_lock_path(kopi_home: &Path, distribution: &str, slug: &str) -> PathBuf {
    let file_name = format!("{slug}.lock");
    install_lock_directory(kopi_home, distribution).join(file_name)
}

pub fn cache_lock_path(kopi_home: &Path) -> PathBuf {
    locks_root(kopi_home).join(CACHE_LOCK_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locking::{PackageCoordinate, PackageKind};

    #[test]
    fn locks_root_joins_directory() {
        let home = Path::new("/tmp/kopi");
        assert_eq!(locks_root(home), Path::new("/tmp/kopi/locks"));
    }

    #[test]
    fn install_lock_directory_sanitizes_distribution() {
        let home = Path::new("/tmp/kopi");
        let dir = install_lock_directory(home, "Temurin FX");
        assert_eq!(dir, Path::new("/tmp/kopi/locks/install/temurin-fx"));
    }

    #[test]
    fn install_lock_path_uses_coordinate_slug() {
        let home = Path::new("/tmp/kopi");
        let coordinate = PackageCoordinate::new("Temurin", 21, PackageKind::Jdk)
            .with_architecture(Some("x64"))
            .with_javafx(true);
        let expected = Path::new("/tmp/kopi/locks/install/temurin/temurin-21-jdk-x64-javafx.lock");
        let actual = install_lock_path(home, coordinate.distribution(), coordinate.slug().as_ref());
        assert_eq!(actual, expected);
    }

    #[test]
    fn cache_lock_path_is_deterministic() {
        let home = Path::new("/tmp/kopi");
        assert_eq!(
            cache_lock_path(home),
            Path::new("/tmp/kopi/locks/cache.lock")
        );
    }
}
