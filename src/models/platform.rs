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

#[cfg(test)]
mod tests {
    use super::*;

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
}
