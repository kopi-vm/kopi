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

/// Shared test fixtures for creating test JDK instances in unit tests
use crate::storage::InstalledJdk;
use crate::version::Version;
use std::path::PathBuf;
use std::str::FromStr;

/// Creates a test JDK with automatic path generation
///
/// # Arguments
/// * `distribution` - The distribution name (e.g., "temurin", "corretto")
/// * `version` - The version string (e.g., "21.0.5+11", "17.0.9+9")
///
/// # Returns
/// An InstalledJdk instance with a generated path under `/test/jdks/`
pub fn create_test_jdk(distribution: &str, version: &str) -> InstalledJdk {
    InstalledJdk::new(
        distribution.to_string(),
        Version::from_str(version).unwrap(),
        PathBuf::from(format!("/test/jdks/{distribution}-{version}")),
        false,
    )
}

/// Creates a test JDK with a custom path
///
/// # Arguments
/// * `distribution` - The distribution name (e.g., "temurin", "corretto")
/// * `version` - The version string (e.g., "21.0.5+11", "17.0.9+9")
/// * `path` - The custom path for the JDK installation
///
/// # Returns
/// An InstalledJdk instance with the specified path
pub fn create_test_jdk_with_path(distribution: &str, version: &str, path: &str) -> InstalledJdk {
    InstalledJdk::new(
        distribution.to_string(),
        Version::from_str(version).unwrap(),
        PathBuf::from(path),
        false,
    )
}

/// Creates multiple test JDKs with different distributions and versions
///
/// # Returns
/// A vector of InstalledJdk instances representing a typical test setup
pub fn create_test_jdk_collection() -> Vec<InstalledJdk> {
    vec![
        create_test_jdk("temurin", "21.0.5+11"),
        create_test_jdk("temurin", "17.0.9+9"),
        create_test_jdk("corretto", "21.0.1"),
        create_test_jdk("corretto", "17.0.5"),
        create_test_jdk("zulu", "11.0.25"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_jdk() {
        let jdk = create_test_jdk("temurin", "21.0.5+11");
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "21.0.5+11");
        assert_eq!(jdk.path, PathBuf::from("/test/jdks/temurin-21.0.5+11"));
    }

    #[test]
    fn test_create_test_jdk_with_path() {
        let jdk = create_test_jdk_with_path("corretto", "17.0.9", "/custom/path");
        assert_eq!(jdk.distribution, "corretto");
        assert_eq!(jdk.version.to_string(), "17.0.9");
        assert_eq!(jdk.path, PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_create_test_jdk_collection() {
        let jdks = create_test_jdk_collection();
        assert_eq!(jdks.len(), 5);
        assert!(jdks.iter().any(|jdk| jdk.distribution == "temurin"));
        assert!(jdks.iter().any(|jdk| jdk.distribution == "corretto"));
        assert!(jdks.iter().any(|jdk| jdk.distribution == "zulu"));
    }
}
