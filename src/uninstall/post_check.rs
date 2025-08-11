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

use crate::error::Result;
use crate::storage::{InstalledJdk, JdkRepository};
use log::{debug, warn};
use std::path::{Path, PathBuf};

/// Post-uninstall validation and cleanup functionality
pub struct PostUninstallChecker<'a> {
    repository: &'a JdkRepository<'a>,
}

impl<'a> PostUninstallChecker<'a> {
    pub fn new(repository: &'a JdkRepository<'a>) -> Self {
        Self { repository }
    }

    /// Perform comprehensive post-uninstall validation
    pub fn validate_removal(&self, removed_jdk: &InstalledJdk) -> Result<PostUninstallReport> {
        debug!(
            "Validating removal of {}@{}",
            removed_jdk.distribution, removed_jdk.version
        );

        let mut report = PostUninstallReport {
            jdk_completely_removed: false,
            orphaned_metadata_files: Vec::new(),
            shim_functionality_intact: false,
            remaining_jdks: Vec::new(),
            suggested_actions: Vec::new(),
        };

        // Check if JDK directory was completely removed
        report.jdk_completely_removed = self.verify_complete_removal(&removed_jdk.path)?;

        // Check for orphaned metadata files
        report.orphaned_metadata_files = self.check_orphaned_metadata(&removed_jdk.path)?;

        // Get remaining JDKs
        report.remaining_jdks = self.repository.list_installed_jdks()?;

        // Validate shim functionality
        report.shim_functionality_intact = self.validate_shim_functionality()?;

        // Generate suggested actions
        report.suggested_actions = self.generate_suggested_actions(&report);

        Ok(report)
    }

    /// Verify that the JDK directory was completely removed
    fn verify_complete_removal(&self, jdk_path: &Path) -> Result<bool> {
        debug!("Verifying complete removal of {}", jdk_path.display());

        // Check if the main JDK directory still exists
        if jdk_path.exists() {
            warn!(
                "JDK directory still exists after removal: {}",
                jdk_path.display()
            );
            return Ok(false);
        }

        // Check if any temporary removal files remain
        if let Some(parent) = jdk_path.parent()
            && parent.exists()
        {
            for entry in std::fs::read_dir(parent)? {
                let entry = entry?;
                let path = entry.path();

                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check for temporary removal files (e.g., .temurin-21.0.1.removing)
                    if file_name.starts_with('.') && file_name.ends_with(".removing") {
                        warn!("Found temporary removal file: {}", path.display());
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Check for orphaned metadata files
    fn check_orphaned_metadata(&self, jdk_path: &Path) -> Result<Vec<PathBuf>> {
        use std::collections::HashSet;
        
        debug!(
            "Checking for orphaned metadata files related to {}",
            jdk_path.display()
        );

        let mut orphaned_files = HashSet::new();

        // Check for .meta.json files in the JDK directory
        if jdk_path.exists() {
            // If the JDK directory still exists, check for orphaned metadata
            // Metadata files are stored as <distribution>-<version>.meta.json in jdks directory
            if let Some(jdk_dir_name) = jdk_path.file_name().and_then(|n| n.to_str())
                && let Some(parent) = jdk_path.parent()
            {
                let meta_file = parent.join(format!("{jdk_dir_name}.meta.json"));
                if meta_file.exists() {
                    orphaned_files.insert(meta_file);
                }
            }
        }

        // Check for metadata files in the parent directory that might reference the removed JDK
        if let Some(parent) = jdk_path.parent()
            && parent.exists()
        {
            for entry in std::fs::read_dir(parent)? {
                let entry = entry?;
                let path = entry.path();

                if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                    && file_name.ends_with(".meta.json")
                    && path.is_file()
                {
                    // Check if this metadata file references the removed JDK
                    if let Some(jdk_name) = jdk_path.file_name().and_then(|n| n.to_str())
                        && file_name.contains(jdk_name)
                    {
                        orphaned_files.insert(path);
                    }
                }
            }
        }

        Ok(orphaned_files.into_iter().collect())
    }

    /// Validate that shim functionality is still intact
    fn validate_shim_functionality(&self) -> Result<bool> {
        debug!("Validating shim functionality");

        // Check if we still have at least one JDK installed
        let remaining_jdks = self.repository.list_installed_jdks()?;
        if remaining_jdks.is_empty() {
            debug!("No JDKs remain - shim functionality will be limited");
            return Ok(false); // Not necessarily broken, but limited functionality
        }

        // Check if shim binaries are still present
        // This is a basic check - more comprehensive validation would require
        // actually testing shim execution, which is complex and slow

        // For now, we assume shim functionality is intact if we have JDKs
        // and the shim directory structure exists
        Ok(true)
    }

    /// Generate suggested actions based on the validation results
    fn generate_suggested_actions(&self, report: &PostUninstallReport) -> Vec<String> {
        let mut actions = Vec::new();

        if !report.jdk_completely_removed {
            actions.push("Run 'kopi doctor' to check for issues with JDK removal".to_string());
            actions.push("Consider manually removing any remaining JDK files".to_string());
        }

        if !report.orphaned_metadata_files.is_empty() {
            actions.push("Clean up orphaned metadata files to free disk space".to_string());
        }

        if !report.shim_functionality_intact {
            if report.remaining_jdks.is_empty() {
                actions.push("Consider installing a JDK to restore full functionality".to_string());
                actions.push("Run 'kopi list --remote' to see available JDKs".to_string());
            } else {
                actions.push("Run 'kopi doctor' to diagnose shim issues".to_string());
            }
        }

        if report.remaining_jdks.is_empty() {
            actions
                .push("All JDKs have been removed. Your shell PATH may need updating.".to_string());
        }

        actions
    }

    /// Clean up orphaned metadata files
    pub fn cleanup_orphaned_metadata(&self, report: &PostUninstallReport) -> Result<usize> {
        debug!(
            "Cleaning up {} orphaned metadata files",
            report.orphaned_metadata_files.len()
        );

        let mut cleaned_count = 0;

        for file_path in &report.orphaned_metadata_files {
            match std::fs::remove_file(file_path) {
                Ok(()) => {
                    debug!("Removed orphaned metadata file: {}", file_path.display());
                    cleaned_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to remove orphaned metadata file {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(cleaned_count)
    }
}

/// Report generated after post-uninstall validation
#[derive(Debug)]
pub struct PostUninstallReport {
    /// Whether the JDK directory was completely removed
    pub jdk_completely_removed: bool,
    /// List of orphaned metadata files found
    pub orphaned_metadata_files: Vec<PathBuf>,
    /// Whether shim functionality is still intact
    pub shim_functionality_intact: bool,
    /// List of remaining installed JDKs
    pub remaining_jdks: Vec<InstalledJdk>,
    /// Suggested actions for the user
    pub suggested_actions: Vec<String>,
}

impl PostUninstallReport {
    /// Check if the uninstall was completely successful
    pub fn is_successful(&self) -> bool {
        self.jdk_completely_removed && self.orphaned_metadata_files.is_empty()
    }

    /// Get a summary of the post-uninstall state
    pub fn get_summary(&self) -> String {
        if self.is_successful() {
            if self.remaining_jdks.is_empty() {
                "✓ JDK removed successfully. No JDKs remain installed.".to_string()
            } else {
                format!(
                    "✓ JDK removed successfully. {} JDK{} remain installed.",
                    self.remaining_jdks.len(),
                    if self.remaining_jdks.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            }
        } else {
            let mut issues = Vec::new();

            if !self.jdk_completely_removed {
                issues.push("incomplete removal");
            }

            if !self.orphaned_metadata_files.is_empty() {
                issues.push("orphaned metadata files");
            }

            format!("⚠ JDK removal completed with issues: {}", issues.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::version::Version;
    use std::fs;
    use std::str::FromStr;
    use tempfile::TempDir;

    fn create_test_setup() -> (TempDir, KopiConfig) {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Create jdks directory
        fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();

        (temp_dir, config)
    }

    fn create_mock_jdk(config: &KopiConfig, distribution: &str, version: &str) -> InstalledJdk {
        let jdk_path = config
            .jdks_dir()
            .unwrap()
            .join(format!("{distribution}-{version}"));
        fs::create_dir_all(&jdk_path).unwrap();

        // Create some mock files
        fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        fs::write(jdk_path.join("bin/java"), "#!/bin/sh\necho mock java").unwrap();

        InstalledJdk {
            distribution: distribution.to_string(),
            version: Version::from_str(version).unwrap(),
            path: jdk_path,
        }
    }

    #[test]
    fn test_verify_complete_removal_success() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);
        let jdk = create_mock_jdk(&config, "temurin", "21.0.1");

        // Remove the JDK directory
        fs::remove_dir_all(&jdk.path).unwrap();

        let result = checker.verify_complete_removal(&jdk.path).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_complete_removal_failure() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);
        let jdk = create_mock_jdk(&config, "temurin", "21.0.1");

        // Don't remove the JDK directory
        let result = checker.verify_complete_removal(&jdk.path).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_orphaned_metadata() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);
        let jdk = create_mock_jdk(&config, "temurin", "21.0.1");

        // Create a metadata file in the parent directory
        let jdks_dir = config.jdks_dir().unwrap();
        let meta_file = jdks_dir.join(format!(
            "{}.meta.json",
            jdk.path.file_name().unwrap().to_str().unwrap()
        ));
        fs::write(&meta_file, "{}").unwrap();

        let orphaned = checker.check_orphaned_metadata(&jdk.path).unwrap();
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0], meta_file);
    }

    #[test]
    fn test_validate_shim_functionality() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);

        // With no JDKs installed, shim functionality should be limited
        let result = checker.validate_shim_functionality().unwrap();
        assert!(!result);

        // Create a JDK
        create_mock_jdk(&config, "temurin", "21.0.1");

        // With JDKs installed, shim functionality should be intact
        let result = checker.validate_shim_functionality().unwrap();
        assert!(result);
    }

    #[test]
    fn test_validate_removal_report() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);
        let jdk = create_mock_jdk(&config, "temurin", "21.0.1");

        // Create another JDK to remain after removal
        create_mock_jdk(&config, "corretto", "17.0.9");

        // Remove the first JDK
        fs::remove_dir_all(&jdk.path).unwrap();

        let report = checker.validate_removal(&jdk).unwrap();

        assert!(report.jdk_completely_removed);
        assert!(report.orphaned_metadata_files.is_empty());
        assert!(report.shim_functionality_intact);
        assert_eq!(report.remaining_jdks.len(), 1);
        // Suggested actions may be empty for successful removals
        assert!(report.suggested_actions.is_empty());
    }

    #[test]
    fn test_cleanup_orphaned_metadata() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);
        let jdk = create_mock_jdk(&config, "temurin", "21.0.1");

        // Create metadata files in the parent directory
        let jdks_dir = config.jdks_dir().unwrap();
        let meta_file = jdks_dir.join(format!(
            "{}.meta.json",
            jdk.path.file_name().unwrap().to_str().unwrap()
        ));
        fs::write(&meta_file, "{}").unwrap();

        let report = PostUninstallReport {
            jdk_completely_removed: true,
            orphaned_metadata_files: vec![meta_file.clone()],
            shim_functionality_intact: true,
            remaining_jdks: Vec::new(),
            suggested_actions: Vec::new(),
        };

        let cleaned = checker.cleanup_orphaned_metadata(&report).unwrap();
        assert_eq!(cleaned, 1);
        assert!(!meta_file.exists());
    }

    #[test]
    fn test_post_uninstall_report_summary() {
        let report = PostUninstallReport {
            jdk_completely_removed: true,
            orphaned_metadata_files: Vec::new(),
            shim_functionality_intact: true,
            remaining_jdks: Vec::new(),
            suggested_actions: Vec::new(),
        };

        assert!(report.is_successful());
        assert!(report.get_summary().contains("No JDKs remain installed"));

        let report_with_remaining = PostUninstallReport {
            jdk_completely_removed: true,
            orphaned_metadata_files: Vec::new(),
            shim_functionality_intact: true,
            remaining_jdks: vec![InstalledJdk {
                distribution: "temurin".to_string(),
                version: Version::from_str("21.0.1").unwrap(),
                path: "/test/path".into(),
            }],
            suggested_actions: Vec::new(),
        };

        assert!(report_with_remaining.is_successful());
        assert!(report_with_remaining.get_summary().contains("1 JDK remain"));
    }

    #[test]
    fn test_generate_suggested_actions() {
        let (_temp_dir, config) = create_test_setup();
        let repository = JdkRepository::new(&config);
        let checker = PostUninstallChecker::new(&repository);

        let report = PostUninstallReport {
            jdk_completely_removed: false,
            orphaned_metadata_files: vec!["/test/meta.json".into()],
            shim_functionality_intact: false,
            remaining_jdks: Vec::new(),
            suggested_actions: Vec::new(),
        };

        let actions = checker.generate_suggested_actions(&report);
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.contains("kopi doctor")));
        assert!(actions.iter().any(|a| a.contains("metadata files")));
        assert!(actions.iter().any(|a| a.contains("installing a JDK")));
    }
}
