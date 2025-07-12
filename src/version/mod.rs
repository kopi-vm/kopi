use crate::error::{KopiError, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(pattern_version) = Version::from_str(pattern) {
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
        } else {
            false
        }
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
}

impl VersionRequest {
    pub fn new(version_pattern: String) -> Self {
        Self {
            version_pattern,
            distribution: None,
            package_type: None,
        }
    }

    pub fn with_distribution(mut self, distribution: String) -> Self {
        self.distribution = Some(distribution);
        self
    }

    pub fn with_package_type(mut self, package_type: crate::models::package::PackageType) -> Self {
        self.package_type = Some(package_type);
        self
    }
}

impl FromStr for VersionRequest {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        if s.contains('@') {
            let parts: Vec<&str> = s.split('@').collect();
            if parts.len() != 2 {
                return Err(KopiError::InvalidVersionFormat(s.to_string()));
            }
            Ok(VersionRequest::new(parts[1].to_string()).with_distribution(parts[0].to_string()))
        } else {
            Ok(VersionRequest::new(s.to_string()))
        }
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

        let req = VersionRequest::from_str("corretto@17").unwrap();
        assert_eq!(req.version_pattern, "17");
        assert_eq!(req.distribution, Some("corretto".to_string()));

        assert!(VersionRequest::from_str("invalid@format@").is_err());
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
}
