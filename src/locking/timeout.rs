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

use std::fmt;
use std::time::Duration;

/// Represents the resolved timeout budget for lock acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockTimeoutValue {
    Finite(Duration),
    Infinite,
}

impl LockTimeoutValue {
    pub const fn from_secs(seconds: u64) -> Self {
        Self::Finite(Duration::from_secs(seconds))
    }

    pub fn as_duration(&self) -> Duration {
        match self {
            LockTimeoutValue::Finite(duration) => *duration,
            LockTimeoutValue::Infinite => Duration::MAX,
        }
    }

    pub fn is_infinite(&self) -> bool {
        matches!(self, LockTimeoutValue::Infinite)
    }
}

impl fmt::Display for LockTimeoutValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockTimeoutValue::Infinite => f.write_str("infinite"),
            LockTimeoutValue::Finite(duration) => write!(f, "{}s", duration.as_secs()),
        }
    }
}

/// Source precedence used when resolving the effective timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LockTimeoutSource {
    #[default]
    Default,
    Config,
    Environment,
    Cli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockTimeoutResolution {
    pub value: LockTimeoutValue,
    pub source: LockTimeoutSource,
}

impl fmt::Display for LockTimeoutSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            LockTimeoutSource::Default => "built-in default",
            LockTimeoutSource::Config => "configuration file",
            LockTimeoutSource::Environment => "environment variable",
            LockTimeoutSource::Cli => "CLI flag",
        };
        f.write_str(label)
    }
}

/// Error produced when parsing a timeout override fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockTimeoutParseError {
    message: String,
}

impl fmt::Display for LockTimeoutParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LockTimeoutParseError {}

impl LockTimeoutParseError {
    fn invalid_value(value: &str) -> Self {
        Self {
            message: format!(
                "Lock timeout value '{value}' is invalid. Use an integer number of seconds or \
                 the word 'infinite'."
            ),
        }
    }
}

/// Parses a lock-timeout override value originating from CLI, environment, or configuration.
pub fn parse_timeout_override(value: &str) -> Result<LockTimeoutValue, LockTimeoutParseError> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("infinite") {
        return Ok(LockTimeoutValue::Infinite);
    }

    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Ok(LockTimeoutValue::from_secs(seconds));
    }

    Err(LockTimeoutParseError::invalid_value(trimmed))
}

/// Resolves the effective timeout value based on CLI > env > config > default precedence.
pub struct LockTimeoutResolver<'a> {
    cli_override: Option<&'a str>,
    env_override: Option<&'a str>,
    config_value: LockTimeoutValue,
    default_value: LockTimeoutValue,
}

impl<'a> LockTimeoutResolver<'a> {
    pub fn new(
        cli_override: Option<&'a str>,
        env_override: Option<&'a str>,
        config_value: LockTimeoutValue,
        default_value: LockTimeoutValue,
    ) -> Self {
        Self {
            cli_override,
            env_override,
            config_value,
            default_value,
        }
    }

    pub fn resolve(self) -> Result<LockTimeoutResolution, LockTimeoutParseError> {
        if let Some(cli_value) = self.cli_override {
            let value = parse_timeout_override(cli_value)?;
            return Ok(LockTimeoutResolution {
                value,
                source: LockTimeoutSource::Cli,
            });
        }

        if let Some(env_value) = self.env_override {
            let value = parse_timeout_override(env_value)?;
            return Ok(LockTimeoutResolution {
                value,
                source: LockTimeoutSource::Environment,
            });
        }

        if self.config_value != self.default_value {
            return Ok(LockTimeoutResolution {
                value: self.config_value,
                source: LockTimeoutSource::Config,
            });
        }

        Ok(LockTimeoutResolution {
            value: self.default_value,
            source: LockTimeoutSource::Default,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numeric_values() {
        assert_eq!(
            parse_timeout_override("42").unwrap(),
            LockTimeoutValue::from_secs(42)
        );
        assert_eq!(
            parse_timeout_override("0").unwrap(),
            LockTimeoutValue::from_secs(0)
        );
    }

    #[test]
    fn parse_infinite_keyword() {
        assert_eq!(
            parse_timeout_override("infinite").unwrap(),
            LockTimeoutValue::Infinite
        );
        assert_eq!(
            parse_timeout_override("Infinite").unwrap(),
            LockTimeoutValue::Infinite
        );
    }

    #[test]
    fn parse_rejects_invalid_input() {
        let err = parse_timeout_override("abc").unwrap_err();
        assert!(
            err.to_string()
                .contains("Use an integer number of seconds or the word 'infinite'")
        );
    }

    #[test]
    fn resolver_precedence() {
        let default = LockTimeoutValue::from_secs(600);
        let config = LockTimeoutValue::from_secs(120);
        let resolution = LockTimeoutResolver::new(Some("30"), Some("40"), config, default)
            .resolve()
            .unwrap();
        assert_eq!(resolution.source, LockTimeoutSource::Cli);
        assert_eq!(resolution.value, LockTimeoutValue::from_secs(30));
    }

    #[test]
    fn resolver_config_vs_default() {
        let default = LockTimeoutValue::from_secs(600);
        let config = LockTimeoutValue::from_secs(45);
        let resolution = LockTimeoutResolver::new(None, None, config, default)
            .resolve()
            .unwrap();
        assert_eq!(resolution.source, LockTimeoutSource::Config);
        assert_eq!(resolution.value, LockTimeoutValue::from_secs(45));
    }

    #[test]
    fn resolver_defaults_when_config_matches() {
        let default = LockTimeoutValue::from_secs(600);
        let config = LockTimeoutValue::from_secs(600);
        let resolution = LockTimeoutResolver::new(None, None, config, default)
            .resolve()
            .unwrap();
        assert_eq!(resolution.source, LockTimeoutSource::Default);
        assert_eq!(resolution.value, default);
    }
}
