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
use crate::models::platform::{Architecture, OperatingSystem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Platform specification for filtering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Platform {
    pub os: OperatingSystem,
    pub arch: Architecture,
    pub libc: Option<String>,
}

// Create a hashable key for platform
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformKey {
    pub os: String,
    pub arch: String,
    pub libc: Option<String>,
}

impl From<&Platform> for PlatformKey {
    fn from(p: &Platform) -> Self {
        PlatformKey {
            os: p.os.to_string(),
            arch: p.arch.to_string(),
            libc: p.libc.clone(),
        }
    }
}

impl FromStr for Platform {
    type Err = KopiError;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() < 2 {
            return Err(KopiError::InvalidConfig(format!(
                "Invalid platform format: {s}. Expected: os-arch[-libc]"
            )));
        }

        let os = OperatingSystem::from_str(parts[0])?;
        let arch = Architecture::from_str(parts[1])?;
        let libc = if parts.len() > 2 {
            Some(parts[2].to_string())
        } else {
            None
        };

        Ok(Platform { os, arch, libc })
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(libc) = &self.libc {
            write!(f, "{}-{}-{}", self.os, self.arch, libc)
        } else {
            write!(f, "{}-{}", self.os, self.arch)
        }
    }
}

/// Configuration for metadata generator
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratorConfig {
    pub distributions: Option<Vec<String>>,
    pub platforms: Option<Vec<Platform>>,
    pub javafx_bundled: bool,
    pub parallel_requests: usize,
    #[serde(skip)]
    pub dry_run: bool,
    pub minify_json: bool,
    #[serde(skip)]
    pub force: bool,
}

/// Metadata for a file to be written
pub struct FileMetadata {
    pub distribution: String,
    pub os: String,
    pub architecture: String,
    pub libc: Option<String>,
    pub content: String,
}

/// Information about a JDK update
#[derive(Debug)]
pub struct JdkUpdateInfo {
    pub _id: String,
    pub distribution: String,
    pub version: String,
    pub architecture: String,
    pub update_type: UpdateType,
    pub changes: Vec<String>,
}

/// Type of update for a JDK
#[derive(Debug, PartialEq)]
pub enum UpdateType {
    New,
    Modified,
}

/// State of a file being generated
#[derive(Serialize, Deserialize, Debug)]
pub struct FileState {
    pub status: FileStatus,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempts: u32,
    pub error: Option<String>,
    pub checksum: Option<String>,
}

/// Status of file generation
#[derive(Serialize, Deserialize, Debug)]
pub enum FileStatus {
    InProgress,
    Completed,
    Failed,
}
