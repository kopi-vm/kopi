use crate::error::{KopiError, Result};
use crate::models::jdk::{Distribution, PackageType, Version};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedVersionRequest {
    pub version: Option<Version>,
    pub distribution: Option<Distribution>,
    pub package_type: Option<PackageType>,
    pub latest: bool,
}

pub struct VersionParser;

impl VersionParser {
    pub fn parse(input: &str) -> Result<ParsedVersionRequest> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(KopiError::InvalidVersionFormat(
                "Version string cannot be empty".to_string(),
            ));
        }

        // Check for package type prefix (jre@ or jdk@)
        let (package_type, remaining) = if let Some(spec) = trimmed.strip_prefix("jre@") {
            (Some(PackageType::Jre), spec)
        } else if let Some(spec) = trimmed.strip_prefix("jdk@") {
            (Some(PackageType::Jdk), spec)
        } else {
            // Default to JDK for backward compatibility
            (None, trimmed)
        };

        // Check for "latest" keyword
        if remaining.eq_ignore_ascii_case("latest") {
            return Ok(ParsedVersionRequest {
                version: None,
                distribution: None,
                package_type,
                latest: true,
            });
        }

        // Now parse the remaining part as before
        let (distribution, version_str) = if remaining.contains('@') {
            // Format: distribution@version
            let mut parts = remaining.splitn(2, '@');
            let dist_part = parts.next().unwrap();
            let version_part = parts.next().unwrap_or("");

            // For the @ format, we should only accept known distributions
            if !Self::is_known_distribution(dist_part) {
                return Err(KopiError::InvalidVersionFormat(format!(
                    "Unknown distribution: {}",
                    dist_part
                )));
            }

            let dist = Distribution::from_str(dist_part).map_err(|_| {
                KopiError::InvalidVersionFormat(format!("Unknown distribution: {}", dist_part))
            })?;

            if version_part.is_empty() {
                // Distribution without version (e.g., "corretto")
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: false,
                });
            }

            // Check if version part is "latest"
            if version_part.eq_ignore_ascii_case("latest") {
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: true,
                });
            }

            (Some(dist), version_part)
        } else {
            // No @ symbol - check if it's a known distribution name
            if Self::is_known_distribution(remaining) {
                // It's a distribution name without version
                let dist = Distribution::from_str(remaining).map_err(|_| {
                    KopiError::InvalidVersionFormat(format!("Unknown distribution: {}", remaining))
                })?;
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: false,
                });
            } else {
                // It's a version string
                (None, remaining)
            }
        };

        // Parse version
        let version = Self::parse_version_string(version_str)?;

        Ok(ParsedVersionRequest {
            version: Some(version),
            distribution,
            package_type,
            latest: false,
        })
    }

    fn parse_version_string(version_str: &str) -> Result<Version> {
        // Check for version ranges (not yet implemented)
        if version_str.contains(">=") || version_str.contains("<=") || version_str.contains("><") {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Version ranges are not yet supported: {}",
                version_str
            )));
        }

        // Handle pre-release versions (e.g., "21.0.1-ea")
        if version_str.contains('-') {
            // Split by hyphen to separate version from pre-release
            let parts: Vec<&str> = version_str.splitn(2, '-').collect();
            let base_version = parts[0];
            let pre_release = parts.get(1).map(|s| s.to_string());

            // Parse the base version (which might contain a build number)
            let mut version = Version::from_str(base_version)?;

            // Add pre-release identifier to build field
            if let Some(pre) = pre_release {
                // If there's already a build number, combine them
                if let Some(existing_build) = version.build {
                    version.build = Some(format!("{}-{}", existing_build, pre));
                } else {
                    version.build = Some(pre);
                }
            }
            Ok(version)
        } else {
            // Use the Version::from_str implementation which already handles build numbers
            Version::from_str(version_str)
        }
    }

    pub fn validate_version_semantics(version: &Version) -> Result<()> {
        // Validate reasonable version numbers
        if version.major == 0 || version.major > 99 {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid major version: {}. JDK versions typically range from 1 to 99.",
                version.major
            )));
        }

        if version.minor > 99 {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid minor version: {}. Minor versions typically range from 0 to 99.",
                version.minor
            )));
        }

        if version.patch > 999 {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid patch version: {}. Patch versions typically range from 0 to 999.",
                version.patch
            )));
        }

        Ok(())
    }

    pub fn is_lts_version(major: u32) -> bool {
        // Known LTS versions
        matches!(major, 8 | 11 | 17 | 21)
    }

    fn is_known_distribution(name: &str) -> bool {
        matches!(
            name.to_lowercase().as_str(),
            "temurin"
                | "corretto"
                | "zulu"
                | "openjdk"
                | "graalvm"
                | "dragonwell"
                | "sapmachine"
                | "liberica"
                | "mandrel"
                | "kona"
                | "semeru"
                | "trava"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_only() {
        let result = VersionParser::parse("21").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 21);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_version_with_minor() {
        let result = VersionParser::parse("17.0.9").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 17);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 9);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_distribution_with_version() {
        let result = VersionParser::parse("corretto@21").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert!(result.version.is_some());
        assert_eq!(result.version.unwrap().major, 21);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_distribution_with_full_version() {
        let result = VersionParser::parse("temurin@17.0.9").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Temurin));
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 17);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 9);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_invalid_distribution() {
        let result = VersionParser::parse("invalid@21");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown distribution")
        );
    }

    #[test]
    fn test_parse_distribution_without_version() {
        let result = VersionParser::parse("temurin").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Temurin));
        assert_eq!(result.version, None);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_empty_version() {
        let result = VersionParser::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_version_format() {
        let result = VersionParser::parse("abc");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid version format")
        );
    }

    #[test]
    fn test_validate_version_semantics() {
        assert!(VersionParser::validate_version_semantics(&Version::new(21, 0, 0)).is_ok());
        assert!(VersionParser::validate_version_semantics(&Version::new(0, 0, 0)).is_err());
        assert!(VersionParser::validate_version_semantics(&Version::new(100, 0, 0)).is_err());
    }

    #[test]
    fn test_parse_latest_keyword() {
        let result = VersionParser::parse("latest").unwrap();
        assert_eq!(result.distribution, None);
        assert_eq!(result.version, None);
        assert!(result.latest);
    }

    #[test]
    fn test_parse_distribution_with_latest() {
        let result = VersionParser::parse("corretto@latest").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert_eq!(result.version, None);
        assert!(result.latest);
    }

    #[test]
    fn test_parse_distribution_only() {
        let result = VersionParser::parse("zulu").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Zulu));
        assert_eq!(result.version, None);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_jre_latest() {
        let result = VersionParser::parse("jre@latest").unwrap();
        assert_eq!(result.distribution, None);
        assert_eq!(result.version, None);
        assert_eq!(result.package_type, Some(PackageType::Jre));
        assert!(result.latest);
    }

    #[test]
    fn test_parse_jre_distribution_latest() {
        let result = VersionParser::parse("jre@corretto@latest").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert_eq!(result.version, None);
        assert_eq!(result.package_type, Some(PackageType::Jre));
        assert!(result.latest);
    }

    #[test]
    fn test_parse_jdk_distribution_only() {
        let result = VersionParser::parse("jdk@temurin").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Temurin));
        assert_eq!(result.version, None);
        assert_eq!(result.package_type, Some(PackageType::Jdk));
        assert!(!result.latest);
    }

    #[test]
    fn test_is_lts_version() {
        assert!(VersionParser::is_lts_version(8));
        assert!(VersionParser::is_lts_version(11));
        assert!(VersionParser::is_lts_version(17));
        assert!(VersionParser::is_lts_version(21));
        assert!(!VersionParser::is_lts_version(9));
        assert!(!VersionParser::is_lts_version(18));
    }

    #[test]
    fn test_parse_with_package_type_prefix() {
        // Test JRE prefix
        let parsed = VersionParser::parse("jre@21").unwrap();
        assert_eq!(parsed.version, Some(Version::new(21, 0, 0)));
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, Some(PackageType::Jre));

        // Test JDK prefix (explicit)
        let parsed = VersionParser::parse("jdk@21").unwrap();
        assert_eq!(parsed.version, Some(Version::new(21, 0, 0)));
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, Some(PackageType::Jdk));

        // Test JRE with distribution
        let parsed = VersionParser::parse("jre@temurin@21").unwrap();
        assert_eq!(parsed.version, Some(Version::new(21, 0, 0)));
        assert_eq!(parsed.distribution, Some(Distribution::Temurin));
        assert_eq!(parsed.package_type, Some(PackageType::Jre));

        // Test JDK with distribution (explicit)
        let parsed = VersionParser::parse("jdk@temurin@21").unwrap();
        assert_eq!(parsed.version, Some(Version::new(21, 0, 0)));
        assert_eq!(parsed.distribution, Some(Distribution::Temurin));
        assert_eq!(parsed.package_type, Some(PackageType::Jdk));

        // Test no prefix (defaults to JDK)
        let parsed = VersionParser::parse("21").unwrap();
        assert_eq!(parsed.version, Some(Version::new(21, 0, 0)));
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, None); // None means JDK by default

        // Test with full version
        let parsed = VersionParser::parse("jre@21.0.1+12").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::new(21, 0, 1).with_build("12".to_string()))
        );
        assert_eq!(parsed.package_type, Some(PackageType::Jre));
    }

    #[test]
    fn test_version_ranges_not_supported() {
        let result = VersionParser::parse(">=17");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Version ranges are not yet supported")
        );
    }

    #[test]
    fn test_parse_version_with_build_number() {
        let result = VersionParser::parse("17.0.9+7").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 17);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 9);
        assert_eq!(version.build, Some("7".to_string()));
    }

    #[test]
    fn test_parse_version_with_pre_release() {
        let result = VersionParser::parse("21.0.1-ea").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 21);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 1);
        assert_eq!(version.build, Some("ea".to_string()));
    }

    #[test]
    fn test_parse_version_with_build_and_pre_release() {
        let result = VersionParser::parse("17.0.2+8-LTS").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 17);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 2);
        assert_eq!(version.build, Some("8-LTS".to_string()));
    }

    #[test]
    fn test_parse_distribution_with_complex_version() {
        let result = VersionParser::parse("corretto@21.0.1-amzn").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 21);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 1);
        assert_eq!(version.build, Some("amzn".to_string()));
    }

    #[test]
    fn test_parse_version_with_complex_build() {
        let result = VersionParser::parse("11.0.21+9-LTS-3299655").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major, 11);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 21);
        assert_eq!(version.build, Some("9-LTS-3299655".to_string()));
    }
}
