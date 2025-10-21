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
use crate::platform::{shim_binary_name, with_executable_extension};
use std::path::{Path, PathBuf};

pub fn shims_root(kopi_home: &Path) -> PathBuf {
    home::shims_dir(kopi_home)
}

pub fn ensure_shims_root(kopi_home: &Path) -> Result<PathBuf> {
    home::ensure_shims_dir(kopi_home)
}

pub fn shim_launcher_path(kopi_home: &Path) -> PathBuf {
    shims_root(kopi_home).join(shim_binary_name())
}

pub fn tool_shim_path(kopi_home: &Path, tool_name: &str) -> PathBuf {
    shims_root(kopi_home).join(with_executable_extension(tool_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn shim_paths_match_platform_expectations() {
        let home = Path::new("/opt/kopi");
        assert_eq!(shims_root(home), PathBuf::from("/opt/kopi/shims"));
        assert_eq!(
            shim_launcher_path(home),
            PathBuf::from("/opt/kopi/shims").join(shim_binary_name())
        );
        assert_eq!(
            tool_shim_path(home, "java"),
            PathBuf::from("/opt/kopi/shims").join(with_executable_extension("java"))
        );
    }

    #[test]
    fn ensure_shims_root_creates_directory() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();
        let shims = ensure_shims_root(home).unwrap();

        assert!(shims.exists());
        assert_eq!(shims, home.join("shims"));
    }
}
