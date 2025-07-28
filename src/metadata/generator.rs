use crate::error::{KopiError, Result};
use crate::metadata::index::{IndexFile, IndexFileEntry};
use crate::metadata::{FoojayMetadataSource, MetadataSource};
use crate::models::metadata::JdkMetadata;
use crate::models::platform::{Architecture, OperatingSystem};
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Platform specification for filtering
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct GeneratorConfig {
    pub distributions: Option<Vec<String>>,
    pub platforms: Option<Vec<Platform>>,
    pub javafx_bundled: bool,
    pub parallel_requests: usize,
    pub dry_run: bool,
    pub minify_json: bool,
}

/// Metadata for a file to be written
pub struct FileMetadata {
    pub distribution: String,
    pub os: String,
    pub architecture: String,
    pub libc: Option<String>,
    pub content: String,
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

        // Create file metadata for each group
        for (path, jdks) in grouped {
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

        for (path, metadata) in files {
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
        })
    }

    /// Calculate SHA256 checksum of content
    fn calculate_sha256(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
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

        // Write index.json
        let index_path = output_dir.join("index.json");
        let index_json = if self.config.minify_json {
            serde_json::to_string(index)?
        } else {
            serde_json::to_string_pretty(index)?
        };
        fs::write(&index_path, &index_json)?;
        self.report_progress(&format!("Wrote index.json ({} bytes)", index_json.len()));

        // Write metadata files
        for (path, metadata) in files {
            let file_path = output_dir.join(path);
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

    /// Show dry run output
    fn show_dry_run_output(&self, index: &IndexFile, files: &HashMap<String, FileMetadata>) {
        println!("\n📋 Dry run - would create the following files:");
        println!("  index.json ({} entries)", index.files.len());

        for (path, metadata) in files {
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
