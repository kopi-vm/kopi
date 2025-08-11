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
use crate::version::Version;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct InstalledJdk {
    pub distribution: String,
    pub version: Version,
    pub path: PathBuf,
}

impl InstalledJdk {
    pub fn write_to(&self, path: &Path) -> Result<()> {
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

        // Format version string
        let version_string = format!("{}@{}", self.distribution, self.version);

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

        log::debug!("Wrote version file: {path:?}");
        Ok(())
    }

    /// Resolves the correct JAVA_HOME path for this JDK installation.
    ///
    /// On macOS, this handles different directory structures:
    /// - Bundle structure: Returns path/Contents/Home
    /// - Direct structure: Returns path directly
    ///
    /// On other platforms, always returns the path directly.
    pub fn resolve_java_home(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            // Check for bundle structure (Contents/Home)
            let bundle_path = self.path.join("Contents").join("Home");
            if bundle_path.join("bin").exists() {
                log::debug!(
                    "Resolved JAVA_HOME for {} using bundle structure: {}",
                    self.distribution,
                    bundle_path.display()
                );
                return bundle_path;
            }

            // Direct structure or hybrid (has bin at root)
            if self.path.join("bin").exists() {
                log::debug!(
                    "Resolved JAVA_HOME for {} using direct structure: {}",
                    self.distribution,
                    self.path.display()
                );
                return self.path.clone();
            }

            // Fallback: return path as-is and log warning
            log::warn!(
                "Could not detect JDK structure for {} at {}, using path as-is",
                self.distribution,
                self.path.display()
            );
            self.path.clone()
        }

        #[cfg(not(target_os = "macos"))]
        {
            // On non-macOS platforms, always use direct structure
            log::debug!(
                "Resolved JAVA_HOME for {} on non-macOS platform: {}",
                self.distribution,
                self.path.display()
            );
            self.path.clone()
        }
    }

    /// Resolves the path to the bin directory for this JDK installation.
    ///
    /// This method uses resolve_java_home() and appends "bin" to get the
    /// correct bin directory path regardless of the JDK structure.
    pub fn resolve_bin_path(&self) -> Result<PathBuf> {
        let java_home = self.resolve_java_home();
        let bin_path = java_home.join("bin");

        if !bin_path.exists() {
            return Err(KopiError::SystemError(format!(
                "JDK bin directory not found at expected location: {}",
                bin_path.display()
            )));
        }

        log::debug!(
            "Resolved bin path for {}: {}",
            self.distribution,
            bin_path.display()
        );

        Ok(bin_path)
    }
}

pub struct JdkLister;

impl JdkLister {
    pub fn list_installed_jdks(jdks_dir: &Path) -> Result<Vec<InstalledJdk>> {
        if !jdks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();

        for entry in fs::read_dir(jdks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(jdk_info) = Self::parse_jdk_dir_name(&path) {
                installed.push(jdk_info);
            }
        }

        installed.sort_by(|a, b| {
            a.distribution
                .cmp(&b.distribution)
                .then(b.version.cmp(&a.version))
        });

        Ok(installed)
    }

    pub fn parse_jdk_dir_name(path: &Path) -> Option<InstalledJdk> {
        let file_name = path.file_name()?.to_str()?;

        let mut split_pos = None;
        let chars: Vec<char> = file_name.chars().collect();

        for i in 0..chars.len() - 1 {
            if chars[i] == '-' && chars[i + 1].is_numeric() {
                split_pos = Some(i);
                break;
            }
        }

        let (distribution, version) = if let Some(pos) = split_pos {
            let dist = &file_name[..pos];
            let ver = &file_name[pos + 1..];
            (dist, ver)
        } else {
            return None;
        };

        let parsed_version = match Version::from_str(version) {
            Ok(v) => v,
            Err(_) => return None,
        };

        Some(InstalledJdk {
            distribution: distribution.to_string(),
            version: parsed_version,
            path: path.to_path_buf(),
        })
    }

    pub fn get_jdk_size(path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_installed_jdks() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = temp_dir.path().join("jdks");
        fs::create_dir_all(&jdks_dir).unwrap();

        fs::create_dir_all(jdks_dir.join("temurin-21.0.1")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-17.0.9")).unwrap();
        fs::create_dir_all(jdks_dir.join(".tmp")).unwrap();

        let installed = JdkLister::list_installed_jdks(&jdks_dir).unwrap();
        assert_eq!(installed.len(), 2);

        assert_eq!(installed[0].distribution, "corretto");
        assert_eq!(installed[0].version.to_string(), "17.0.9");

        assert_eq!(installed[1].distribution, "temurin");
        assert_eq!(installed[1].version.to_string(), "21.0.1");
    }

    #[test]
    fn test_parse_jdk_dir_name() {
        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-21.0.1")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "21.0.1");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-22-ea")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "22-ea");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("corretto-17.0.9+9")).unwrap();
        assert_eq!(jdk.distribution, "corretto");
        assert_eq!(jdk.version.to_string(), "17.0.9+9");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("graalvm-ce-21.0.1")).unwrap();
        assert_eq!(jdk.distribution, "graalvm-ce");
        assert_eq!(jdk.version.to_string(), "21.0.1");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("liberica-21.0.1-13")).unwrap();
        assert_eq!(jdk.distribution, "liberica");
        assert_eq!(jdk.version.to_string(), "21.0.1-13");

        let jdk = JdkLister::parse_jdk_dir_name(Path::new("temurin-17")).unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "17");

        assert!(JdkLister::parse_jdk_dir_name(Path::new("invalid")).is_none());
        assert!(JdkLister::parse_jdk_dir_name(Path::new("no-hyphen-here")).is_none());
        assert!(JdkLister::parse_jdk_dir_name(Path::new("temurin")).is_none());

        // Version with 'v' prefix should not be parsed
        assert!(JdkLister::parse_jdk_dir_name(Path::new("zulu-v11.0.21")).is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_bundle_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Create bundle structure
        let bundle_bin_path = jdk_path.join("Contents").join("Home").join("bin");
        fs::create_dir_all(&bundle_bin_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path.join("Contents").join("Home"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_direct_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("liberica-21.0.1");

        // Create direct structure
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "liberica".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_hybrid_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("zulu-21.0.1");

        // Create hybrid structure (bin at root + Contents/Home exists)
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        fs::create_dir_all(jdk_path.join("Contents").join("Home").join("bin")).unwrap();

        let jdk = InstalledJdk {
            distribution: "zulu".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        // Should prefer bundle structure when both exist
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path.join("Contents").join("Home"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_java_home_missing_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("broken-jdk");
        fs::create_dir_all(&jdk_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "broken".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        // Should return path as-is when structure cannot be detected
        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_resolve_java_home_non_macos() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Even if bundle structure exists, should return direct path on non-macOS
        fs::create_dir_all(jdk_path.join("Contents").join("Home").join("bin")).unwrap();
        fs::create_dir_all(jdk_path.join("bin")).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        let java_home = jdk.resolve_java_home();
        assert_eq!(java_home, jdk_path);
    }

    #[test]
    fn test_resolve_bin_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("test-jdk");

        // Create a bin directory
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "test".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path.clone(),
        };

        let resolved_bin = jdk.resolve_bin_path().unwrap();
        assert_eq!(resolved_bin, bin_path);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_resolve_bin_path_bundle_structure() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("temurin-21.0.1");

        // Create bundle structure
        let bundle_bin_path = jdk_path.join("Contents").join("Home").join("bin");
        fs::create_dir_all(&bundle_bin_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path,
        };

        let resolved_bin = jdk.resolve_bin_path().unwrap();
        assert_eq!(resolved_bin, bundle_bin_path);
    }

    #[test]
    fn test_resolve_bin_path_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("broken-jdk");
        fs::create_dir_all(&jdk_path).unwrap();

        let jdk = InstalledJdk {
            distribution: "broken".to_string(),
            version: Version::new(21, 0, 1),
            path: jdk_path,
        };

        let result = jdk.resolve_bin_path();
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                KopiError::SystemError(msg) => {
                    assert!(msg.contains("bin directory not found"));
                }
                _ => panic!("Expected SystemError"),
            }
        }
    }

    #[test]
    fn test_installed_jdk_write_to() {
        let temp_dir = TempDir::new().unwrap();
        let version_file = temp_dir.path().join("test-version");

        let jdk = InstalledJdk {
            distribution: "temurin".to_string(),
            version: Version::new(21, 0, 1),
            path: temp_dir.path().join("temurin-21.0.1"),
        };

        jdk.write_to(&version_file).unwrap();

        let content = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content, "temurin@21.0.1");

        // Test overwriting
        let jdk2 = InstalledJdk {
            distribution: "corretto".to_string(),
            version: Version::new(17, 0, 9),
            path: temp_dir.path().join("corretto-17.0.9"),
        };

        jdk2.write_to(&version_file).unwrap();

        let content2 = fs::read_to_string(&version_file).unwrap();
        assert_eq!(content2, "corretto@17.0.9");
    }
}
