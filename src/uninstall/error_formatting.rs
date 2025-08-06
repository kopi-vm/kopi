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

/// Error formatting utilities for uninstall operations
///
/// This module provides standardized error formatting for various uninstall scenarios,
/// ensuring consistent error presentation across the uninstall module.
use crate::error::KopiError;
use crate::storage::InstalledJdk;

/// Formats and displays an error when multiple JDKs match a version specification
///
/// # Arguments
/// * `version_spec` - The version specification that caused the ambiguity
/// * `matching_jdks` - The list of JDKs that matched the specification
///
/// # Returns
/// A KopiError with a formatted message for the multiple match scenario
pub fn format_multiple_jdk_matches_error(
    version_spec: &str,
    matching_jdks: &[InstalledJdk],
) -> KopiError {
    // Format the list of matching JDKs
    let jdk_list: Vec<String> = matching_jdks
        .iter()
        .map(|jdk| format!("  - {}@{}", jdk.distribution, jdk.version))
        .collect();

    // Display the error message to stderr
    eprintln!("Error: Multiple JDKs match the pattern '{version_spec}'");
    eprintln!("\nFound the following JDKs:");
    for jdk_str in &jdk_list {
        eprintln!("{jdk_str}");
    }
    eprintln!("\nPlease specify exactly one JDK to uninstall using the full version:");
    eprintln!("  kopi uninstall <distribution>@<full-version>");
    eprintln!("\nExample:");
    if let Some(first_jdk) = matching_jdks.first() {
        eprintln!(
            "  kopi uninstall {}@{}",
            first_jdk.distribution, first_jdk.version
        );
    }

    // Return the error
    KopiError::SystemError(format!(
        "Multiple JDKs match '{version_spec}'. Please specify exactly one JDK to uninstall"
    ))
}

/// Formats and displays an error when no JDKs match a version specification
///
/// # Arguments
/// * `version_spec` - The version specification that had no matches
///
/// # Returns
/// A KopiError with a formatted message for the no match scenario
pub fn format_no_jdk_matches_error(version_spec: &str) -> KopiError {
    eprintln!("Error: No JDKs match the pattern '{version_spec}'");
    eprintln!("\nUse 'kopi list' to see available JDKs");
    eprintln!("Use 'kopi uninstall <distribution>@<version>' to uninstall a specific JDK");

    KopiError::SystemError(format!(
        "No JDKs match '{version_spec}'. Use 'kopi list' to see available JDKs"
    ))
}

/// Formats and displays an error when a JDK is not found for uninstallation
///
/// # Arguments
/// * `distribution` - The distribution name
/// * `version` - The version string
///
/// # Returns
/// A KopiError with a formatted message for the JDK not found scenario
pub fn format_jdk_not_found_error(distribution: &str, version: &str) -> KopiError {
    eprintln!("Error: JDK {distribution}@{version} is not installed");
    eprintln!("\nUse 'kopi list' to see available JDKs");
    eprintln!("Use 'kopi install {distribution}@{version}' to install this JDK");

    KopiError::SystemError(format!("JDK {distribution}@{version} is not installed"))
}

/// Formats and displays an error when uninstall confirmation is declined
///
/// # Arguments
/// * `jdk` - The JDK that was not uninstalled
///
/// # Returns
/// A KopiError with a formatted message for the cancelled uninstall scenario
pub fn format_uninstall_cancelled_error(jdk: &InstalledJdk) -> KopiError {
    eprintln!(
        "Uninstall cancelled for {}@{}",
        jdk.distribution, jdk.version
    );

    KopiError::SystemError(format!(
        "Uninstall cancelled for {}@{}",
        jdk.distribution, jdk.version
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::fixtures::create_test_jdk_collection;

    #[test]
    fn test_format_multiple_jdk_matches_error() {
        let jdks = create_test_jdk_collection();
        let error = format_multiple_jdk_matches_error("temurin", &jdks[0..2]);

        match error {
            KopiError::SystemError(msg) => {
                assert!(msg.contains("Multiple JDKs match 'temurin'"));
            }
            _ => panic!("Expected SystemError"),
        }
    }

    #[test]
    fn test_format_no_jdk_matches_error() {
        let error = format_no_jdk_matches_error("nonexistent");

        match error {
            KopiError::SystemError(msg) => {
                assert!(msg.contains("No JDKs match 'nonexistent'"));
            }
            _ => panic!("Expected SystemError"),
        }
    }

    #[test]
    fn test_format_jdk_not_found_error() {
        let error = format_jdk_not_found_error("temurin", "99.0.0");

        match error {
            KopiError::SystemError(msg) => {
                assert!(msg.contains("JDK temurin@99.0.0 is not installed"));
            }
            _ => panic!("Expected SystemError"),
        }
    }

    #[test]
    fn test_format_uninstall_cancelled_error() {
        let jdk = create_test_jdk_collection().into_iter().next().unwrap();
        let error = format_uninstall_cancelled_error(&jdk);

        match error {
            KopiError::SystemError(msg) => {
                assert!(msg.contains("Uninstall cancelled"));
            }
            _ => panic!("Expected SystemError"),
        }
    }
}
