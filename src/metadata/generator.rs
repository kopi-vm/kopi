use crate::error::{KopiError, Result};
use crate::metadata::index::{IndexFile, IndexFileEntry};
use crate::metadata::{FoojayMetadataSource, MetadataSource};
use crate::models::metadata::JdkMetadata;
use crate::models::platform::{Architecture, OperatingSystem};
use crate::storage::formatting::format_size;
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
struct JdkUpdateInfo {
    _id: String,
    distribution: String,
    version: String,
    architecture: String,
    update_type: UpdateType,
    changes: Vec<String>,
}

/// Type of update for a JDK
#[derive(Debug, PartialEq)]
enum UpdateType {
    New,
    Modified,
}

/// State of a file being generated
#[derive(Serialize, Deserialize, Debug)]
struct FileState {
    status: FileStatus,
    started_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    attempts: u32,
    error: Option<String>,
    checksum: Option<String>,
}

/// Status of file generation
#[derive(Serialize, Deserialize, Debug)]
enum FileStatus {
    InProgress,
    Completed,
    Failed,
}

/// Metadata generator for creating metadata files from foojay API
pub struct MetadataGenerator {
    config: GeneratorConfig,
}

impl MetadataGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate metadata files
    pub fn generate(&self, output_dir: &Path) -> Result<()> {
        println!("🚀 Starting metadata generation...");

        // Step 1: Fetch all metadata from foojay
        self.report_progress("Fetching metadata from foojay API...");
        let source = FoojayMetadataSource::new();
        let all_metadata = source.fetch_all()?;
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
                    if !jdk.is_complete {
                        match source.fetch_package_details(&jdk.id) {
                            Ok(details) => {
                                jdk.download_url = Some(details.download_url);
                                jdk.checksum = details.checksum;
                                jdk.checksum_type = details.checksum_type;
                                jdk.is_complete = true;
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
                checksum: Some(self.calculate_sha256(&metadata.content)),
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

    /// Calculate SHA256 checksum of content
    fn calculate_sha256(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculate SHA256 checksum of a file
    fn calculate_file_checksum(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Process a single metadata file with state management
    fn process_metadata_file(&self, path: &Path, metadata: &[JdkMetadata]) -> Result<()> {
        let state_path = PathBuf::from(format!("{}.state", path.display()));

        // 1. Create .state file at work start
        let state = FileState {
            status: FileStatus::InProgress,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            attempts: 1,
            error: None,
            checksum: None,
        };
        fs::write(&state_path, serde_json::to_string(&state)?)?;

        // 2. Perform actual processing
        let content = if self.config.minify_json {
            serde_json::to_string(metadata)?
        } else {
            serde_json::to_string_pretty(metadata)?
        };

        match fs::write(path, &content) {
            Ok(_) => {
                // 3. Update .state on success
                let mut state = state;
                state.status = FileStatus::Completed;
                state.updated_at = Utc::now();
                state.checksum = Some(self.calculate_file_checksum(path)?);
                fs::write(&state_path, serde_json::to_string(&state)?)?;
                Ok(())
            }
            Err(e) => {
                // Update .state on failure
                let mut state = state;
                state.status = FileStatus::Failed;
                state.error = Some(e.to_string());
                state.updated_at = Utc::now();
                fs::write(&state_path, serde_json::to_string(&state)?)?;
                Err(e.into())
            }
        }
    }

    /// Check if a file should be skipped based on its state file
    fn should_skip_file(&self, json_path: &Path) -> bool {
        let state_path = PathBuf::from(format!("{}.state", json_path.display()));

        if let Ok(content) = fs::read_to_string(&state_path) {
            if let Ok(state) = serde_json::from_str::<FileState>(&content) {
                match state.status {
                    FileStatus::Completed => {
                        // Validate the file still exists and matches checksum
                        if let Some(checksum) = state.checksum {
                            if json_path.exists() {
                                if let Ok(current_checksum) =
                                    self.calculate_file_checksum(json_path)
                                {
                                    return current_checksum == checksum;
                                }
                            }
                        }
                        false
                    }
                    FileStatus::InProgress => {
                        // Check if the process is stale (e.g., > 1 hour old)
                        let age = Utc::now() - state.updated_at;
                        age.num_hours() > 1
                    }
                    FileStatus::Failed => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Detect if there are any .state files in the output directory
    fn detect_resume_state(&self, output_dir: &Path) -> bool {
        use walkdir::WalkDir;

        if !output_dir.exists() {
            return false;
        }

        // Check for any .state files
        for entry in WalkDir::new(output_dir).max_depth(3).into_iter().flatten() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("state")) {
                return true;
            }
        }

        false
    }

    /// Cleanup state files after successful generation
    fn cleanup_state_files(&self, output_dir: &Path) -> Result<()> {
        use walkdir::WalkDir;

        // 1. Remove all .state files in subdirectories
        for entry in WalkDir::new(output_dir) {
            let entry = entry?;
            let path = entry.path();

            // Skip index.json.state for now
            if path.extension() == Some(OsStr::new("state"))
                && path != output_dir.join("index.json.state")
            {
                fs::remove_file(path)?;
            }
        }

        // 2. Finally remove index.json.state
        let index_state = output_dir.join("index.json.state");
        if index_state.exists() {
            fs::remove_file(index_state)?;
        }

        Ok(())
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

        // Check if resume is needed based on .state files
        let has_state_files = self.detect_resume_state(output_dir);
        let should_resume = if self.config.force {
            // Force flag overrides any resume behavior
            if has_state_files {
                println!(
                    "⚠️  Found existing state files, but --force was specified. Starting fresh generation..."
                );
                // Clean up old state files when forcing
                let _ = self.cleanup_state_files(output_dir);
            }
            false
        } else {
            has_state_files
        };

        if should_resume {
            println!("🔄 Found incomplete generation state files. Automatically resuming...");
            println!("   (Use --force to start fresh and ignore existing state)");
            // Use state-based writing with resume support
            self.write_output_with_state(output_dir, index, files)
        } else {
            // Use traditional writing without state management
            self.write_output_without_state(output_dir, index, files)
        }
    }

    /// Write output files without state management (traditional approach)
    fn write_output_without_state(
        &self,
        output_dir: &Path,
        index: &IndexFile,
        files: &HashMap<String, FileMetadata>,
    ) -> Result<()> {
        // Write index.json
        let index_path = output_dir.join("index.json");
        let index_json = if self.config.minify_json {
            serde_json::to_string(index)?
        } else {
            serde_json::to_string_pretty(index)?
        };
        fs::write(&index_path, &index_json)?;
        self.report_progress(&format!("Wrote index.json ({} bytes)", index_json.len()));

        // Write metadata files in sorted order
        let mut sorted_paths: Vec<_> = files.keys().cloned().collect();
        sorted_paths.sort();

        for path in sorted_paths {
            let metadata = &files[&path];
            let file_path = output_dir.join(&path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, &metadata.content)?;
            self.report_progress(&format!(
                "Wrote {} ({} bytes)",
                path,
                metadata.content.len()
            ));
        }

        Ok(())
    }

    /// Write output files with state management for resume support
    fn write_output_with_state(
        &self,
        output_dir: &Path,
        index: &IndexFile,
        files: &HashMap<String, FileMetadata>,
    ) -> Result<()> {
        let mut errors = Vec::new();
        let mut skipped = 0;
        let mut written = 0;

        // Process metadata files first in sorted order
        let mut sorted_paths: Vec<_> = files.keys().cloned().collect();
        sorted_paths.sort();

        for path in sorted_paths {
            let metadata = &files[&path];
            let file_path = output_dir.join(&path);

            // Check if should skip this file
            if self.should_skip_file(&file_path) {
                self.report_progress(&format!("Skipping {path} (already completed)"));
                skipped += 1;
                continue;
            }

            // Create parent directory if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Parse JSON to get JdkMetadata for process_metadata_file
            match serde_json::from_str::<Vec<JdkMetadata>>(&metadata.content) {
                Ok(jdk_metadata) => match self.process_metadata_file(&file_path, &jdk_metadata) {
                    Ok(_) => {
                        self.report_progress(&format!(
                            "Wrote {} ({} bytes)",
                            path,
                            metadata.content.len()
                        ));
                        written += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to write {path}: {e}"));
                    }
                },
                Err(e) => {
                    errors.push(format!("Failed to parse metadata for {path}: {e}"));
                }
            }
        }

        // Process index.json last
        let index_path = output_dir.join("index.json");
        let index_state_path = PathBuf::from(format!("{}.state", index_path.display()));

        if !self.should_skip_file(&index_path) {
            // Create state for index.json
            let state = FileState {
                status: FileStatus::InProgress,
                started_at: Utc::now(),
                updated_at: Utc::now(),
                attempts: 1,
                error: None,
                checksum: None,
            };
            fs::write(&index_state_path, serde_json::to_string(&state)?)?;

            let index_json = if self.config.minify_json {
                serde_json::to_string(index)?
            } else {
                serde_json::to_string_pretty(index)?
            };

            match fs::write(&index_path, &index_json) {
                Ok(_) => {
                    // Update state on success
                    let mut state = state;
                    state.status = FileStatus::Completed;
                    state.updated_at = Utc::now();
                    state.checksum = Some(self.calculate_file_checksum(&index_path)?);
                    fs::write(&index_state_path, serde_json::to_string(&state)?)?;
                    self.report_progress(&format!("Wrote index.json ({} bytes)", index_json.len()));
                    written += 1;
                }
                Err(e) => {
                    // Update state on failure
                    let mut state = state;
                    state.status = FileStatus::Failed;
                    state.error = Some(e.to_string());
                    state.updated_at = Utc::now();
                    fs::write(&index_state_path, serde_json::to_string(&state)?)?;
                    errors.push(format!("Failed to write index.json: {e}"));
                }
            }
        } else {
            self.report_progress("Skipping index.json (already completed)");
            skipped += 1;
        }

        // Report summary
        if skipped > 0 {
            println!("📊 Skipped {skipped} already completed files");
        }
        if written > 0 {
            println!("✏️  Wrote {written} new files");
        }

        // Handle errors
        if !errors.is_empty() {
            eprintln!("\n❌ Errors during write:");
            for error in &errors {
                eprintln!("  - {error}");
            }
            return Err(KopiError::GenerationFailed(format!(
                "{} files failed to write",
                errors.len()
            )));
        }

        // Clean up state files on complete success
        if errors.is_empty() {
            self.cleanup_state_files(output_dir)?;
            println!("🧹 Cleaned up state files");
        }

        Ok(())
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
        println!("🔍 Validating metadata directory: {}", input_dir.display());

        // Check if directory exists
        if !input_dir.exists() {
            return Err(KopiError::NotFound(format!(
                "Directory not found: {}",
                input_dir.display()
            )));
        }

        // Check for index.json
        let index_path = input_dir.join("index.json");
        if !index_path.exists() {
            return Err(KopiError::InvalidConfig("index.json not found".to_string()));
        }

        // Parse index.json
        let index_content = fs::read_to_string(&index_path)?;
        let index: IndexFile = serde_json::from_str(&index_content)
            .map_err(|e| KopiError::InvalidConfig(format!("Invalid index.json: {e}")))?;

        println!(
            "  ✓ index.json is valid (version: {}, {} files)",
            index.version,
            index.files.len()
        );

        // Validate each file referenced in index
        let mut errors = Vec::new();
        for entry in &index.files {
            let file_path = input_dir.join(&entry.path);
            if !file_path.exists() {
                errors.push(format!("File not found: {}", entry.path));
                continue;
            }

            // Check file size
            let metadata = fs::metadata(&file_path)?;
            if metadata.len() != entry.size {
                errors.push(format!(
                    "Size mismatch for {}: expected {}, actual {}",
                    entry.path,
                    entry.size,
                    metadata.len()
                ));
            }

            // Validate JSON content
            let content = fs::read_to_string(&file_path)?;
            match serde_json::from_str::<Vec<JdkMetadata>>(&content) {
                Ok(jdks) => {
                    println!("  ✓ {} ({} JDKs)", entry.path, jdks.len());
                }
                Err(e) => {
                    errors.push(format!("Invalid JSON in {}: {}", entry.path, e));
                }
            }
        }

        if !errors.is_empty() {
            println!("\n❌ Validation errors:");
            for error in &errors {
                println!("  - {error}");
            }
            return Err(KopiError::InvalidConfig(format!(
                "{} validation errors found",
                errors.len()
            )));
        }

        println!("\n✅ All metadata files are valid!");
        Ok(())
    }

    /// Update existing metadata - efficient incremental synchronization
    pub fn update(&self, input_dir: &Path, output_dir: &Path) -> Result<()> {
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
                copy_metadata_directory(input_dir, output_dir)?;
            }
            return Ok(());
        }

        // Store detailed change info for dry run summary
        let detailed_changes = if self.config.dry_run {
            self.detect_detailed_changes(&existing_by_id, &filtered_final)
        } else {
            Vec::new()
        };

        // Step 5: Fetch complete details only for changed items
        self.report_progress("Fetching details for changed packages...");
        let updated_metadata = self.fetch_complete_metadata(updates_needed)?;

        // Step 6: Combine updated and unchanged metadata
        let mut all_metadata = updated_metadata;
        all_metadata.extend(unchanged);

        // Step 7: Organize and write output (same as generate)
        let organized_files = self.organize_metadata(all_metadata)?;
        println!("  Organized into {} files", organized_files.len());

        let index = self.create_index(&organized_files)?;

        if self.config.dry_run {
            self.show_dry_run_output(&index, &organized_files);
            self.show_detailed_update_summary(&detailed_changes);
        } else {
            self.write_output(output_dir, &index, &organized_files)?;
            println!(
                "✅ Successfully updated metadata in {}",
                output_dir.display()
            );
        }

        Ok(())
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
    fn detect_detailed_changes(
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
    fn show_detailed_update_summary(&self, changes: &[JdkUpdateInfo]) {
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

/// Copy metadata directory when output differs from input
fn copy_metadata_directory(from: &Path, to: &Path) -> Result<()> {
    use std::fs;

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
            copy_metadata_directory(&source, &target)?;
        } else if file_type.is_file() {
            fs::copy(&source, &target)?;
        }
    }

    Ok(())
}
