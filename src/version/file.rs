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
use crate::models::package::PackageType;
use crate::version::format_version_minimal;
use crate::version::parser::ParsedVersionRequest;
use log::debug;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Write a version file atomically to the specified path
pub fn write_version_file(path: &PathBuf, version_request: &ParsedVersionRequest) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            KopiError::SystemError(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Format version string - use minimal representation
    let version = version_request.version.as_ref().unwrap();
    let version_str = format_version_minimal(version);

    // Build the version string
    let mut parts = Vec::new();

    // Add package type prefix only if it's JRE (JDK is the default)
    if let Some(package_type) = &version_request.package_type
        && *package_type == PackageType::Jre
    {
        parts.push("jre".to_string());
    }

    // Add distribution if present
    if let Some(dist) = &version_request.distribution {
        parts.push(dist.id().to_string());
    }

    // Add version
    parts.push(version_str);

    // Join with @ separator
    let version_string = parts.join("@");

    // Write atomically using a temporary file
    let temp_path = path.with_extension("tmp");

    {
        let mut file = fs::File::create(&temp_path).map_err(|e| {
            KopiError::SystemError(format!("Failed to create {}: {}", temp_path.display(), e))
        })?;

        file.write_all(version_string.as_bytes()).map_err(|e| {
            KopiError::SystemError(format!("Failed to write to {}: {}", temp_path.display(), e))
        })?;

        file.flush().map_err(|e| {
            KopiError::SystemError(format!("Failed to flush {}: {}", temp_path.display(), e))
        })?;
    }

    // Rename temp file to final location
    fs::rename(&temp_path, path).map_err(|e| {
        KopiError::SystemError(format!(
            "Failed to rename {} to {}: {}",
            temp_path.display(),
            path.display(),
            e
        ))
    })?;

    debug!("Wrote version file: {path:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::distribution::Distribution;
    use crate::version::Version;
    use tempfile::TempDir;

    #[test]
    fn test_write_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join("version");

        // Test with distribution and default JDK package type
        let version_request = ParsedVersionRequest {
            distribution: Some(Distribution::Temurin),
            version: Some(Version::new(21, 0, 0)),
            package_type: Some(PackageType::Jdk),
            latest: false,
        };

        write_version_file(&version_file, &version_request).unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "temurin@21"); // JDK is omitted

        // Test without distribution
        let version_request2 = ParsedVersionRequest {
            distribution: None,
            version: Some(Version::new(17, 0, 0)),
            package_type: Some(PackageType::Jdk),
            latest: false,
        };

        write_version_file(&version_file, &version_request2).unwrap();

        let content2 = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "17"); // JDK is omitted
    }

    #[test]
    fn test_write_version_file_with_full_version() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join("version");

        let version_request = ParsedVersionRequest {
            distribution: Some(Distribution::Corretto),
            version: Some(Version::new(11, 0, 21)),
            package_type: Some(PackageType::Jdk),
            latest: false,
        };

        write_version_file(&version_file, &version_request).unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "corretto@11.0.21");
    }

    #[test]
    fn test_write_version_file_creates_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("subdir").join("version");

        let version_request = ParsedVersionRequest {
            distribution: None,
            version: Some(Version::new(21, 0, 0)),
            package_type: Some(PackageType::Jdk),
            latest: false,
        };

        write_version_file(&nested_path, &version_request).unwrap();

        assert!(nested_path.exists());
        let content = fs::read_to_string(&nested_path).unwrap();
        assert_eq!(content, "21");
    }

    #[test]
    fn test_write_version_file_with_jre() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join("version");

        // Test JRE with distribution
        let version_request = ParsedVersionRequest {
            distribution: Some(Distribution::Temurin),
            version: Some(Version::new(21, 0, 0)),
            package_type: Some(PackageType::Jre),
            latest: false,
        };

        write_version_file(&version_file, &version_request).unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "jre@temurin@21");

        // Test JRE without distribution
        let version_request2 = ParsedVersionRequest {
            distribution: None,
            version: Some(Version::new(17, 0, 0)),
            package_type: Some(PackageType::Jre),
            latest: false,
        };

        write_version_file(&version_file, &version_request2).unwrap();

        let content2 = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "jre@17");
    }
}
