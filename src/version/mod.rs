use crate::error::{KopiError, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor: Some(minor),
            patch: Some(patch),
            build: None,
        }
    }

    pub fn from_components(major: u32, minor: Option<u32>, patch: Option<u32>) -> Self {
        Self {
            major,
            minor,
            patch,
            build: None,
        }
    }

    pub fn with_build(mut self, build: String) -> Self {
        self.build = Some(build);
        self
    }

    fn matches_optional_component(
        pattern_component: Option<u32>,
        self_component: Option<u32>,
    ) -> bool {
        match (pattern_component, self_component) {
            (Some(pattern_val), Some(self_val)) => pattern_val == self_val,
            (Some(_), None) => false, // Pattern specifies value but self doesn't have it
            (None, _) => true,        // Pattern doesn't specify value, matches any
        }
    }

    /// Matches a version string against this version.
    /// When the user specifies "21", it matches cache entries like "21.0" and "21.0.0".
    /// When the user specifies "21.0.0", it does NOT match cache entries like "21".
    /// When the user specifies "21.0", it matches cache entries like "21.0.0" and "21.0+32".
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(pattern_version) = Version::from_str(pattern) {
            // Major version must always match
            if self.major != pattern_version.major {
                return false;
            }

            // Check minor and patch using the common logic
            if !Self::matches_optional_component(pattern_version.minor, self.minor) {
                return false;
            }

            if !Self::matches_optional_component(pattern_version.patch, self.patch) {
                return false;
            }

            // Build matching if specified
            if pattern_version.build.is_some() && pattern_version.build != self.build {
                return false;
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
        let parts: Vec<&str> = s.split('+').collect();
        let version_part = parts[0];
        let build_part = parts.get(1).map(|&s| s.to_string());

        let components: Vec<&str> = version_part.split('.').collect();

        if components.is_empty() || components.len() > 3 {
            return Err(KopiError::InvalidVersionFormat(s.to_string()));
        }

        let major = components[0]
            .parse::<u32>()
            .map_err(|_| KopiError::InvalidVersionFormat(s.to_string()))?;

        let minor = components.get(1).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<u32>().ok()
            }
        });

        let patch = components.get(2).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<u32>().ok()
            }
        });

        let mut version = Version::from_components(major, minor, patch);
        if let Some(build) = build_part {
            version = version.with_build(build);
        }

        Ok(version)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.major)?;

        if let Some(minor) = self.minor {
            write!(f, ".{minor}")?;

            if let Some(patch) = self.patch {
                write!(f, ".{patch}")?;
            }
        }

        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
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

        assert_eq!(
            Version::from_str("11.0.2+9").unwrap(),
            Version::new(11, 0, 2).with_build("9".to_string())
        );

        assert!(Version::from_str("invalid").is_err());
        assert!(Version::from_str("1.2.3.4").is_err());
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
        assert_eq!(
            Version::new(11, 0, 2)
                .with_build("9".to_string())
                .to_string(),
            "11.0.2+9"
        );
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

        // Test that cache entries with None don't match specific user values
        let v21 = Version::from_components(21, None, None);
        assert!(v21.matches_pattern("21"));
        assert!(!v21.matches_pattern("21.0")); // User specifies 21.0, cache has only 21
        assert!(!v21.matches_pattern("21.0.0")); // User specifies 21.0.0, cache has only 21

        let v21_0 = Version::from_components(21, Some(0), None);
        assert!(v21_0.matches_pattern("21")); // User specifies 21, matches 21.0
        assert!(v21_0.matches_pattern("21.0")); // User specifies 21.0, matches 21.0
        assert!(!v21_0.matches_pattern("21.0.0")); // User specifies 21.0.0, cache has only 21.0
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
}
