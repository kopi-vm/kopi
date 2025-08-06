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

use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticSummary};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use std::io::Write;

pub fn format_human_readable<W: Write>(
    writer: &mut W,
    results: &[CheckResult],
    summary: &DiagnosticSummary,
    verbose: bool,
) -> std::io::Result<()> {
    writeln!(writer, "\nKopi Doctor Report")?;
    writeln!(writer, "==================")?;
    writeln!(writer)?;

    let categories = CheckCategory::all();

    for category in categories {
        let category_results: Vec<&CheckResult> =
            results.iter().filter(|r| r.category == category).collect();

        if category_results.is_empty() {
            continue;
        }

        writeln!(writer, "{category}")?;
        writeln!(writer, "{}", "-".repeat(category.to_string().len()))?;

        for result in category_results {
            let status_symbol = match result.status {
                CheckStatus::Pass => "✓".green(),
                CheckStatus::Fail => "✗".red(),
                CheckStatus::Warning => "⚠".yellow(),
                CheckStatus::Skip => "○".bright_black(),
            };

            writeln!(
                writer,
                "{} {} {}",
                status_symbol, result.name, result.message
            )?;

            if verbose
                || result.status == CheckStatus::Fail
                || result.status == CheckStatus::Warning
            {
                if let Some(ref details) = result.details {
                    for line in details.lines() {
                        writeln!(writer, "    {line}")?;
                    }
                }

                if let Some(ref suggestion) = result.suggestion {
                    writeln!(writer)?;
                    writeln!(writer, "    To fix:")?;
                    for line in suggestion.lines() {
                        writeln!(writer, "      {line}")?;
                    }
                }
            }

            if verbose {
                writeln!(writer, "    Duration: {:?}", result.duration)?;
            }
        }
        writeln!(writer)?;
    }

    writeln!(writer, "Summary")?;
    writeln!(writer, "-------")?;
    writeln!(
        writer,
        "Total checks: {} (✓ {} passed, ✗ {} failed, ⚠ {} warnings, ○ {} skipped)",
        summary.total_checks, summary.passed, summary.failed, summary.warnings, summary.skipped
    )?;
    writeln!(
        writer,
        "Total time: {:.2}s",
        summary.total_duration.as_secs_f64()
    )?;

    Ok(())
}

#[derive(Serialize)]
struct JsonOutput {
    version: String,
    timestamp: DateTime<Utc>,
    summary: JsonSummary,
    categories: Vec<JsonCategory>,
}

#[derive(Serialize)]
struct JsonSummary {
    total_checks: usize,
    passed: usize,
    failed: usize,
    warnings: usize,
    skipped: usize,
    total_duration_ms: u128,
    exit_code: i32,
}

#[derive(Serialize)]
struct JsonCategory {
    name: String,
    checks: Vec<JsonCheck>,
}

#[derive(Serialize)]
struct JsonCheck {
    name: String,
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    duration_ms: u128,
}

pub fn format_json<W: Write>(
    writer: &mut W,
    results: &[CheckResult],
    summary: &DiagnosticSummary,
) -> std::io::Result<()> {
    let mut categories: Vec<JsonCategory> = Vec::new();

    for category in CheckCategory::all() {
        let checks: Vec<JsonCheck> = results
            .iter()
            .filter(|r| r.category == category)
            .map(|r| JsonCheck {
                name: r.name.clone(),
                status: r.status.to_string(),
                message: r.message.clone(),
                details: r.details.clone(),
                suggestion: r.suggestion.clone(),
                duration_ms: r.duration.as_millis(),
            })
            .collect();

        if !checks.is_empty() {
            categories.push(JsonCategory {
                name: category.to_string(),
                checks,
            });
        }
    }

    let output = JsonOutput {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        summary: JsonSummary {
            total_checks: summary.total_checks,
            passed: summary.passed,
            failed: summary.failed,
            warnings: summary.warnings,
            skipped: summary.skipped,
            total_duration_ms: summary.total_duration.as_millis(),
            exit_code: summary.determine_exit_code(),
        },
        categories,
    };

    serde_json::to_writer_pretty(&mut *writer, &output)?;
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_results() -> Vec<CheckResult> {
        vec![
            CheckResult::new(
                "Kopi binary in PATH",
                CheckCategory::Installation,
                CheckStatus::Pass,
                "Found at /usr/local/bin/kopi",
                Duration::from_millis(100),
            ),
            CheckResult::new(
                "Configuration file",
                CheckCategory::Installation,
                CheckStatus::Warning,
                "Config file missing",
                Duration::from_millis(150),
            )
            .with_suggestion("Run 'kopi config init' to create a default configuration"),
            CheckResult::new(
                "PATH contains shims",
                CheckCategory::Shell,
                CheckStatus::Fail,
                "~/.kopi/shims not found in PATH",
                Duration::from_millis(200),
            )
            .with_details("Current PATH: /usr/bin:/bin")
            .with_suggestion("Add 'export PATH=\"$HOME/.kopi/shims:$PATH\"' to your shell config"),
        ]
    }

    #[test]
    fn test_human_readable_format() {
        let results = create_test_results();
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));

        let mut output = Vec::new();
        format_human_readable(&mut output, &results, &summary, false).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Kopi Doctor Report"));
        assert!(output_str.contains("Installation"));
        assert!(output_str.contains("Shell"));
        assert!(output_str.contains("✓"));
        assert!(output_str.contains("✗"));
        assert!(output_str.contains("⚠"));
        assert!(output_str.contains("Summary"));
    }

    #[test]
    fn test_verbose_human_format() {
        let results = create_test_results();
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));

        let mut output = Vec::new();
        format_human_readable(&mut output, &results, &summary, true).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Duration:"));
        assert!(output_str.contains("Current PATH:"));
    }

    #[test]
    fn test_json_format() {
        let results = create_test_results();
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));

        let mut output = Vec::new();
        format_json(&mut output, &results, &summary).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

        assert_eq!(json["summary"]["total_checks"], 3);
        assert_eq!(json["summary"]["passed"], 1);
        assert_eq!(json["summary"]["failed"], 1);
        assert_eq!(json["summary"]["warnings"], 1);
        assert!(json["version"].is_string());
        assert!(json["timestamp"].is_string());
        assert!(json["categories"].is_array());
    }

    #[test]
    fn test_json_optional_fields() {
        let result = CheckResult::new(
            "Simple check",
            CheckCategory::Network,
            CheckStatus::Pass,
            "All good",
            Duration::from_millis(50),
        );

        let results = vec![result];
        let summary = DiagnosticSummary::from_results(&results, Duration::from_secs(1));

        let mut output = Vec::new();
        format_json(&mut output, &results, &summary).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

        let check = &json["categories"][0]["checks"][0];
        assert!(check.get("details").is_none());
        assert!(check.get("suggestion").is_none());
    }
}
