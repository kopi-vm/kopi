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

pub const TEMP_STAGING_DIR: &str = ".tmp";
pub const BUNDLE_CONTENTS_DIR: &str = "Contents";
pub const BUNDLE_JAVA_HOME_DIR: &str = "Home";
pub const BUNDLE_JAVA_HOME_SUFFIX: &str = "Contents/Home";

pub fn installations_root(kopi_home: &Path) -> PathBuf {
    home::jdks_dir(kopi_home)
}

pub fn ensure_installations_root(kopi_home: &Path) -> Result<PathBuf> {
    home::ensure_jdks_dir(kopi_home)
}

pub fn installation_directory<S: AsRef<str>>(kopi_home: &Path, slug: S) -> PathBuf {
    installations_root(kopi_home).join(slug.as_ref())
}

pub fn metadata_file<S: AsRef<str>>(kopi_home: &Path, slug: S) -> PathBuf {
    installations_root(kopi_home).join(format!("{}.meta.json", slug.as_ref()))
}

pub fn temp_staging_directory(kopi_home: &Path) -> PathBuf {
    installations_root(kopi_home).join(TEMP_STAGING_DIR)
}

pub fn ensure_temp_staging_directory(kopi_home: &Path) -> Result<PathBuf> {
    ensure_nested_directory(kopi_home, [home::JDKS_DIR, TEMP_STAGING_DIR])
}

pub fn bin_directory(java_home: &Path) -> PathBuf {
    java_home.join(home::BIN_DIR)
}

pub fn bundle_contents_directory(jdk_root: &Path) -> PathBuf {
    jdk_root.join(BUNDLE_CONTENTS_DIR)
}

pub fn bundle_java_home(jdk_root: &Path) -> PathBuf {
    bundle_contents_directory(jdk_root).join(BUNDLE_JAVA_HOME_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn installation_paths_follow_previous_layout() {
        let home = Path::new("/opt/kopi");
        let slug = "temurin-21-jdk-x64";
        let jdk_root = Path::new("/opt/kopi/jdks/temurin-21-jdk-x64");

        assert_eq!(installations_root(home), PathBuf::from("/opt/kopi/jdks"));
        assert_eq!(
            installation_directory(home, slug),
            PathBuf::from("/opt/kopi/jdks/temurin-21-jdk-x64")
        );
        assert_eq!(
            metadata_file(home, slug),
            PathBuf::from("/opt/kopi/jdks/temurin-21-jdk-x64.meta.json")
        );
        assert_eq!(
            temp_staging_directory(home),
            PathBuf::from("/opt/kopi/jdks/.tmp")
        );
        assert_eq!(
            bin_directory(jdk_root),
            PathBuf::from("/opt/kopi/jdks/temurin-21-jdk-x64/bin")
        );
        assert_eq!(
            bundle_contents_directory(jdk_root),
            PathBuf::from("/opt/kopi/jdks/temurin-21-jdk-x64/Contents")
        );
        assert_eq!(
            bundle_java_home(jdk_root),
            PathBuf::from("/opt/kopi/jdks/temurin-21-jdk-x64/Contents/Home")
        );
    }

    #[test]
    fn ensure_helpers_create_directories() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();

        let installs = ensure_installations_root(home).unwrap();
        let temp_dir = ensure_temp_staging_directory(home).unwrap();

        assert!(installs.exists());
        assert!(temp_dir.exists());
        assert_eq!(installs, home.join("jdks"));
        assert_eq!(temp_dir, home.join("jdks").join(TEMP_STAGING_DIR));
    }
}
