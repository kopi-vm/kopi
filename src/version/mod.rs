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
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub mod file;
pub mod parser;
pub mod resolver;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub components: Vec<u32>,        // All numeric components
    pub build: Option<Vec<u32>>,     // Build numbers as numeric array
    pub pre_release: Option<String>, // Pre-release string
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            components: vec![major, minor, patch],
            build: None,
            pre_release: None,
        }
    }

    pub fn from_components(major: u32, minor: Option<u32>, patch: Option<u32>) -> Self {
        let mut components = vec![major];
        if let Some(minor) = minor {
            components.push(minor);
            if let Some(patch) = patch {
                components.push(patch);
            }
        }
        Self {
            components,
            build: None,
            pre_release: None,
        }
    }

    pub fn with_build(mut self, build: String) -> Self {
        // Parse build string into numeric components if possible
        let build_parts: Vec<u32> = build
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();

        if !build_parts.is_empty() {
            self.build = Some(build_parts);
        } else {
            // If build is not numeric, store it as pre-release
            self.pre_release = Some(build);
        }
        self
    }

    /// Try to extract a build number from the version components.
    /// For example, convert "24.0.2.12.1" to "24.0.2" with build [12].
    /// This is useful for matching versions where build numbers are incorporated into components.
    pub fn try_extract_build(&self) -> Option<Version> {
        // Only attempt extraction if we have more than 3 components and no existing build
        if self.components.len() > 3 && self.build.is_none() {
            // Check if the 4th component could be a build number
            if let Some(&potential_build) = self.components.get(3) {
                // Create a new version with the first 3 components and the 4th as build
                let new_version = Version {
                    components: self.components[..3].to_vec(),
                    build: Some(vec![potential_build]),
                    pre_release: self.pre_release.clone(),
                };

                // If there are more components after the build, keep the original
                if self.components.len() > 4 {
                    // This handles cases like "24.0.2.12.1" where we can't cleanly extract
                    // In this case, don't extract the build
                    return None;
                }

                return Some(new_version);
            }
        }
        None
    }

    /// Convert a version with build number to one with build incorporated into components.
    /// For example, convert "24.0.2" with build [12] to "24.0.2.12".
    /// This is useful for creating directory names that include the build number.
    pub fn incorporate_build_into_components(&self) -> Version {
        if let Some(build) = &self.build
            && build.len() == 1
        {
            let mut new_components = self.components.clone();
            new_components.push(build[0]);
            return Version {
                components: new_components,
                build: None,
                pre_release: self.pre_release.clone(),
            };
        }
        self.clone()
    }

    // Helper methods for backward compatibility
    pub fn major(&self) -> u32 {
        self.components.first().copied().unwrap_or(0)
    }

    pub fn minor(&self) -> Option<u32> {
        self.components.get(1).copied()
    }

    pub fn patch(&self) -> Option<u32> {
        self.components.get(2).copied()
    }

    /// Matches a version string against this version.
    /// When the user specifies "21", it matches cache entries like "21.0" and "21.0.0".
    /// When the user specifies "21.0.0", it does NOT match cache entries like "21".
    /// When the user specifies "21.0", it matches cache entries like "21.0.0" and "21.0+32".
    /// When the user specifies "X.Y.Z+B", it also matches "X.Y.Z.B" or "X.Y.Z.B.*" (build incorporated into components).
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(pattern_version) = Version::from_str(pattern) {
            log::trace!("Matching version {self} against pattern {pattern}");

            // First try standard matching
            if self.matches_standard(&pattern_version) {
                log::trace!("Standard match succeeded");
                return true;
            }

            // If pattern has a build number, try flexible build matching
            // This handles cases where build numbers are incorporated into version components
            // e.g., pattern "24.0.2+12" matches "24.0.2.12.1"
            if let Some(pattern_build) = &pattern_version.build
                && pattern_build.len() == 1
            {
                let build_num = pattern_build[0];
                let pattern_comp_len = pattern_version.components.len();

                log::trace!(
                    "Trying flexible build matching: pattern has build {build_num}, self has {} components",
                    self.components.len()
                );

                // Check if self has the pattern components followed by the build number
                if self.components.len() > pattern_comp_len {
                    // Check that initial components match
                    for (i, pattern_comp) in pattern_version.components.iter().enumerate() {
                        if self.components.get(i) != Some(pattern_comp) {
                            log::trace!(
                                "Component mismatch at index {i}: {pattern_comp} != {:?}",
                                self.components.get(i)
                            );
                            return false;
                        }
                    }

                    // Check if the next component matches the build number
                    if self.components.get(pattern_comp_len) == Some(&build_num) {
                        // This handles cases like:
                        // pattern "24.0.2+12" matches "24.0.2.12" or "24.0.2.12.1"
                        log::trace!("Flexible build match succeeded");
                        return true;
                    } else {
                        log::trace!(
                            "Build number mismatch: expected {build_num}, got {:?}",
                            self.components.get(pattern_comp_len)
                        );
                    }
                }
            }

            // Also handle the reverse case: pattern without build but self has build
            // e.g., pattern "21.0.5.11" should match self "21.0.5+11"
            if self.build.is_some()
                && self.build.as_ref().unwrap().len() == 1
                && pattern_version.build.is_none()
                && pattern_version.components.len() == self.components.len() + 1
            {
                // Check if pattern's last component matches our build number
                let build_num = self.build.as_ref().unwrap()[0];
                let pattern_last_comp = pattern_version.components.last().unwrap();

                if *pattern_last_comp == build_num {
                    // Check that all other components match
                    for i in 0..self.components.len() {
                        if self.components[i] != pattern_version.components[i] {
                            return false;
                        }
                    }
                    log::trace!("Reverse flexible build match succeeded");
                    return true;
                }
            }

            log::trace!("No match found");
            false
        } else {
            log::trace!("Failed to parse pattern: {pattern}");
            false
        }
    }

    /// Standard version matching without flexible build handling
    fn matches_standard(&self, pattern_version: &Version) -> bool {
        // Compare components up to the length specified in pattern
        for (i, pattern_comp) in pattern_version.components.iter().enumerate() {
            match self.components.get(i) {
                Some(self_comp) => {
                    if pattern_comp != self_comp {
                        return false;
                    }
                }
                None => {
                    // Pattern specifies more components than self has
                    return false;
                }
            }
        }

        // Build matching if specified
        if let Some(pattern_build) = &pattern_version.build {
            if let Some(self_build) = &self.build {
                if pattern_build != self_build {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Pre-release matching if specified
        if let Some(pattern_pre) = &pattern_version.pre_release {
            if let Some(self_pre) = &self.pre_release {
                if pattern_pre != self_pre {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl FromStr for Version {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(KopiError::InvalidVersionFormat(s.to_string()));
        }

        let mut remaining = s;
        let mut pre_release = None;
        let mut build = None;

        // Check for pre-release part (after '-')
        // But we need to be careful not to split build metadata that contains '-'
        // First check if there's a '+' and handle that first
        let plus_pos = remaining.find('+');
        let dash_pos = remaining.find('-');

        match (plus_pos, dash_pos) {
            (Some(p), Some(d)) => {
                if p < d {
                    // '+' comes before '-', so everything after '+' is build/pre-release
                    let (before_plus, after_plus) = remaining.split_at(p);
                    remaining = before_plus;
                    let build_str = &after_plus[1..];

                    // Check if build string is empty
                    if build_str.is_empty() {
                        return Err(KopiError::InvalidVersionFormat(s.to_string()));
                    }

                    // Check if build string is purely numeric
                    let parts: Vec<&str> = build_str.split('.').collect();
                    if parts
                        .iter()
                        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                    {
                        let build_parts: Vec<u32> =
                            parts.iter().map(|s| s.parse().unwrap()).collect();
                        build = Some(build_parts);
                    } else {
                        // Not purely numeric, treat as pre-release
                        pre_release = Some(build_str.to_string());
                    }
                } else {
                    // '-' comes before '+', handle pre-release first
                    let (before_dash, after_dash) = remaining.split_at(d);
                    remaining = before_dash;
                    let pre_str = &after_dash[1..];

                    // Check if pre-release string is empty
                    if pre_str.is_empty() {
                        return Err(KopiError::InvalidVersionFormat(s.to_string()));
                    }

                    pre_release = Some(pre_str.to_string());
                }
            }
            (Some(p), None) => {
                // Only '+' present
                let (before_plus, after_plus) = remaining.split_at(p);
                remaining = before_plus;
                let build_str = &after_plus[1..];

                // Check if build string is empty
                if build_str.is_empty() {
                    return Err(KopiError::InvalidVersionFormat(s.to_string()));
                }

                // Check if build string is purely numeric
                let parts: Vec<&str> = build_str.split('.').collect();
                if parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                {
                    let build_parts: Vec<u32> = parts.iter().map(|s| s.parse().unwrap()).collect();
                    build = Some(build_parts);
                } else {
                    // Not purely numeric, treat as pre-release
                    pre_release = Some(build_str.to_string());
                }
            }
            (None, Some(d)) => {
                // Only '-' present
                let (before_dash, after_dash) = remaining.split_at(d);
                remaining = before_dash;
                let pre_str = &after_dash[1..];

                // Check if pre-release string is empty
                if pre_str.is_empty() {
                    return Err(KopiError::InvalidVersionFormat(s.to_string()));
                }

                pre_release = Some(pre_str.to_string());
            }
            (None, None) => {
                // Neither '+' nor '-' present
            }
        }

        // Parse numeric components
        let components: Result<Vec<u32>> = remaining
            .split('.')
            .map(|s| {
                s.parse::<u32>()
                    .map_err(|_| KopiError::InvalidVersionFormat(s.to_string()))
            })
            .collect();

        let components = components?;

        if components.is_empty() {
            return Err(KopiError::InvalidVersionFormat(s.to_string()));
        }

        Ok(Version {
            components,
            build,
            pre_release,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Write components separated by dots
        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{component}")?;
        }

        // Write build if present
        if let Some(build) = &self.build {
            write!(f, "+")?;
            for (i, component) in build.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                write!(f, "{component}")?;
            }
        }

        // Write pre-release if present
        if let Some(pre_release) = &self.pre_release {
            write!(f, "-{pre_release}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequest {
    pub version_pattern: String,
    pub distribution: Option<String>,
    pub package_type: Option<crate::models::package::PackageType>,
    pub javafx_bundled: Option<bool>,
}

impl VersionRequest {
    pub fn new(version_pattern: String) -> Result<Self> {
        // Special handling for "latest" - not allowed for local command
        if version_pattern.eq_ignore_ascii_case("latest") {
            return Err(KopiError::InvalidVersionFormat(
                "Local command requires a specific version, not 'latest'".to_string(),
            ));
        }

        // Validate that the pattern can be parsed as a version
        Version::from_str(&version_pattern)?;
        Ok(Self {
            version_pattern,
            distribution: None,
            package_type: None,
            javafx_bundled: None,
        })
    }

    pub fn with_distribution(mut self, distribution: String) -> Self {
        self.distribution = Some(distribution);
        self
    }

    pub fn with_package_type(mut self, package_type: crate::models::package::PackageType) -> Self {
        self.package_type = Some(package_type);
        self
    }

    pub fn with_javafx_bundled(mut self, javafx_bundled: bool) -> Self {
        self.javafx_bundled = Some(javafx_bundled);
        self
    }
}

impl std::fmt::Display for VersionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = match &self.distribution {
            Some(dist) => format!("{}@{}", dist, self.version_pattern),
            None => self.version_pattern.clone(),
        };

        // Append JavaFX suffix if specified
        if self.javafx_bundled == Some(true) {
            write!(f, "{base}+fx")
        } else {
            write!(f, "{base}")
        }
    }
}

impl FromStr for VersionRequest {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        // Check for JavaFX suffix (+fx at the end)
        let (javafx_bundled, remaining) = if let Some(stripped) = s.strip_suffix("+fx") {
            (Some(true), stripped)
        } else {
            (None, s)
        };

        let mut request = if remaining.contains('@') {
            let parts: Vec<&str> = remaining.split('@').collect();
            match parts.len() {
                2 => {
                    // Legacy format: distribution@version
                    VersionRequest::new(parts[1].to_string())?
                        .with_distribution(parts[0].to_string())
                }
                3 => {
                    // New format: package_type@version@distribution
                    let package_type = crate::models::package::PackageType::from_str(parts[0])?;
                    VersionRequest::new(parts[1].to_string())?
                        .with_distribution(parts[2].to_string())
                        .with_package_type(package_type)
                }
                _ => return Err(KopiError::InvalidVersionFormat(s.to_string())),
            }
        } else {
            VersionRequest::new(remaining.to_string())?
        };

        // Apply JavaFX bundled flag if present
        if let Some(javafx) = javafx_bundled {
            request = request.with_javafx_bundled(javafx);
        }

        Ok(request)
    }
}

/// Format version in minimal representation
/// - Just major version if minor and patch are 0 (e.g., "21" instead of "21.0.0")
/// - Major.minor if patch is 0 (e.g., "21.1" instead of "21.1.0")
/// - Full version otherwise
pub fn format_version_minimal(version: &Version) -> String {
    if version.minor() == Some(0) && version.patch() == Some(0) {
        // Just major version (e.g., "21" instead of "21.0.0")
        version.major().to_string()
    } else if version.patch() == Some(0) {
        // Major.minor (e.g., "21.1" instead of "21.1.0")
        format!("{}.{}", version.major(), version.minor().unwrap())
    } else {
        // Full version
        version.to_string()
    }
}

/// Common validation for version commands
pub fn validate_version_for_command<'a>(
    version: &'a Option<Version>,
    command_name: &str,
) -> Result<&'a Version> {
    version.as_ref().ok_or_else(|| {
        KopiError::InvalidVersionFormat(format!(
            "{command_name} command requires a specific version (e.g., '21' or 'temurin@21')"
        ))
    })
}

/// Build a VersionRequest for auto-installation
pub fn build_install_request(
    distribution: &crate::models::distribution::Distribution,
    version: &Version,
) -> VersionRequest {
    VersionRequest {
        distribution: Some(distribution.id().to_string()),
        version_pattern: version.to_string(),
        package_type: None,
        javafx_bundled: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // Basic versions
        assert_eq!(
            Version::from_str("21").unwrap(),
            Version::from_components(21, None, None)
        );
        assert_eq!(
            Version::from_str("21.0").unwrap(),
            Version::from_components(21, Some(0), None)
        );
        assert_eq!(Version::from_str("21.0.0").unwrap(), Version::new(21, 0, 0));
        assert_eq!(Version::from_str("17.0.9").unwrap(), Version::new(17, 0, 9));

        // Version with numeric build
        let v = Version::from_str("11.0.2+9").unwrap();
        assert_eq!(v.components, vec![11, 0, 2]);
        assert_eq!(v.build, Some(vec![9]));

        // Extended versions (Corretto format)
        let v = Version::from_str("21.0.7.6.1").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 6, 1]);
        assert_eq!(v.build, None);

        // Dragonwell 6-component format
        let v = Version::from_str("21.0.7.0.7.6").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 0, 7, 6]);

        // Multi-component build
        let v = Version::from_str("21.0.7+9.1").unwrap();
        assert_eq!(v.components, vec![21, 0, 7]);
        assert_eq!(v.build, Some(vec![9, 1]));

        // Pre-release version
        let v = Version::from_str("21.0.7-ea").unwrap();
        assert_eq!(v.components, vec![21, 0, 7]);
        assert_eq!(v.pre_release, Some("ea".to_string()));

        assert!(Version::from_str("invalid").is_err());
        assert!(Version::from_str("").is_err());
    }

    #[test]
    fn test_version_display() {
        assert_eq!(Version::from_components(21, None, None).to_string(), "21");
        assert_eq!(
            Version::from_components(21, Some(0), None).to_string(),
            "21.0"
        );
        assert_eq!(Version::new(21, 0, 0).to_string(), "21.0.0");
        assert_eq!(Version::new(17, 0, 9).to_string(), "17.0.9");

        // Version with single-component build
        let v = Version::from_str("11.0.2+9").unwrap();
        assert_eq!(v.to_string(), "11.0.2+9");

        // Extended Corretto version
        let v = Version::from_str("21.0.7.6.1").unwrap();
        assert_eq!(v.to_string(), "21.0.7.6.1");

        // Multi-component build
        let v = Version::from_str("21.0.7+9.1.3").unwrap();
        assert_eq!(v.to_string(), "21.0.7+9.1.3");

        // Pre-release version
        let v = Version::from_str("21.0.7-ea").unwrap();
        assert_eq!(v.to_string(), "21.0.7-ea");
    }

    #[test]
    fn test_version_matching() {
        // Test matching with full version
        let v21_0_1 = Version::new(21, 0, 1);
        assert!(v21_0_1.matches_pattern("21")); // User specifies 21, matches 21.0.1
        assert!(!v21_0_1.matches_pattern("17"));

        let v17_0_9 = Version::new(17, 0, 9);
        assert!(v17_0_9.matches_pattern("17"));
        assert!(v17_0_9.matches_pattern("17.0"));
        assert!(v17_0_9.matches_pattern("17.0.9"));
        assert!(!v17_0_9.matches_pattern("17.0.8"));

        // Test that cache entries with fewer components don't match specific user values
        let v21 = Version::from_components(21, None, None);
        assert!(v21.matches_pattern("21"));
        assert!(!v21.matches_pattern("21.0")); // User specifies 21.0, cache has only 21
        assert!(!v21.matches_pattern("21.0.0")); // User specifies 21.0.0, cache has only 21

        let v21_0 = Version::from_components(21, Some(0), None);
        assert!(v21_0.matches_pattern("21")); // User specifies 21, matches 21.0
        assert!(v21_0.matches_pattern("21.0")); // User specifies 21.0, matches 21.0
        assert!(!v21_0.matches_pattern("21.0.0")); // User specifies 21.0.0, cache has only 21.0

        // Test extended version matching (Corretto)
        let v_corretto = Version::from_str("21.0.7.6.1").unwrap();
        assert!(v_corretto.matches_pattern("21"));
        assert!(v_corretto.matches_pattern("21.0"));
        assert!(v_corretto.matches_pattern("21.0.7"));
        assert!(v_corretto.matches_pattern("21.0.7.6"));
        assert!(v_corretto.matches_pattern("21.0.7.6.1"));
        assert!(!v_corretto.matches_pattern("21.0.7.6.2"));
    }

    #[test]
    fn test_matches_pattern() {
        // Test with complete version in cache
        let v21_0_0_build = Version::new(21, 0, 0).with_build("37".to_string());
        assert!(v21_0_0_build.matches_pattern("21")); // User: 21, Cache: 21.0.0+37 - match
        assert!(v21_0_0_build.matches_pattern("21.0")); // User: 21.0, Cache: 21.0.0+37 - match
        assert!(v21_0_0_build.matches_pattern("21.0.0")); // User: 21.0.0, Cache: 21.0.0+37 - match
        assert!(v21_0_0_build.matches_pattern("21.0.0+37")); // With build - match
        assert!(!v21_0_0_build.matches_pattern("21.0.0+38")); // Different build - no match
        assert!(!v21_0_0_build.matches_pattern("22")); // Different major - no match

        // Test with non-zero minor/patch
        let v21_0_7_build = Version::new(21, 0, 7).with_build("9".to_string());
        assert!(v21_0_7_build.matches_pattern("21")); // User: 21, Cache: 21.0.7+9 - match
        assert!(v21_0_7_build.matches_pattern("21.0")); // User: 21.0, Cache: 21.0.7+9 - match  
        assert!(!v21_0_7_build.matches_pattern("21.0.0")); // User: 21.0.0, Cache: 21.0.7 - no match (different patch)
        assert!(v21_0_7_build.matches_pattern("21.0.7")); // Exact match
        assert!(v21_0_7_build.matches_pattern("21.0.7+9")); // Exact match with build
        assert!(!v21_0_7_build.matches_pattern("21.0.7+10")); // Different build

        // Test version without build
        let v17_0_9 = Version::new(17, 0, 9);
        assert!(v17_0_9.matches_pattern("17")); // User: 17, Cache: 17.0.9 - match
        assert!(v17_0_9.matches_pattern("17.0")); // User: 17.0, Cache: 17.0.9 - match
        assert!(v17_0_9.matches_pattern("17.0.9")); // Exact match
        assert!(!v17_0_9.matches_pattern("17.0.8")); // Different patch
        assert!(!v17_0_9.matches_pattern("17.1")); // Different minor

        // Test incomplete versions in cache
        let v21 = Version::from_components(21, None, None);
        assert!(v21.matches_pattern("21")); // User: 21, Cache: 21 - match
        assert!(!v21.matches_pattern("21.0")); // User: 21.0, Cache: 21 - no match
        assert!(!v21.matches_pattern("21.0.0")); // User: 21.0.0, Cache: 21 - no match

        let v21_0 = Version::from_components(21, Some(0), None);
        assert!(v21_0.matches_pattern("21")); // User: 21, Cache: 21.0 - match
        assert!(v21_0.matches_pattern("21.0")); // User: 21.0, Cache: 21.0 - match
        assert!(!v21_0.matches_pattern("21.0.0")); // User: 21.0.0, Cache: 21.0 - no match

        // Test major-only version with build
        let v23_build = Version::from_components(23, None, None).with_build("38".to_string());
        assert!(v23_build.matches_pattern("23")); // User: 23, Cache: 23+38 - match
        assert!(v23_build.matches_pattern("23+38")); // With build - match
        assert!(!v23_build.matches_pattern("23+37")); // Different build - no match
        assert!(!v23_build.matches_pattern("23.0")); // User: 23.0, Cache: 23+38 - no match
    }

    #[test]
    fn test_version_request_parsing() {
        let req = VersionRequest::from_str("21").unwrap();
        assert_eq!(req.version_pattern, "21");
        assert_eq!(req.distribution, None);
        assert_eq!(req.package_type, None);

        // Legacy format: distribution@version
        let req = VersionRequest::from_str("corretto@17").unwrap();
        assert_eq!(req.version_pattern, "17");
        assert_eq!(req.distribution, Some("corretto".to_string()));
        assert_eq!(req.package_type, None);

        // New format: package_type@version@distribution
        let req = VersionRequest::from_str("jre@21@temurin").unwrap();
        assert_eq!(req.version_pattern, "21");
        assert_eq!(req.distribution, Some("temurin".to_string()));
        assert_eq!(
            req.package_type,
            Some(crate::models::package::PackageType::Jre)
        );

        let req = VersionRequest::from_str("jdk@17.0.9@corretto").unwrap();
        assert_eq!(req.version_pattern, "17.0.9");
        assert_eq!(req.distribution, Some("corretto".to_string()));
        assert_eq!(
            req.package_type,
            Some(crate::models::package::PackageType::Jdk)
        );

        // Invalid formats
        assert!(VersionRequest::from_str("invalid@format@").is_err());
        assert!(VersionRequest::from_str("too@many@parts@here").is_err());
        assert!(VersionRequest::from_str("invalid_type@21@temurin").is_err()); // Invalid package type
    }

    #[test]
    fn test_version_request_with_javafx() {
        // Test version with JavaFX
        let req = VersionRequest::from_str("21+fx").unwrap();
        assert_eq!(req.version_pattern, "21");
        assert_eq!(req.distribution, None);
        assert_eq!(req.javafx_bundled, Some(true));

        // Test distribution@version with JavaFX
        let req = VersionRequest::from_str("liberica@21+fx").unwrap();
        assert_eq!(req.version_pattern, "21");
        assert_eq!(req.distribution, Some("liberica".to_string()));
        assert_eq!(req.javafx_bundled, Some(true));

        // Test full version with JavaFX
        let req = VersionRequest::from_str("zulu@21.0.5+fx").unwrap();
        assert_eq!(req.version_pattern, "21.0.5");
        assert_eq!(req.distribution, Some("zulu".to_string()));
        assert_eq!(req.javafx_bundled, Some(true));

        // Test version with build number and JavaFX
        let req = VersionRequest::from_str("corretto@21.0.5+11+fx").unwrap();
        assert_eq!(req.version_pattern, "21.0.5+11");
        assert_eq!(req.distribution, Some("corretto".to_string()));
        assert_eq!(req.javafx_bundled, Some(true));

        // Test without JavaFX
        let req = VersionRequest::from_str("21").unwrap();
        assert_eq!(req.javafx_bundled, None);

        // Test Display format with JavaFX
        let req = VersionRequest::from_str("liberica@21+fx").unwrap();
        assert_eq!(req.to_string(), "liberica@21+fx");

        // Test Display format without JavaFX
        let req = VersionRequest::from_str("liberica@21").unwrap();
        assert_eq!(req.to_string(), "liberica@21");
    }

    #[test]
    fn test_corretto_version_formats() {
        // Corretto 4-component format
        let v = Version::from_str("21.0.7.6").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 6]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, None);

        // Corretto 5-component format
        let v = Version::from_str("21.0.7.6.1").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 6, 1]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, None);

        // Corretto Java 8 special format (no leading zero)
        let v = Version::from_str("8.452.9.1").unwrap();
        assert_eq!(v.components, vec![8, 452, 9, 1]);
        assert_eq!(v.major(), 8);

        // Corretto with build number
        let v = Version::from_str("21.0.7.6.1+13").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 6, 1]);
        assert_eq!(v.build, Some(vec![13]));
    }

    #[test]
    fn test_flexible_build_matching() {
        // Test that X.Y.Z+B matches X.Y.Z.B and X.Y.Z.B.C

        // Corretto case: 24.0.2+12 should match 24.0.2.12.1
        let installed = Version::from_str("24.0.2.12.1").unwrap();
        assert!(installed.matches_pattern("24.0.2+12"));

        // Should also match without the .1
        let installed = Version::from_str("24.0.2.12").unwrap();
        assert!(installed.matches_pattern("24.0.2+12"));

        // Zulu case: 21.0.5+11 should match 21.0.5.11
        let installed = Version::from_str("21.0.5.11").unwrap();
        assert!(installed.matches_pattern("21.0.5+11"));

        // Should also match with additional components
        let installed = Version::from_str("21.0.5.11.0.25").unwrap();
        assert!(installed.matches_pattern("21.0.5+11"));

        // Should NOT match if build number is different
        let installed = Version::from_str("24.0.2.13.1").unwrap();
        assert!(!installed.matches_pattern("24.0.2+12"));

        // Should NOT match if base version is different
        let installed = Version::from_str("24.0.3.12.1").unwrap();
        assert!(!installed.matches_pattern("24.0.2+12"));

        // Standard matching should still work (exact build match)
        let installed = Version::from_str("21.0.5+11").unwrap();
        assert!(installed.matches_pattern("21.0.5+11"));
        assert!(!installed.matches_pattern("21.0.5+12"));
    }

    #[test]
    fn test_reverse_build_matching() {
        // Test the reverse case: pattern has build as component, installed has build metadata

        // Pattern "21.0.5.11" should match installed "21.0.5+11"
        let installed = Version::from_str("21.0.5+11").unwrap();
        assert!(installed.matches_pattern("21.0.5.11"));

        // Pattern "24.0.2.12" should match installed "24.0.2+12"
        let installed = Version::from_str("24.0.2+12").unwrap();
        assert!(installed.matches_pattern("24.0.2.12"));

        // Should NOT match if build number is different
        let installed = Version::from_str("21.0.5+12").unwrap();
        assert!(!installed.matches_pattern("21.0.5.11"));

        // Should NOT match if base version is different
        let installed = Version::from_str("21.0.4+11").unwrap();
        assert!(!installed.matches_pattern("21.0.5.11"));
    }

    #[test]
    fn test_local_command_version_matching() {
        // Specific test for the reported issue:
        // kopi install 21.0.5+11 creates directory with version 21.0.5.11
        // kopi local 21.0.5+11 should find it

        // Simulate installed JDK with version parsed from directory name
        let installed_version = Version::from_str("21.0.5.11").unwrap();

        // User runs: kopi local 21.0.5+11
        let search_pattern = "21.0.5+11";

        // This should match
        assert!(
            installed_version.matches_pattern(search_pattern),
            "Version {installed_version} should match pattern {search_pattern}"
        );

        // Also test with different vendors that format versions differently

        // Temurin format
        let temurin_installed = Version::from_str("21.0.5.11").unwrap();
        assert!(temurin_installed.matches_pattern("21.0.5+11"));

        // Corretto format (may have additional components)
        let corretto_installed = Version::from_str("21.0.5.11.1").unwrap();
        assert!(corretto_installed.matches_pattern("21.0.5+11"));

        // Zulu format
        let zulu_installed = Version::from_str("21.0.5.11.0.25").unwrap();
        assert!(zulu_installed.matches_pattern("21.0.5+11"));
    }

    #[test]
    fn test_try_extract_build() {
        // Test extracting build from 4-component version
        let v = Version::from_str("24.0.2.12").unwrap();
        let extracted = v.try_extract_build().unwrap();
        assert_eq!(extracted.components, vec![24, 0, 2]);
        assert_eq!(extracted.build, Some(vec![12]));

        // Should not extract from 5-component version (ambiguous)
        let v = Version::from_str("24.0.2.12.1").unwrap();
        assert!(v.try_extract_build().is_none());

        // Should not extract from 3-component version
        let v = Version::from_str("24.0.2").unwrap();
        assert!(v.try_extract_build().is_none());

        // Should not extract if already has build
        let v = Version::from_str("24.0.2.12+5").unwrap();
        assert!(v.try_extract_build().is_none());
    }

    #[test]
    fn test_incorporate_build_into_components() {
        // Test incorporating build into components
        let v = Version::from_str("24.0.2+12").unwrap();
        let incorporated = v.incorporate_build_into_components();
        assert_eq!(incorporated.components, vec![24, 0, 2, 12]);
        assert_eq!(incorporated.build, None);

        // Should not change if no build
        let v = Version::from_str("24.0.2").unwrap();
        let incorporated = v.incorporate_build_into_components();
        assert_eq!(incorporated.components, vec![24, 0, 2]);
        assert_eq!(incorporated.build, None);

        // Should not change if multi-component build
        let v = Version::from_str("24.0.2+12.1").unwrap();
        let incorporated = v.incorporate_build_into_components();
        assert_eq!(incorporated.components, vec![24, 0, 2]);
        assert_eq!(incorporated.build, Some(vec![12, 1]));
    }

    #[test]
    fn test_dragonwell_version_formats() {
        // Dragonwell 6-component format
        let v = Version::from_str("21.0.7.0.7.6").unwrap();
        assert_eq!(v.components, vec![21, 0, 7, 0, 7, 6]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, None);

        // Dragonwell with build
        let v = Version::from_str("17.0.13.0.13.11+11").unwrap();
        assert_eq!(v.components, vec![17, 0, 13, 0, 13, 11]);
        assert_eq!(v.build, Some(vec![11]));
    }

    #[test]
    fn test_jetbrains_large_build_numbers() {
        // JetBrains Runtime with large build numbers
        let v = Version::from_str("21.0.5+13.674.11").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, Some(vec![13, 674, 11]));

        // JetBrains Runtime with b prefix (not numeric, so becomes pre-release)
        let v = Version::from_str("21.0.5+13-b674.11").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, Some("13-b674.11".to_string()));
    }

    #[test]
    fn test_graalvm_complex_identifiers() {
        // GraalVM CE with jvmci identifier
        let v = Version::from_str("21.0.5+11-jvmci-24.1-b01").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, Some("11-jvmci-24.1-b01".to_string()));

        // GraalVM EE with complex pre-release
        let v = Version::from_str("21.0.5-ea+11").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, None);
        assert_eq!(v.pre_release, Some("ea+11".to_string()));
    }

    #[test]
    fn test_edge_cases() {
        // Single component
        let v = Version::from_str("8").unwrap();
        assert_eq!(v.components, vec![8]);
        assert_eq!(v.major(), 8);
        assert_eq!(v.minor(), None);
        assert_eq!(v.patch(), None);

        // Many components (theoretical case)
        let v = Version::from_str("1.2.3.4.5.6.7.8.9").unwrap();
        assert_eq!(v.components, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        // Zero values
        let v = Version::from_str("0.0.0").unwrap();
        assert_eq!(v.components, vec![0, 0, 0]);

        // Mixed zeros and non-zeros
        let v = Version::from_str("21.0.0.0.1").unwrap();
        assert_eq!(v.components, vec![21, 0, 0, 0, 1]);
    }

    #[test]
    fn test_invalid_formats() {
        // Empty string
        assert!(Version::from_str("").is_err());

        // Non-numeric components
        assert!(Version::from_str("abc").is_err());
        assert!(Version::from_str("21.x.0").is_err());
        assert!(Version::from_str("21.0.0.beta").is_err());

        // Invalid separators
        assert!(Version::from_str("21_0_7").is_err());
        assert!(Version::from_str("21,0,7").is_err());

        // Leading/trailing dots
        assert!(Version::from_str(".21.0.7").is_err());
        assert!(Version::from_str("21.0.7.").is_err());
        assert!(Version::from_str("21..0").is_err());

        // Invalid build/pre-release
        assert!(Version::from_str("21.0.7+").is_err());
        assert!(Version::from_str("21.0.7-").is_err());
    }

    #[test]
    fn test_version_pattern_matching_extended() {
        // Test Corretto 4-5 component matching
        let v_corretto = Version::from_str("21.0.7.6.1").unwrap();
        assert!(v_corretto.matches_pattern("21"));
        assert!(v_corretto.matches_pattern("21.0"));
        assert!(v_corretto.matches_pattern("21.0.7"));
        assert!(v_corretto.matches_pattern("21.0.7.6"));
        assert!(v_corretto.matches_pattern("21.0.7.6.1"));
        assert!(!v_corretto.matches_pattern("21.0.7.6.2"));
        assert!(!v_corretto.matches_pattern("21.0.7.5"));

        // Test Dragonwell 6-component matching
        let v_dragonwell = Version::from_str("21.0.7.0.7.6").unwrap();
        assert!(v_dragonwell.matches_pattern("21"));
        assert!(v_dragonwell.matches_pattern("21.0"));
        assert!(v_dragonwell.matches_pattern("21.0.7"));
        assert!(v_dragonwell.matches_pattern("21.0.7.0"));
        assert!(v_dragonwell.matches_pattern("21.0.7.0.7"));
        assert!(v_dragonwell.matches_pattern("21.0.7.0.7.6"));
        assert!(!v_dragonwell.matches_pattern("21.0.7.0.7.5"));

        // Test build number matching
        let v_with_build = Version::from_str("21.0.5+13.674.11").unwrap();
        assert!(v_with_build.matches_pattern("21"));
        assert!(v_with_build.matches_pattern("21.0"));
        assert!(v_with_build.matches_pattern("21.0.5"));
        assert!(v_with_build.matches_pattern("21.0.5+13.674.11"));
        assert!(!v_with_build.matches_pattern("21.0.5+13.674"));
        assert!(!v_with_build.matches_pattern("21.0.5+13.674.12"));

        // Test pre-release matching
        let v_pre = Version::from_str("21.0.5-ea").unwrap();
        assert!(v_pre.matches_pattern("21"));
        assert!(v_pre.matches_pattern("21.0"));
        assert!(v_pre.matches_pattern("21.0.5"));
        assert!(v_pre.matches_pattern("21.0.5-ea"));
        assert!(!v_pre.matches_pattern("21.0.5-beta"));
    }

    #[test]
    fn test_version_ordering() {
        // Basic ordering
        assert!(Version::from_str("21").unwrap() < Version::from_str("22").unwrap());
        assert!(Version::from_str("21.0").unwrap() < Version::from_str("21.1").unwrap());
        assert!(Version::from_str("21.0.0").unwrap() < Version::from_str("21.0.1").unwrap());

        // Extended component ordering
        assert!(Version::from_str("21.0.7.6").unwrap() < Version::from_str("21.0.7.6.1").unwrap());
        assert!(
            Version::from_str("21.0.7.5.9").unwrap() < Version::from_str("21.0.7.6.1").unwrap()
        );

        // Same version different component count
        assert!(Version::from_str("21").unwrap() < Version::from_str("21.0").unwrap());
        assert!(Version::from_str("21.0").unwrap() < Version::from_str("21.0.0").unwrap());

        // Build number ordering
        assert!(Version::from_str("21.0.5+9").unwrap() < Version::from_str("21.0.5+10").unwrap());
        assert!(Version::from_str("21.0.5").unwrap() < Version::from_str("21.0.5+1").unwrap());
    }

    #[test]
    fn test_semeru_and_other_formats() {
        // IBM Semeru format
        let v = Version::from_str("21.0.5+11.0.572").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, Some(vec![11, 0, 572]));

        // Temurin standard format
        let v = Version::from_str("21.0.5+11").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, Some(vec![11]));

        // Zulu format with build
        let v = Version::from_str("21.0.5+11.0.25").unwrap();
        assert_eq!(v.components, vec![21, 0, 5]);
        assert_eq!(v.build, Some(vec![11, 0, 25]));
    }

    #[test]
    fn test_format_version_minimal() {
        // Test major only
        let v1 = Version::new(21, 0, 0);
        assert_eq!(format_version_minimal(&v1), "21");

        // Test major.minor
        let v2 = Version::new(17, 1, 0);
        assert_eq!(format_version_minimal(&v2), "17.1");

        // Test full version
        let v3 = Version::new(11, 0, 21);
        assert_eq!(format_version_minimal(&v3), "11.0.21");
    }

    #[test]
    fn test_validate_version_for_command() {
        let version = Some(Version::new(21, 0, 0));
        let result = validate_version_for_command(&version, "test");
        assert!(result.is_ok());

        let none_version: Option<Version> = None;
        let result = validate_version_for_command(&none_version, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_install_request() {
        use crate::models::distribution::Distribution;

        let dist = Distribution::Temurin;
        let version = Version::new(21, 0, 0);
        let request = build_install_request(&dist, &version);

        assert_eq!(request.distribution, Some("temurin".to_string()));
        assert_eq!(request.version_pattern, "21.0.0");
    }
}
