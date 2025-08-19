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

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::distribution::Distribution;
use crate::models::package::PackageType;
use crate::version::Version;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedVersionRequest {
    pub version: Option<Version>,
    pub distribution: Option<Distribution>,
    pub package_type: Option<PackageType>,
    pub latest: bool,
    pub javafx_bundled: Option<bool>,
}

pub struct VersionParser<'a> {
    config: &'a KopiConfig,
}

impl<'a> VersionParser<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    pub fn parse(&self, input: &str) -> Result<ParsedVersionRequest> {
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
            (Some(PackageType::Jdk), trimmed)
        };

        // Check for JavaFX suffix (+fx at the end)
        let (javafx_bundled, remaining) = if let Some(stripped) = remaining.strip_suffix("+fx") {
            (Some(true), stripped)
        } else {
            (None, remaining)
        };

        // Check for "latest" keyword
        if remaining.eq_ignore_ascii_case("latest") {
            return Ok(ParsedVersionRequest {
                version: None,
                distribution: None,
                package_type,
                latest: true,
                javafx_bundled,
            });
        }

        // Now parse the remaining part as before
        let (distribution, version_str) = if remaining.contains('@') {
            // Format: distribution@version
            let mut parts = remaining.splitn(2, '@');
            let dist_part = parts.next().unwrap();
            let version_part = parts.next().unwrap_or("");

            // For the @ format, we should only accept known distributions
            if !self.is_known_distribution(dist_part) {
                return Err(KopiError::InvalidVersionFormat(format!(
                    "Unknown distribution: {dist_part}"
                )));
            }

            // Normalize distribution name to lowercase for consistency with additional_distributions config
            let normalized_dist = if self.is_default_distribution(dist_part) {
                dist_part
            } else {
                &dist_part.to_lowercase()
            };

            let dist = Distribution::from_str(normalized_dist).map_err(|_| {
                KopiError::InvalidVersionFormat(format!("Unknown distribution: {dist_part}"))
            })?;

            if version_part.is_empty() {
                // Distribution without version (e.g., "corretto")
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: false,
                    javafx_bundled,
                });
            }

            // Check if version part is "latest"
            if version_part.eq_ignore_ascii_case("latest") {
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: true,
                    javafx_bundled,
                });
            }

            (Some(dist), version_part)
        } else {
            // No @ symbol - check if it's a known distribution name
            if self.is_known_distribution(remaining) {
                // It's a distribution name without version
                // Normalize distribution name to lowercase for consistency with additional_distributions config
                let normalized_dist = if self.is_default_distribution(remaining) {
                    remaining
                } else {
                    &remaining.to_lowercase()
                };

                let dist = Distribution::from_str(normalized_dist).map_err(|_| {
                    KopiError::InvalidVersionFormat(format!("Unknown distribution: {remaining}"))
                })?;
                return Ok(ParsedVersionRequest {
                    version: None,
                    distribution: Some(dist),
                    package_type,
                    latest: false,
                    javafx_bundled,
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
            javafx_bundled,
        })
    }

    fn parse_version_string(version_str: &str) -> Result<Version> {
        // Check for version ranges (not yet implemented)
        if version_str.contains(">=") || version_str.contains("<=") || version_str.contains("><") {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Version ranges are not yet supported: {version_str}"
            )));
        }

        // Use the Version::from_str implementation which already handles build numbers and pre-release
        // The Version parser handles '+' and '-' correctly
        Version::from_str(version_str)
    }

    pub fn validate_version_semantics(version: &Version) -> Result<()> {
        // Validate reasonable version numbers
        if version.major() == 0 || version.major() > 99 {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid major version: {}. JDK versions typically range from 1 to 99.",
                version.major()
            )));
        }

        if let Some(minor) = version.minor()
            && minor > 99
        {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid minor version: {minor}. Minor versions typically range from 0 to 99."
            )));
        }

        if let Some(patch) = version.patch()
            && patch > 999
        {
            return Err(KopiError::InvalidVersionFormat(format!(
                "Invalid patch version: {patch}. Patch versions typically range from 0 to 999."
            )));
        }

        Ok(())
    }

    pub fn is_lts_version(major: u32) -> bool {
        // Known LTS versions
        matches!(major, 8 | 11 | 17 | 21)
    }

    fn is_default_distribution(&self, name: &str) -> bool {
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
                | "aoj"
                | "aoj_openj9"
                | "bisheng"
                | "gluon_graalvm"
                | "graalvm_ce8"
                | "graalvm_ce11"
                | "graalvm_ce16"
                | "graalvm_ce17"
                | "graalvm_ce19"
                | "graalvm_community"
                | "jetbrains"
                | "liberica_native"
                | "microsoft"
                | "ojdk_build"
                | "openlogic"
                | "oracle"
                | "oracle_open_jdk"
                | "sap_machine"
                | "semeru_certified"
                | "zulu_prime"
        )
    }

    fn is_known_distribution(&self, name: &str) -> bool {
        // First check if it looks like a version (starts with a digit)
        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check against default known distributions
        let is_default = self.is_default_distribution(name);

        // If it's in the default list, return true
        if is_default {
            return true;
        }

        // Check against additional distributions from config
        self.config
            .additional_distributions
            .iter()
            .any(|dist| dist.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use serial_test::serial;

    fn create_test_config() -> KopiConfig {
        // Clear any leftover environment variables
        unsafe {
            std::env::remove_var("KOPI_AUTO_INSTALL");
            std::env::remove_var("KOPI_AUTO_INSTALL__ENABLED");
        }

        // Create a test config with a temporary directory
        let temp_dir = std::env::temp_dir();
        KopiConfig::new(temp_dir).unwrap()
    }

    #[test]
    fn test_parse_version_only() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("21").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 21);
        assert_eq!(version.minor(), None);
        assert_eq!(version.patch(), None);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_version_with_minor() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("17.0.9").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 17);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(9));
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_distribution_with_version() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("corretto@21").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert!(result.version.is_some());
        assert_eq!(result.version.unwrap().major(), 21);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_distribution_with_full_version() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("temurin@17.0.9").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Temurin));
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 17);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(9));
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_invalid_distribution() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("invalid@21");
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
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("temurin").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Temurin));
        assert_eq!(result.version, None);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_empty_version() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_version_format() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("abc");
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
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("latest").unwrap();
        assert_eq!(result.distribution, None);
        assert_eq!(result.version, None);
        assert!(result.latest);
    }

    #[test]
    fn test_parse_distribution_with_latest() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("corretto@latest").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert_eq!(result.version, None);
        assert!(result.latest);
    }

    #[test]
    fn test_parse_distribution_only() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("zulu").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Zulu));
        assert_eq!(result.version, None);
        assert!(!result.latest);
    }

    #[test]
    fn test_parse_jre_latest() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("jre@latest").unwrap();
        assert_eq!(result.distribution, None);
        assert_eq!(result.version, None);
        assert_eq!(result.package_type, Some(PackageType::Jre));
        assert!(result.latest);
    }

    #[test]
    fn test_parse_jre_distribution_latest() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("jre@corretto@latest").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert_eq!(result.version, None);
        assert_eq!(result.package_type, Some(PackageType::Jre));
        assert!(result.latest);
    }

    #[test]
    fn test_parse_jdk_distribution_only() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("jdk@temurin").unwrap();
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
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        // Test JRE prefix
        let parsed = parser.parse("jre@21").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::from_components(21, None, None))
        );
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, Some(PackageType::Jre));

        // Test JDK prefix (explicit)
        let parsed = parser.parse("jdk@21").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::from_components(21, None, None))
        );
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, Some(PackageType::Jdk));

        // Test JRE with distribution
        let parsed = parser.parse("jre@temurin@21").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::from_components(21, None, None))
        );
        assert_eq!(parsed.distribution, Some(Distribution::Temurin));
        assert_eq!(parsed.package_type, Some(PackageType::Jre));

        // Test JDK with distribution (explicit)
        let parsed = parser.parse("jdk@temurin@21").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::from_components(21, None, None))
        );
        assert_eq!(parsed.distribution, Some(Distribution::Temurin));
        assert_eq!(parsed.package_type, Some(PackageType::Jdk));

        // Test no prefix (defaults to JDK)
        let parsed = parser.parse("21").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::from_components(21, None, None))
        );
        assert_eq!(parsed.distribution, None);
        assert_eq!(parsed.package_type, Some(PackageType::Jdk)); // Defaults to JDK

        // Test with full version
        let parsed = parser.parse("jre@21.0.1+12").unwrap();
        assert_eq!(
            parsed.version,
            Some(Version::new(21, 0, 1).with_build("12".to_string()))
        );
        assert_eq!(parsed.package_type, Some(PackageType::Jre));
    }

    #[test]
    fn test_version_ranges_not_supported() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse(">=17");
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
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("17.0.9+7").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 17);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(9));
        assert_eq!(version.build, Some(vec![7]));
    }

    #[test]
    fn test_parse_version_with_pre_release() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("21.0.1-ea").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 21);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(1));
        assert_eq!(version.pre_release, Some("ea".to_string()));
    }

    #[test]
    fn test_parse_version_with_build_and_pre_release() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("17.0.2+8-LTS").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 17);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(2));
        assert_eq!(version.pre_release, Some("8-LTS".to_string()));
    }

    #[test]
    fn test_parse_distribution_with_complex_version() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("corretto@21.0.1-amzn").unwrap();
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 21);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(1));
        assert_eq!(version.pre_release, Some("amzn".to_string()));
    }

    #[test]
    fn test_parse_version_with_complex_build() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);
        let result = parser.parse("11.0.21+9-LTS-3299655").unwrap();
        assert_eq!(result.distribution, None);
        assert!(result.version.is_some());
        let version = result.version.unwrap();
        assert_eq!(version.major(), 11);
        assert_eq!(version.minor(), Some(0));
        assert_eq!(version.patch(), Some(21));
        assert_eq!(version.pre_release, Some("9-LTS-3299655".to_string()));
    }

    #[test]
    #[serial]
    fn test_additional_distributions() {
        use crate::config::new_kopi_config;
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary config with additional distributions
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("KOPI_HOME", temp_dir.path());
        }

        let config_content = r#"
default_distribution = "temurin"
additional_distributions = ["mycustom", "private-jdk", "company-build"]
"#;
        fs::write(temp_dir.path().join("config.toml"), config_content).unwrap();

        let config = new_kopi_config().unwrap();
        let parser = VersionParser::new(&config);

        // Test that custom distributions are recognized
        let result = parser.parse("mycustom@21").unwrap();
        assert_eq!(
            result.distribution,
            Some(Distribution::Other("mycustom".to_string()))
        );
        assert_eq!(result.version.unwrap().major(), 21);

        let result = parser.parse("private-jdk").unwrap();
        assert_eq!(
            result.distribution,
            Some(Distribution::Other("private-jdk".to_string()))
        );
        assert_eq!(result.version, None);

        // Test case insensitive matching
        let result = parser.parse("COMPANY-BUILD@17.0.1").unwrap();
        // The parser normalizes additional distributions to lowercase
        assert_eq!(
            result.distribution,
            Some(Distribution::Other("company-build".to_string()))
        );
        assert_eq!(result.version.unwrap().major(), 17);

        // Test that unknown distributions still fail
        let result = parser.parse("unknown-dist@21");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown distribution")
        );

        // Clean up
        unsafe {
            std::env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_parse_with_javafx() {
        let config = create_test_config();
        let parser = VersionParser::new(&config);

        // Test version with JavaFX
        let result = parser.parse("21+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.version.unwrap().to_string(), "21");

        // Test distribution@version with JavaFX
        let result = parser.parse("liberica@21+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.distribution, Some(Distribution::Liberica));
        assert_eq!(result.version.unwrap().to_string(), "21");

        // Test full version with JavaFX
        let result = parser.parse("zulu@21.0.5+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.distribution, Some(Distribution::Zulu));
        assert_eq!(result.version.unwrap().to_string(), "21.0.5");

        // Test version with build number and JavaFX
        let result = parser.parse("corretto@21.0.5+11+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.distribution, Some(Distribution::Corretto));
        let version = result.version.unwrap();
        assert_eq!(version.components, vec![21, 0, 5]);
        assert_eq!(version.build, Some(vec![11]));

        // Test JRE with JavaFX
        let result = parser.parse("jre@liberica@21+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.package_type, Some(PackageType::Jre));
        assert_eq!(result.distribution, Some(Distribution::Liberica));

        // Test latest with JavaFX
        let result = parser.parse("latest+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert!(result.latest);

        // Test distribution@latest with JavaFX
        let result = parser.parse("liberica@latest+fx").unwrap();
        assert_eq!(result.javafx_bundled, Some(true));
        assert_eq!(result.distribution, Some(Distribution::Liberica));
        assert!(result.latest);

        // Test without JavaFX (should be None)
        let result = parser.parse("21").unwrap();
        assert_eq!(result.javafx_bundled, None);
    }
}
