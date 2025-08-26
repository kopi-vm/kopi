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

pub mod state;
pub mod types;
pub mod updater;
pub mod validator;
pub mod writer;

use crate::error::{KopiError, Result};
use crate::indicator::SilentProgress;
use crate::metadata::index::{IndexFile, IndexFileEntry};
use crate::metadata::{FoojayMetadataSource, MetadataSource};
use crate::models::metadata::JdkMetadata;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use self::types::FileMetadata;
use self::updater::UpdateHandler;

pub use self::types::{GeneratorConfig, Platform};

/// Metadata generator for creating metadata files from foojay API
pub struct MetadataGenerator {
    config: self::types::GeneratorConfig,
}

impl MetadataGenerator {
    pub fn new(config: self::types::GeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate metadata files
    pub fn generate(&self, output_dir: &Path) -> Result<()> {
        println!("🚀 Starting metadata generation...");

        // Step 1: Fetch all metadata from foojay
        self.report_progress("Fetching metadata from foojay API...");
        let source = FoojayMetadataSource::new();
        let mut progress = SilentProgress;
        let all_metadata = source.fetch_all(&mut progress)?;
        println!("  Found {} JDK packages", all_metadata.len());

        // Step 2: Filter by distribution if specified
        let filtered_by_dist = self.filter_by_distribution(all_metadata);
        println!(
            "  After distribution filter: {} packages",
            filtered_by_dist.len()
        );

        // Step 3: Filter by platform if specified
        let filtered_by_platform = self.filter_by_platform(filtered_by_dist);
        println!(
            "  After platform filter: {} packages",
            filtered_by_platform.len()
        );

        // Step 4: Filter by JavaFX if specified
        let filtered_final = self.filter_by_javafx(filtered_by_platform);
        println!("  After JavaFX filter: {} packages", filtered_final.len());

        if filtered_final.is_empty() {
            return Err(KopiError::NotFound(
                "No packages match the specified filters".to_string(),
            ));
        }

        // Step 5: Fetch complete details for each package
        self.report_progress("Fetching package details...");
        let complete_metadata = self.fetch_complete_metadata(filtered_final)?;

        // Step 6: Organize metadata by distribution and platform
        let organized_files = self.organize_metadata(complete_metadata)?;
        println!("  Organized into {} files", organized_files.len());

        // Step 7: Create index.json
        let index = self.create_index(&organized_files)?;

        // Step 8: Write files (or show dry run output)
        if self.config.dry_run {
            self.show_dry_run_output(&index, &organized_files);
        } else {
            self.write_output(output_dir, &index, &organized_files)?;
            println!(
                "✅ Successfully generated metadata in {}",
                output_dir.display()
            );
        }

        Ok(())
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

    /// Fetch complete metadata with lazy-loaded fields
    fn fetch_complete_metadata(&self, metadata: Vec<JdkMetadata>) -> Result<Vec<JdkMetadata>> {
        let total = metadata.len();
        let pb = self.create_progress_bar(total as u64);

        let results = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let semaphore = Arc::new(AtomicUsize::new(0));
        let max_concurrent = self.config.parallel_requests;

        let chunks: Vec<_> = metadata.chunks(100).collect();
        let mut handles = vec![];

        for chunk in chunks {
            let chunk_vec = chunk.to_vec();
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);
            let semaphore = Arc::clone(&semaphore);
            let pb = pb.clone();

            let handle = thread::spawn(move || {
                // Create a new FoojayMetadataSource for this thread
                let source = FoojayMetadataSource::new();
                for mut jdk in chunk_vec {
                    // Simple semaphore implementation
                    loop {
                        let current = semaphore.load(Ordering::SeqCst);
                        if current < max_concurrent
                            && semaphore
                                .compare_exchange(
                                    current,
                                    current + 1,
                                    Ordering::SeqCst,
                                    Ordering::SeqCst,
                                )
                                .is_ok()
                        {
                            break;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }

                    // Fetch package details if not complete
                    if !jdk.is_complete() {
                        let mut progress = SilentProgress;
                        match source.fetch_package_details(&jdk.id, &mut progress) {
                            Ok(details) => {
                                jdk.download_url = Some(details.download_url);
                                jdk.checksum = details.checksum;
                                jdk.checksum_type = details.checksum_type;
                            }
                            Err(e) => {
                                errors
                                    .lock()
                                    .unwrap()
                                    .push(format!("Failed to fetch details for {}: {}", jdk.id, e));
                                continue;
                            }
                        }
                    }

                    results.lock().unwrap().push(jdk);
                    pb.inc(1);

                    // Release semaphore
                    semaphore.fetch_sub(1, Ordering::SeqCst);

                    // Small delay to avoid overwhelming the API
                    thread::sleep(Duration::from_millis(100));
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle
                .join()
                .map_err(|_| KopiError::ThreadPanic("Worker thread panicked".to_string()))?;
        }

        pb.finish_with_message("Package details fetched");

        // Check for errors
        let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
        if !errors.is_empty() {
            eprintln!("⚠️  Warnings during fetch:");
            for error in &errors {
                eprintln!("  - {error}");
            }
        }

        let results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        Ok(results)
    }

    /// Organize metadata into files by distribution and platform
    fn organize_metadata(
        &self,
        metadata: Vec<JdkMetadata>,
    ) -> Result<HashMap<String, FileMetadata>> {
        let mut files = HashMap::new();
        let mut grouped: HashMap<String, Vec<JdkMetadata>> = HashMap::new();

        // Group by platform/distribution
        for jdk in metadata {
            // Create platform directory name
            let platform_dir = if let Some(libc) = &jdk.lib_c_type {
                format!("{}-{}-{}", jdk.operating_system, jdk.architecture, libc)
            } else {
                format!("{}-{}", jdk.operating_system, jdk.architecture)
            };

            // Group by platform/distribution
            let key = format!("{}/{}.json", platform_dir, jdk.distribution);

            grouped.entry(key).or_default().push(jdk);
        }

        // Sort the keys to ensure deterministic order
        let mut sorted_keys: Vec<_> = grouped.keys().cloned().collect();
        sorted_keys.sort();

        // Create file metadata for each group in sorted order
        for path in sorted_keys {
            let mut jdks = grouped.remove(&path).unwrap();

            // Sort JdkMetadata entries by distribution_version (descending) as primary key, id as secondary key
            jdks.sort_by(
                |a, b| match b.distribution_version.cmp(&a.distribution_version) {
                    std::cmp::Ordering::Equal => a.id.cmp(&b.id),
                    other => other,
                },
            );

            // Extract platform and distribution from path
            // e.g., "linux-x64-glibc/temurin.json"
            let parts: Vec<&str> = path.splitn(2, '/').collect();
            let platform_parts: Vec<&str> = parts[0].split('-').collect();
            let distribution_path = parts[1];
            let distribution = distribution_path.trim_end_matches(".json").to_string();

            let os = platform_parts[0].to_string();
            let architecture = platform_parts[1].to_string();
            let libc = if platform_parts.len() > 2 {
                Some(platform_parts[2].to_string())
            } else {
                None
            };

            let content = if self.config.minify_json {
                serde_json::to_string(&jdks)?
            } else {
                serde_json::to_string_pretty(&jdks)?
            };

            files.insert(
                path,
                FileMetadata {
                    distribution,
                    os,
                    architecture,
                    libc,
                    content,
                },
            );
        }

        Ok(files)
    }

    /// Create index.json with metadata about all files
    fn create_index(&self, files: &HashMap<String, FileMetadata>) -> Result<IndexFile> {
        let mut entries = Vec::new();

        // Sort file paths to ensure deterministic order
        let mut sorted_paths: Vec<_> = files.keys().cloned().collect();
        sorted_paths.sort();

        for path in sorted_paths {
            let metadata = &files[&path];
            entries.push(IndexFileEntry {
                path: path.clone(),
                distribution: metadata.distribution.clone(),
                architectures: Some(vec![metadata.architecture.clone()]),
                operating_systems: Some(vec![metadata.os.clone()]),
                lib_c_types: metadata.libc.as_ref().map(|l| vec![l.clone()]),
                size: metadata.content.len() as u64,
                checksum: Some(self::state::calculate_sha256(&metadata.content)),
                last_modified: Some(Utc::now().to_rfc3339()),
            });
        }

        Ok(IndexFile {
            version: 2,
            updated: Utc::now().to_rfc3339(),
            files: entries,
            generator_config: Some(self.config.clone()),
        })
    }

    /// Write output files
    fn write_output(
        &self,
        output_dir: &Path,
        index: &IndexFile,
        files: &HashMap<String, FileMetadata>,
    ) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Delegate to writer module
        self::writer::write_output(&self.config, output_dir, index, files)
    }

    /// Show dry run output
    fn show_dry_run_output(&self, index: &IndexFile, files: &HashMap<String, FileMetadata>) {
        println!("\n📋 Dry run - would create the following files:");
        println!("  index.json ({} entries)", index.files.len());

        // Sort file paths for consistent output
        let mut sorted_paths: Vec<_> = files.keys().cloned().collect();
        sorted_paths.sort();

        for path in sorted_paths {
            let metadata = &files[&path];
            println!("  {} ({} bytes)", path, metadata.content.len());
        }

        println!("\nTotal: {} files", files.len() + 1);
    }

    /// Validate metadata directory structure
    pub fn validate(&self, input_dir: &Path) -> Result<()> {
        self::validator::validate(input_dir)
    }

    /// Update existing metadata - efficient incremental synchronization
    pub fn update(&self, input_dir: &Path, output_dir: &Path) -> Result<()> {
        let updater = UpdateHandler::new(self.config.clone());
        let result = updater.analyze_updates(input_dir, output_dir, self.config.dry_run)?;

        // If no updates are needed and metadata is already copied, we're done
        if result.updates_needed.is_empty() && !result.needs_copy {
            return Ok(());
        }

        // Step 5: Fetch complete details only for changed items
        self.report_progress("Fetching details for changed packages...");
        let updated_metadata = self.fetch_complete_metadata(result.updates_needed)?;

        // Step 6: Combine updated and unchanged metadata
        let mut all_metadata = updated_metadata;
        all_metadata.extend(result.unchanged);

        // Step 7: Organize and write output (same as generate)
        let organized_files = self.organize_metadata(all_metadata)?;
        println!("  Organized into {} files", organized_files.len());

        let index = self.create_index(&organized_files)?;

        if self.config.dry_run {
            self.show_dry_run_output(&index, &organized_files);
            updater.show_detailed_update_summary(&result.detailed_changes);
        } else {
            self.write_output(output_dir, &index, &organized_files)?;
            println!(
                "✅ Successfully updated metadata in {}",
                output_dir.display()
            );
        }

        Ok(())
    }

    /// Report progress
    fn report_progress(&self, message: &str) {
        println!("📦 {message}");
    }

    /// Create progress bar
    fn create_progress_bar(&self, total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    }
}
