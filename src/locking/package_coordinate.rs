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
use crate::models::api::Package;
use std::fmt;

/// Represents the type of Java package being coordinated for locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    Jdk,
    Jre,
}

impl PackageKind {
    fn slug_segment(self) -> &'static str {
        match self {
            PackageKind::Jdk => "jdk",
            PackageKind::Jre => "jre",
        }
    }

    pub fn try_from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "jdk" => Ok(PackageKind::Jdk),
            "jre" => Ok(PackageKind::Jre),
            other => Err(KopiError::ValidationError(format!(
                "Unsupported package type '{other}'"
            ))),
        }
    }
}

impl fmt::Display for PackageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.slug_segment())
    }
}

/// Coordinate that uniquely identifies a package for lock scoping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCoordinate {
    distribution: String,
    major_version: u32,
    package_kind: PackageKind,
    architecture: Option<String>,
    operating_system: Option<String>,
    libc_variant: Option<String>,
    javafx_bundled: bool,
    variant_tags: Vec<String>,
}

impl PackageCoordinate {
    /// Creates a new coordinate with the required fields.
    pub fn new<S: Into<String>>(
        distribution: S,
        major_version: u32,
        package_kind: PackageKind,
    ) -> Self {
        Self {
            distribution: distribution.into(),
            major_version,
            package_kind,
            architecture: None,
            operating_system: None,
            libc_variant: None,
            javafx_bundled: false,
            variant_tags: Vec::new(),
        }
    }

    pub fn distribution(&self) -> &str {
        &self.distribution
    }

    pub fn major_version(&self) -> u32 {
        self.major_version
    }

    pub fn package_kind(&self) -> PackageKind {
        self.package_kind
    }

    pub fn architecture(&self) -> Option<&str> {
        self.architecture.as_deref()
    }

    pub fn operating_system(&self) -> Option<&str> {
        self.operating_system.as_deref()
    }

    pub fn libc_variant(&self) -> Option<&str> {
        self.libc_variant.as_deref()
    }

    pub fn variant_tags(&self) -> &[String] {
        &self.variant_tags
    }

    pub fn javafx_bundled(&self) -> bool {
        self.javafx_bundled
    }

    pub fn with_architecture<S: Into<String>>(mut self, architecture: Option<S>) -> Self {
        self.architecture = architecture.map(|value| value.into());
        self
    }

    pub fn with_operating_system<S: Into<String>>(mut self, operating_system: Option<S>) -> Self {
        self.operating_system = operating_system.map(|value| value.into());
        self
    }

    pub fn with_libc_variant<S: Into<String>>(mut self, libc_variant: Option<S>) -> Self {
        self.libc_variant = libc_variant.map(|value| value.into());
        self
    }

    pub fn with_javafx(mut self, javafx_bundled: bool) -> Self {
        self.javafx_bundled = javafx_bundled;
        self
    }

    pub fn with_variant_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.variant_tags = tags.into_iter().map(|tag| tag.into()).collect();
        self
    }

    /// Generates a deterministic slug suitable for filesystem lock names.
    pub fn slug(&self) -> String {
        let mut segments = Vec::new();

        if let Some(segment) = sanitize_segment(&self.distribution) {
            segments.push(segment);
        }

        segments.push(self.major_version.to_string());
        segments.push(self.package_kind.slug_segment().to_string());

        if let Some(architecture) = self
            .architecture
            .as_ref()
            .and_then(|value| sanitize_segment(value))
        {
            segments.push(architecture);
        }

        if let Some(os) = self
            .operating_system
            .as_ref()
            .and_then(|value| sanitize_segment(value))
        {
            segments.push(os);
        }

        if let Some(libc) = self
            .libc_variant
            .as_ref()
            .and_then(|value| sanitize_segment(value))
        {
            segments.push(libc);
        }

        let mut extras: Vec<String> = self
            .variant_tags
            .iter()
            .filter_map(|value| sanitize_segment(value))
            .collect();
        extras.sort();
        extras.dedup();
        segments.extend(extras);

        if self.javafx_bundled {
            segments.push("javafx".to_string());
        }

        segments.join("-")
    }

    /// Attempts to build a coordinate from a metadata package definition.
    pub fn try_from_package(package: &Package) -> Result<Self> {
        let kind = PackageKind::try_from_str(&package.package_type)?;
        let variants = build_variant_tags(package);

        Ok(
            Self::new(package.distribution.clone(), package.major_version, kind)
                .with_architecture(package.architecture.clone())
                .with_operating_system(Some(package.operating_system.clone()))
                .with_libc_variant(package.lib_c_type.clone())
                .with_javafx(package.javafx_bundled)
                .with_variant_tags(variants),
        )
    }
}

fn build_variant_tags(package: &Package) -> Vec<String> {
    let mut tags = Vec::new();

    if let Some(term) = &package.term_of_support {
        tags.push(term.clone());
    }

    if let Some(status) = &package.release_status {
        tags.push(status.clone());
    }

    if package.latest_build_available.unwrap_or(false) {
        tags.push("latest".to_string());
    }

    tags
}

pub(crate) fn sanitize_segment(value: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_package() -> Package {
        Package {
            id: "pkg-id".to_string(),
            archive_type: "tar.gz".to_string(),
            distribution: "Temurin".to_string(),
            major_version: 21,
            java_version: "21.0.2".to_string(),
            distribution_version: "21.0.2".to_string(),
            jdk_version: 21,
            directly_downloadable: true,
            filename: "openjdk.tar.gz".to_string(),
            links: crate::models::api::Links {
                pkg_download_redirect: "https://example.com".to_string(),
                pkg_info_uri: Some("https://example.com/info".to_string()),
            },
            free_use_in_production: true,
            tck_tested: "yes".to_string(),
            size: 1024,
            operating_system: "linux".to_string(),
            architecture: Some("x64".to_string()),
            lib_c_type: Some("gnu".to_string()),
            package_type: "JDK".to_string(),
            javafx_bundled: true,
            term_of_support: Some("lts".to_string()),
            release_status: Some("ga".to_string()),
            latest_build_available: Some(true),
        }
    }

    #[test]
    fn slug_includes_expected_segments() {
        let coordinate = PackageCoordinate::new("Temurin", 21, PackageKind::Jdk)
            .with_architecture(Some("x64"))
            .with_javafx(true);

        assert_eq!(coordinate.slug(), "temurin-21-jdk-x64-javafx");
    }

    #[test]
    fn slug_is_deterministic_with_variants() {
        let coordinate = PackageCoordinate::new("Temurin", 21, PackageKind::Jdk)
            .with_architecture(Some("x64"))
            .with_operating_system(Some("Linux"))
            .with_libc_variant(Some("gnu"))
            .with_variant_tags(["ga", "lts", "ga"])
            .with_javafx(false);

        assert_eq!(coordinate.slug(), "temurin-21-jdk-x64-linux-gnu-ga-lts");
    }

    #[test]
    fn try_from_package_populates_fields() {
        let package = sample_package();
        let coordinate = PackageCoordinate::try_from_package(&package).unwrap();

        assert_eq!(coordinate.distribution(), "Temurin");
        assert_eq!(coordinate.major_version(), 21);
        assert_eq!(coordinate.package_kind(), PackageKind::Jdk);
        assert_eq!(coordinate.architecture(), Some("x64"));
        assert_eq!(coordinate.operating_system(), Some("linux"));
        assert_eq!(coordinate.libc_variant(), Some("gnu"));
        assert!(coordinate.javafx_bundled());
        assert!(coordinate.variant_tags().iter().any(|tag| tag == "lts"));
    }

    #[test]
    fn sanitize_segment_removes_duplicates_and_case() {
        assert_eq!(sanitize_segment(" Tem urin "), Some("tem-urin".to_string()));
        assert_eq!(sanitize_segment("***"), None);
    }
}
