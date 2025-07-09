use crate::error::{KopiError, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
    Sha1,
    Sha256,
    Sha512,
    Md5,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_type_parsing() {
        assert_eq!(ArchiveType::from_str("tar.gz").unwrap(), ArchiveType::TarGz);
        assert_eq!(ArchiveType::from_str("tgz").unwrap(), ArchiveType::TarGz);
        assert_eq!(ArchiveType::from_str("zip").unwrap(), ArchiveType::Zip);
        assert!(ArchiveType::from_str("invalid").is_err());
    }

    #[test]
    fn test_checksum_type_serialization() {
        // Test serialization of all checksum types
        assert_eq!(
            serde_json::to_string(&ChecksumType::Sha1).unwrap(),
            "\"sha1\""
        );
        assert_eq!(
            serde_json::to_string(&ChecksumType::Sha256).unwrap(),
            "\"sha256\""
        );
        assert_eq!(
            serde_json::to_string(&ChecksumType::Sha512).unwrap(),
            "\"sha512\""
        );
        assert_eq!(
            serde_json::to_string(&ChecksumType::Md5).unwrap(),
            "\"md5\""
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<ChecksumType>("\"sha1\"").unwrap(),
            ChecksumType::Sha1
        );
        assert_eq!(
            serde_json::from_str::<ChecksumType>("\"sha256\"").unwrap(),
            ChecksumType::Sha256
        );
        assert_eq!(
            serde_json::from_str::<ChecksumType>("\"sha512\"").unwrap(),
            ChecksumType::Sha512
        );
        assert_eq!(
            serde_json::from_str::<ChecksumType>("\"md5\"").unwrap(),
            ChecksumType::Md5
        );
    }
}
