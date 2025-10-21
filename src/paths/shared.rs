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
use std::fs;
use std::path::{Path, PathBuf};

/// Normalise an arbitrary string into a filesystem-safe slug fragment.
///
/// The transformation mirrors the sanitisation logic previously hosted under
/// `locking::package_coordinate::sanitize_segment`, preserving backwards
/// compatibility for directory naming conventions.
pub fn sanitize_segment(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut last_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            output.push('-');
            last_dash = true;
        }
    }

    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Ensure the provided path exists, returning it on success.
pub fn ensure_directory(path: PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(&path).map_err(|error| {
        KopiError::ConfigError(format!(
            "Failed to create directory {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

/// Join a single directory segment onto the root and ensure the resulting path exists.
pub fn ensure_child_directory(root: &Path, child: &str) -> Result<PathBuf> {
    ensure_directory(root.join(child))
}

/// Join multiple directory segments onto the root and ensure the resulting path exists.
pub fn ensure_nested_directory<'a, I>(root: &Path, segments: I) -> Result<PathBuf>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut path = PathBuf::from(root);
    for segment in segments {
        path.push(segment);
    }
    ensure_directory(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sanitize_segment_matches_legacy_behaviour() {
        assert_eq!(sanitize_segment(" Tem urin "), Some("tem-urin".to_string()));
        assert_eq!(sanitize_segment("***"), None);
        assert_eq!(
            sanitize_segment("Zulu-21.0.1+35.1"),
            Some("zulu-21-0-1-35-1".to_string())
        );
    }

    #[test]
    fn ensure_nested_directory_creates_full_path() {
        let temp_dir = TempDir::new().unwrap();
        let nested = ensure_nested_directory(temp_dir.path(), ["locks", "install"]).unwrap();
        assert_eq!(nested, temp_dir.path().join("locks").join("install"));
        assert!(nested.exists());
    }
}
