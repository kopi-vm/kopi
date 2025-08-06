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

    /// Returns a list of all known distribution IDs
    pub fn known_distributions() -> Vec<&'static str> {
        vec![
            "temurin",
            "corretto",
            "zulu",
            "openjdk",
            "graalvm",
            "dragonwell",
            "sapmachine",
            "liberica",
            "mandrel",
            "kona",
            "semeru",
            "trava",
        ]
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
    fn test_default_distribution() {
        assert_eq!(Distribution::default_distribution(), "temurin");
    }
}
