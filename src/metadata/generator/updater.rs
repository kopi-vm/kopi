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
use crate::metadata::index::IndexFile;
use crate::metadata::{FoojayMetadataSource, MetadataSource};
use crate::models::metadata::JdkMetadata;
use crate::storage::formatting::format_size;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::types::{GeneratorConfig, JdkUpdateInfo, UpdateType};

pub struct UpdateResult {
    pub updates_needed: Vec<JdkMetadata>,
    pub unchanged: Vec<JdkMetadata>,
    pub detailed_changes: Vec<JdkUpdateInfo>,
    pub needs_copy: bool,
}

pub struct UpdateHandler {
    config: GeneratorConfig,
}

impl UpdateHandler {
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Analyze metadata for updates
    pub fn analyze_updates(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        dry_run: bool,
    ) -> Result<UpdateResult> {
        println!("🚀 Starting metadata update...");

        // Step 1: Load existing metadata from input directory
        self.report_progress("Loading existing metadata...");
        let existing_metadata = self.load_existing_metadata(input_dir)?;
        let existing_by_id: HashMap<String, JdkMetadata> = existing_metadata
            .iter()
            .map(|jdk| (jdk.id.clone(), jdk.clone()))
            .collect();
        println!("  Found {} existing JDK packages", existing_metadata.len());

        // Step 2: Fetch current list from API (unavoidable)
        self.report_progress("Fetching current metadata list from foojay API...");
        let source = FoojayMetadataSource::new();
        let current_list = source.fetch_all()?;
        println!("  Found {} JDK packages in API", current_list.len());

        // Step 3: Filter by configuration (same as generate)
        let filtered_by_dist = self.filter_by_distribution(current_list);
        let filtered_by_platform = self.filter_by_platform(filtered_by_dist);
        let filtered_final = self.filter_by_javafx(filtered_by_platform);
        println!("  After filters: {} packages", filtered_final.len());

        // Safety check: Ensure we're not losing too many packages
        let existing_count = existing_metadata.len();
        let current_count = filtered_final.len();
        if existing_count > 0 && current_count < existing_count {
            let reduction_percentage =
                ((existing_count - current_count) as f64 / existing_count as f64) * 100.0;
            if reduction_percentage >= 5.0 && !self.config.force {
                return Err(KopiError::ValidationError(format!(
                    "Package count dropped by {reduction_percentage:.1}% ({existing_count} → {current_count}). This might indicate an API issue. Use --force to override."
                )));
            } else if reduction_percentage > 0.0 {
                println!(
                    "  ⚠️  Warning: Package count decreased by {reduction_percentage:.1}% ({existing_count} → {current_count})"
                );
            }
        }

        // Step 4: Compare and detect changes
        self.report_progress("Detecting changes...");
        let (updates_needed, unchanged) = self.detect_changes(&existing_by_id, &filtered_final);
        println!(
            "  Changes detected: {} packages need updates",
            updates_needed.len()
        );
        println!("  Unchanged: {} packages", unchanged.len());

        if updates_needed.is_empty() && unchanged.len() == existing_by_id.len() {
            println!("✅ Metadata is already up to date!");

            // If output_dir is different from input_dir, copy the existing metadata
            if input_dir != output_dir {
                self.report_progress("Copying unchanged metadata to output directory...");
                Self::copy_metadata_directory(input_dir, output_dir)?;
            }
            return Ok(UpdateResult {
                updates_needed: vec![],
                unchanged,
                detailed_changes: vec![],
                needs_copy: false,
            });
        }

        // Store detailed change info for dry run summary
        let detailed_changes = if dry_run {
            self.detect_detailed_changes(&existing_by_id, &filtered_final)
        } else {
            Vec::new()
        };

        Ok(UpdateResult {
            updates_needed,
            unchanged,
            detailed_changes,
            needs_copy: input_dir != output_dir,
        })
    }

    /// Load existing metadata from directory
    fn load_existing_metadata(&self, input_dir: &Path) -> Result<Vec<JdkMetadata>> {
        let mut all_metadata = Vec::new();

        // First, validate the directory structure
        let index_path = input_dir.join("index.json");
        if !index_path.exists() {
            return Err(KopiError::NotFound(format!(
                "index.json not found in {}",
                input_dir.display()
            )));
        }

        // Parse index.json
        let index_content = fs::read_to_string(&index_path)?;
        let index: IndexFile = serde_json::from_str(&index_content)
            .map_err(|e| KopiError::InvalidConfig(format!("Invalid index.json: {e}")))?;

        // Load each metadata file
        for entry in index.files {
            let file_path = input_dir.join(&entry.path);
            if file_path.exists() {
                let content = fs::read_to_string(&file_path)?;
                match serde_json::from_str::<Vec<JdkMetadata>>(&content) {
                    Ok(jdks) => {
                        all_metadata.extend(jdks);
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: Failed to parse {}: {}", entry.path, e);
                    }
                }
            } else {
                eprintln!("⚠️  Warning: File not found: {}", entry.path);
            }
        }

        Ok(all_metadata)
    }

    /// Detect changes between existing and current metadata
    fn detect_changes(
        &self,
        existing_by_id: &HashMap<String, JdkMetadata>,
        current_list: &[JdkMetadata],
    ) -> (Vec<JdkMetadata>, Vec<JdkMetadata>) {
        let mut updates_needed = Vec::new();
        let mut unchanged = Vec::new();

        for current_jdk in current_list.iter() {
            if let Some(existing_jdk) = existing_by_id.get(&current_jdk.id) {
                // Check if update is needed
                if self.needs_update(existing_jdk, current_jdk) {
                    updates_needed.push(current_jdk.clone());
                } else {
                    // Use existing metadata which has complete details
                    unchanged.push(existing_jdk.clone());
                }
            } else {
                // New JDK not in existing metadata
                updates_needed.push(current_jdk.clone());
            }
        }

        (updates_needed, unchanged)
    }

    /// Detect detailed changes between existing and current metadata
    pub fn detect_detailed_changes(
        &self,
        existing_by_id: &HashMap<String, JdkMetadata>,
        current_list: &[JdkMetadata],
    ) -> Vec<JdkUpdateInfo> {
        let mut changes = Vec::new();

        for current_jdk in current_list {
            if let Some(existing_jdk) = existing_by_id.get(&current_jdk.id) {
                let mut change_details = Vec::new();

                if existing_jdk.distribution_version != current_jdk.distribution_version {
                    change_details.push(format!(
                        "version: {} → {}",
                        existing_jdk.distribution_version, current_jdk.distribution_version
                    ));
                }

                if existing_jdk.size != current_jdk.size {
                    change_details.push(format!(
                        "size: {} → {}",
                        format_size(existing_jdk.size as u64),
                        format_size(current_jdk.size as u64)
                    ));
                }

                if existing_jdk.latest_build_available != current_jdk.latest_build_available {
                    change_details.push(format!(
                        "latest_build: {} → {}",
                        existing_jdk
                            .latest_build_available
                            .map_or("N/A".to_string(), |v| v.to_string()),
                        current_jdk
                            .latest_build_available
                            .map_or("N/A".to_string(), |v| v.to_string())
                    ));
                }

                if existing_jdk.release_status != current_jdk.release_status {
                    change_details.push(format!(
                        "status: {} → {}",
                        existing_jdk.release_status.as_deref().unwrap_or("N/A"),
                        current_jdk.release_status.as_deref().unwrap_or("N/A")
                    ));
                }

                if existing_jdk.term_of_support != current_jdk.term_of_support {
                    change_details.push(format!(
                        "support: {} → {}",
                        existing_jdk.term_of_support.as_deref().unwrap_or("N/A"),
                        current_jdk.term_of_support.as_deref().unwrap_or("N/A")
                    ));
                }

                if !change_details.is_empty() {
                    changes.push(JdkUpdateInfo {
                        _id: current_jdk.id.clone(),
                        distribution: current_jdk.distribution.clone(),
                        version: current_jdk.version.to_string(),
                        architecture: current_jdk.architecture.to_string(),
                        update_type: UpdateType::Modified,
                        changes: change_details,
                    });
                }
            } else {
                changes.push(JdkUpdateInfo {
                    _id: current_jdk.id.clone(),
                    distribution: current_jdk.distribution.clone(),
                    version: current_jdk.version.to_string(),
                    architecture: current_jdk.architecture.to_string(),
                    update_type: UpdateType::New,
                    changes: vec![],
                });
            }
        }

        changes
    }

    /// Check if a JDK needs to be updated
    fn needs_update(&self, existing: &JdkMetadata, current: &JdkMetadata) -> bool {
        // Check fields available in the list API response
        existing.distribution_version != current.distribution_version
            || existing.size != current.size
            || existing.latest_build_available != current.latest_build_available
            || existing.release_status != current.release_status
            || existing.term_of_support != current.term_of_support
    }

    /// Show detailed update summary in dry run mode
    pub fn show_detailed_update_summary(&self, changes: &[JdkUpdateInfo]) {
        println!("\n📊 Update Summary:");

        let new_jdks: Vec<_> = changes
            .iter()
            .filter(|c| c.update_type == UpdateType::New)
            .collect();

        let updated_jdks: Vec<_> = changes
            .iter()
            .filter(|c| c.update_type == UpdateType::Modified)
            .collect();

        if !new_jdks.is_empty() {
            println!("\n  🆕 New JDKs ({}):", new_jdks.len());
            for jdk in new_jdks.iter().take(10) {
                println!(
                    "    - {} {} {}",
                    jdk.distribution, jdk.version, jdk.architecture
                );
            }
            if new_jdks.len() > 10 {
                println!("    ... and {} more", new_jdks.len() - 10);
            }
        }

        if !updated_jdks.is_empty() {
            println!("\n  🔄 Updated JDKs ({}):", updated_jdks.len());
            for jdk in updated_jdks.iter().take(10) {
                println!(
                    "    - {} {} {}",
                    jdk.distribution, jdk.version, jdk.architecture
                );
                for change in &jdk.changes {
                    println!("        • {change}");
                }
            }
            if updated_jdks.len() > 10 {
                println!("    ... and {} more", updated_jdks.len() - 10);
            }
        }

        if new_jdks.is_empty() && updated_jdks.is_empty() {
            println!("\n  ✨ No changes detected");
        }

        // Summary statistics
        println!("\n  📈 Summary:");
        println!("    • Total packages checked: {}", changes.len());
        println!("    • New packages: {}", new_jdks.len());
        println!("    • Updated packages: {}", updated_jdks.len());
    }

    /// Filter metadata by distribution
    fn filter_by_distribution(&self, metadata: Vec<JdkMetadata>) -> Vec<JdkMetadata> {
        if let Some(distributions) = &self.config.distributions {
            metadata
                .into_iter()
                .filter(|jdk| distributions.contains(&jdk.distribution))
                .collect()
        } else {
            metadata
        }
    }

    /// Filter metadata by platform
    fn filter_by_platform(&self, metadata: Vec<JdkMetadata>) -> Vec<JdkMetadata> {
        if let Some(platforms) = &self.config.platforms {
            metadata
                .into_iter()
                .filter(|jdk| {
                    platforms.iter().any(|p| {
                        p.os == jdk.operating_system
                            && p.arch == jdk.architecture
                            && (p.libc.is_none() || p.libc == jdk.lib_c_type)
                    })
                })
                .collect()
        } else {
            metadata
        }
    }

    /// Filter metadata by JavaFX bundled status
    fn filter_by_javafx(&self, metadata: Vec<JdkMetadata>) -> Vec<JdkMetadata> {
        if self.config.javafx_bundled {
            metadata
                .into_iter()
                .filter(|jdk| jdk.javafx_bundled)
                .collect()
        } else {
            metadata
        }
    }

    /// Copy metadata directory when output differs from input
    fn copy_metadata_directory(from: &Path, to: &Path) -> Result<()> {
        // Create target directory
        fs::create_dir_all(to)?;

        // Copy all files and subdirectories
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source = entry.path();
            let file_name = entry.file_name();
            let target = to.join(&file_name);

            if file_type.is_dir() {
                Self::copy_metadata_directory(&source, &target)?;
            } else if file_type.is_file() {
                fs::copy(&source, &target)?;
            }
        }

        Ok(())
    }

    /// Report progress
    fn report_progress(&self, message: &str) {
        println!("📦 {message}");
    }
}
