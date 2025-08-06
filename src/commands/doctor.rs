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
use crate::doctor::formatters::{format_human_readable, format_json};
use crate::doctor::{CheckCategory, DiagnosticEngine, DiagnosticSummary};
use crate::error::Result;
use std::time::Instant;

pub struct DoctorCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> DoctorCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self, json: bool, verbose: bool, check: Option<&str>) -> Result<()> {
        let start = Instant::now();

        // Parse category filter if provided
        let categories = if let Some(category_str) = check {
            match CheckCategory::parse(category_str) {
                Some(cat) => Some(vec![cat]),
                None => {
                    eprintln!("Invalid check category: {category_str}");
                    eprintln!(
                        "Valid categories: installation, shell, jdks, permissions, network, cache"
                    );
                    return Err(crate::error::KopiError::InvalidConfig(format!(
                        "Invalid check category: {category_str}"
                    )));
                }
            }
        } else {
            None
        };

        // Create diagnostic engine with config - all checks are initialized internally
        let engine = DiagnosticEngine::new(self.config);

        // Run checks with progress display (only when not in JSON mode)
        let results = engine.run_checks(categories, !json);

        let total_duration = start.elapsed();
        let summary = DiagnosticSummary::from_results(&results, total_duration);

        // Output results
        if json {
            format_json(&mut std::io::stdout(), &results, &summary)?;
        } else {
            format_human_readable(&mut std::io::stdout(), &results, &summary, verbose)?;
        }

        // Exit with appropriate code
        std::process::exit(summary.determine_exit_code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_doctor_command_creation() {
        let config = KopiConfig::new(PathBuf::from("/tmp/test")).unwrap();
        let command = DoctorCommand::new(&config).unwrap();
        assert!(std::ptr::eq(command.config, &config));
    }

    #[test]
    fn test_invalid_category() {
        let config = KopiConfig::new(PathBuf::from("/tmp/test")).unwrap();
        let command = DoctorCommand::new(&config).unwrap();

        let result = command.execute(false, false, Some("invalid_category"));
        assert!(result.is_err());
    }
}
