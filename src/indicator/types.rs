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

#[derive(Debug, Clone)]
pub struct ProgressConfig {
    pub total: Option<u64>,
    pub style: ProgressStyle,
}

impl ProgressConfig {
    pub fn new(style: ProgressStyle) -> Self {
        Self { total: None, style }
    }

    pub fn with_total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    Bytes,
    Count,
    Status,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self::Count
    }
}

impl std::fmt::Display for ProgressStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes => write!(f, "bytes"),
            Self::Count => write!(f, "count"),
            Self::Status => write!(f, "status"),
        }
    }
}

/// Indicates which renderer behaviour a progress indicator supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressRendererKind {
    /// Interactive terminal renderer supporting carriage-return updates.
    Tty,
    /// Line-oriented renderer for non-interactive output (pipes, CI logs).
    NonTty,
    /// Silent renderer that suppresses all user-facing output.
    Silent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_config_construction() {
        let config = ProgressConfig::new(ProgressStyle::Bytes);
        assert_eq!(config.style, ProgressStyle::Bytes);
        assert_eq!(config.total, None);
    }

    #[test]
    fn test_progress_config_with_total() {
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(100);
        assert_eq!(config.total, Some(100));
    }

    #[test]
    fn test_progress_style_default() {
        assert_eq!(ProgressStyle::default(), ProgressStyle::Count);
    }

    #[test]
    fn test_progress_style_display() {
        assert_eq!(format!("{}", ProgressStyle::Bytes), "bytes");
        assert_eq!(format!("{}", ProgressStyle::Count), "count");
        assert_eq!(format!("{}", ProgressStyle::Status), "status");
    }

    #[test]
    fn test_progress_config_clone() {
        let config = ProgressConfig::new(ProgressStyle::Count).with_total(50);
        let cloned = config.clone();
        assert_eq!(cloned.style, config.style);
        assert_eq!(cloned.total, config.total);
    }
}
