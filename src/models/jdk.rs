use crate::error::{KopiError, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JdkMetadata {
    pub id: String,
    pub distribution: String,
    pub version: Version,
    pub distribution_version: String,
    pub architecture: Architecture,
    pub operating_system: OperatingSystem,
    pub package_type: PackageType,
    pub archive_type: ArchiveType,
    pub download_url: String,
    pub checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
    pub size: u64,
    pub lib_c_type: Option<String>,
    pub javafx_bundled: bool,
    pub term_of_support: Option<String>,
    pub release_status: Option<String>,
    pub latest_build_available: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
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

    pub fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(pattern_version) = Version::from_str(pattern) {
            if pattern_version.minor == 0
                && pattern_version.patch == 0
                && pattern_version.build.is_none()
            {
                self.major == pattern_version.major
            } else {
                self.major == pattern_version.major
                    && self.minor == pattern_version.minor
                    && (pattern_version.patch == 0 || self.patch == pattern_version.patch)
            }
        } else {
            false
        }
    }

    /// Flexibly matches a version string against this version.
    /// Treats "21", "21.0", and "21.0.0" as equivalent when comparing.
    /// Also handles versions with build numbers like "21+37" or "21.0.0+37".
    pub fn matches_version_string(&self, version_str: &str) -> bool {
        // Try to parse the version string
        if let Ok(other) = Version::from_str(version_str) {
            // First check if build number was specified and if it matches
            let build_matches = if version_str.contains('+') {
                // Build was specified in the query, must match exactly
                self.build == other.build
            } else {
                // Build was not specified in the query, ignore it
                true
            };

            if !build_matches {
                return false;
            }

            // Now check version components based on what was specified
            if version_str.contains('.') {
                // Count the dots to determine what was explicitly specified
                let dot_count = version_str
                    .split('+')
                    .next()
                    .unwrap_or("")
                    .matches('.')
                    .count();

                match dot_count {
                    0 => {
                        // Only major version specified (shouldn't happen if contains '.')
                        self.major == other.major
                    }
                    1 => {
                        // Major and minor specified (e.g., "21.0" or "21.0+37")
                        self.major == other.major && self.minor == other.minor
                    }
                    _ => {
                        // Full version specified (e.g., "21.0.0" or "21.0.0+37")
                        self.major == other.major
                            && self.minor == other.minor
                            && self.patch == other.patch
                    }
                }
            } else {
                // No dots, just major version (e.g., "21" or "21+37")
                self.major == other.major
            }
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

        let minor = components
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        let patch = components
            .get(2)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        let mut version = Version::new(major, minor, patch);
        if let Some(build) = build_part {
            version = version.with_build(build);
        }

        Ok(version)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    X64,
    X86,
    Aarch64,
    Arm32,
    Ppc64,
    Ppc64le,
    S390x,
    Sparcv9,
}

impl FromStr for Architecture {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "x64" | "amd64" | "x86_64" => Ok(Architecture::X64),
            "x86" | "i386" | "i686" => Ok(Architecture::X86),
            "aarch64" | "arm64" => Ok(Architecture::Aarch64),
            "arm32" | "arm" => Ok(Architecture::Arm32),
            "ppc64" => Ok(Architecture::Ppc64),
            "ppc64le" => Ok(Architecture::Ppc64le),
            "s390x" => Ok(Architecture::S390x),
            "sparcv9" => Ok(Architecture::Sparcv9),
            _ => Err(KopiError::InvalidConfig(format!(
                "Unknown architecture: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arch = match self {
            Architecture::X64 => "x64",
            Architecture::X86 => "x86",
            Architecture::Aarch64 => "aarch64",
            Architecture::Arm32 => "arm32",
            Architecture::Ppc64 => "ppc64",
            Architecture::Ppc64le => "ppc64le",
            Architecture::S390x => "s390x",
            Architecture::Sparcv9 => "sparcv9",
        };
        write!(f, "{arch}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingSystem {
    Linux,
    Windows,
    MacOS,
    Alpine,
    Solaris,
    Aix,
}

impl FromStr for OperatingSystem {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "linux" => Ok(OperatingSystem::Linux),
            "windows" | "win" => Ok(OperatingSystem::Windows),
            "macos" | "mac" | "darwin" => Ok(OperatingSystem::MacOS),
            "alpine" | "alpine-linux" => Ok(OperatingSystem::Alpine),
            "solaris" => Ok(OperatingSystem::Solaris),
            "aix" => Ok(OperatingSystem::Aix),
            _ => Err(KopiError::InvalidConfig(format!(
                "Unknown operating system: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let os = match self {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Alpine => "alpine",
            OperatingSystem::Solaris => "solaris",
            OperatingSystem::Aix => "aix",
        };
        write!(f, "{os}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Jdk,
    Jre,
}

impl FromStr for PackageType {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "jdk" => Ok(PackageType::Jdk),
            "jre" => Ok(PackageType::Jre),
            _ => Err(KopiError::InvalidConfig(format!(
                "Unknown package type: {s}"
            ))),
        }
    }
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pkg = match self {
            PackageType::Jdk => "jdk",
            PackageType::Jre => "jre",
        };
        write!(f, "{pkg}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveType {
    TarGz,
    Zip,
    Dmg,
    Msi,
    Exe,
    Deb,
    Rpm,
}

impl FromStr for ArchiveType {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tar.gz" | "tgz" => Ok(ArchiveType::TarGz),
            "zip" => Ok(ArchiveType::Zip),
            "dmg" => Ok(ArchiveType::Dmg),
            "msi" => Ok(ArchiveType::Msi),
            "exe" => Ok(ArchiveType::Exe),
            "deb" => Ok(ArchiveType::Deb),
            "rpm" => Ok(ArchiveType::Rpm),
            _ => Err(KopiError::InvalidConfig(format!(
                "Unknown archive type: {s}"
            ))),
        }
    }
}

impl ArchiveType {
    pub fn extension(&self) -> &str {
        match self {
            ArchiveType::TarGz => "tar.gz",
            ArchiveType::Zip => "zip",
            ArchiveType::Dmg => "dmg",
            ArchiveType::Msi => "msi",
            ArchiveType::Exe => "exe",
            ArchiveType::Deb => "deb",
            ArchiveType::Rpm => "rpm",
        }
    }
}

impl std::fmt::Display for ArchiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumType {
    Sha256,
    Sha512,
    Md5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequest {
    pub version_pattern: String,
    pub distribution: Option<String>,
}

impl VersionRequest {
    pub fn new(version_pattern: String) -> Self {
        Self {
            version_pattern,
            distribution: None,
        }
    }

    pub fn with_distribution(mut self, distribution: String) -> Self {
        self.distribution = Some(distribution);
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    Temurin,
    Corretto,
    Zulu,
    OpenJdk,
    GraalVm,
    Dragonwell,
    SapMachine,
    Liberica,
    Mandrel,
    Kona,
    Semeru,
    Trava,
    Other(String),
}

impl Distribution {
    pub fn id(&self) -> &str {
        match self {
            Distribution::Temurin => "temurin",
            Distribution::Corretto => "corretto",
            Distribution::Zulu => "zulu",
            Distribution::OpenJdk => "openjdk",
            Distribution::GraalVm => "graalvm",
            Distribution::Dragonwell => "dragonwell",
            Distribution::SapMachine => "sapmachine",
            Distribution::Liberica => "liberica",
            Distribution::Mandrel => "mandrel",
            Distribution::Kona => "kona",
            Distribution::Semeru => "semeru",
            Distribution::Trava => "trava",
            Distribution::Other(name) => name,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Distribution::Temurin => "Eclipse Temurin",
            Distribution::Corretto => "Amazon Corretto",
            Distribution::Zulu => "Azul Zulu",
            Distribution::OpenJdk => "OpenJDK",
            Distribution::GraalVm => "GraalVM",
            Distribution::Dragonwell => "Alibaba Dragonwell",
            Distribution::SapMachine => "SAP Machine",
            Distribution::Liberica => "BellSoft Liberica",
            Distribution::Mandrel => "Red Hat Mandrel",
            Distribution::Kona => "Tencent Kona",
            Distribution::Semeru => "IBM Semeru",
            Distribution::Trava => "Trava OpenJDK",
            Distribution::Other(name) => name,
        }
    }

    /// Returns the default distribution API parameter.
    /// Eclipse Temurin is used as the default distribution.
    pub fn default_distribution() -> &'static str {
        "temurin"
    }
}

impl FromStr for Distribution {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "temurin" => Ok(Distribution::Temurin),
            "corretto" => Ok(Distribution::Corretto),
            "zulu" => Ok(Distribution::Zulu),
            "openjdk" => Ok(Distribution::OpenJdk),
            "graalvm" => Ok(Distribution::GraalVm),
            "dragonwell" => Ok(Distribution::Dragonwell),
            "sapmachine" => Ok(Distribution::SapMachine),
            "liberica" => Ok(Distribution::Liberica),
            "mandrel" => Ok(Distribution::Mandrel),
            "kona" => Ok(Distribution::Kona),
            "semeru" => Ok(Distribution::Semeru),
            "trava" => Ok(Distribution::Trava),
            other => Ok(Distribution::Other(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(Version::from_str("21").unwrap(), Version::new(21, 0, 0));

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
        let v21_0_1 = Version::new(21, 0, 1);
        assert!(v21_0_1.matches_pattern("21"));
        assert!(!v21_0_1.matches_pattern("17"));

        let v17_0_9 = Version::new(17, 0, 9);
        assert!(v17_0_9.matches_pattern("17"));
        assert!(v17_0_9.matches_pattern("17.0"));
        assert!(v17_0_9.matches_pattern("17.0.9"));
        assert!(!v17_0_9.matches_pattern("17.0.8"));
    }

    #[test]
    fn test_matches_version_string() {
        // Test flexible matching with major version only
        let v21_0_0_build = Version::new(21, 0, 0).with_build("37".to_string());
        assert!(v21_0_0_build.matches_version_string("21"));
        assert!(v21_0_0_build.matches_version_string("21.0"));
        assert!(v21_0_0_build.matches_version_string("21.0.0"));
        assert!(v21_0_0_build.matches_version_string("21.0.0+37"));
        assert!(!v21_0_0_build.matches_version_string("21.0.0+38"));
        assert!(!v21_0_0_build.matches_version_string("22"));

        // Test with non-zero minor/patch
        let v21_0_7_build = Version::new(21, 0, 7).with_build("9".to_string());
        assert!(v21_0_7_build.matches_version_string("21"));
        assert!(v21_0_7_build.matches_version_string("21.0"));
        assert!(!v21_0_7_build.matches_version_string("21.0.0"));
        assert!(v21_0_7_build.matches_version_string("21.0.7"));
        assert!(v21_0_7_build.matches_version_string("21.0.7+9"));
        assert!(!v21_0_7_build.matches_version_string("21.0.7+10"));

        // Test version without build
        let v17_0_9 = Version::new(17, 0, 9);
        assert!(v17_0_9.matches_version_string("17"));
        assert!(v17_0_9.matches_version_string("17.0"));
        assert!(v17_0_9.matches_version_string("17.0.9"));
        assert!(!v17_0_9.matches_version_string("17.0.8"));
        assert!(!v17_0_9.matches_version_string("17.1"));

        // Test major-only version with build
        let v23_build = Version::new(23, 0, 0).with_build("38".to_string());
        assert!(v23_build.matches_version_string("23"));
        assert!(v23_build.matches_version_string("23+38"));
        assert!(!v23_build.matches_version_string("23+37"));
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
    fn test_architecture_parsing() {
        assert_eq!(Architecture::from_str("x64").unwrap(), Architecture::X64);
        assert_eq!(Architecture::from_str("amd64").unwrap(), Architecture::X64);
        assert_eq!(Architecture::from_str("x86_64").unwrap(), Architecture::X64);
        assert_eq!(
            Architecture::from_str("aarch64").unwrap(),
            Architecture::Aarch64
        );
        assert_eq!(
            Architecture::from_str("arm64").unwrap(),
            Architecture::Aarch64
        );
        assert!(Architecture::from_str("invalid").is_err());
    }

    #[test]
    fn test_operating_system_parsing() {
        assert_eq!(
            OperatingSystem::from_str("linux").unwrap(),
            OperatingSystem::Linux
        );
        assert_eq!(
            OperatingSystem::from_str("windows").unwrap(),
            OperatingSystem::Windows
        );
        assert_eq!(
            OperatingSystem::from_str("macos").unwrap(),
            OperatingSystem::MacOS
        );
        assert_eq!(
            OperatingSystem::from_str("darwin").unwrap(),
            OperatingSystem::MacOS
        );
        assert!(OperatingSystem::from_str("invalid").is_err());
    }

    #[test]
    fn test_archive_type_parsing() {
        assert_eq!(ArchiveType::from_str("tar.gz").unwrap(), ArchiveType::TarGz);
        assert_eq!(ArchiveType::from_str("tgz").unwrap(), ArchiveType::TarGz);
        assert_eq!(ArchiveType::from_str("zip").unwrap(), ArchiveType::Zip);
        assert!(ArchiveType::from_str("invalid").is_err());
    }

    #[test]
    fn test_default_distribution() {
        assert_eq!(Distribution::default_distribution(), "temurin");
    }
}
