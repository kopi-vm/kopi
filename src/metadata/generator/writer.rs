use crate::error::{KopiError, Result};
use crate::metadata::index::IndexFile;
use crate::models::metadata::JdkMetadata;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::state;
use super::types::{FileMetadata, FileState, FileStatus, GeneratorConfig};

/// Write output files
pub fn write_output(
    config: &GeneratorConfig,
    output_dir: &Path,
    index: &IndexFile,
    files: &HashMap<String, FileMetadata>,
) -> Result<()> {
    // Check if resume is needed based on .state files
    let has_state_files = state::detect_resume_state(output_dir);
    let should_resume = if config.force {
        // Force flag overrides any resume behavior
        if has_state_files {
            println!(
                "⚠️  Found existing state files, but --force was specified. Starting fresh generation..."
            );
            // Clean up old state files when forcing
            let _ = state::cleanup_state_files(output_dir);
        }
        false
    } else {
        has_state_files
    };

    if should_resume {
        println!("🔄 Found incomplete generation state files. Automatically resuming...");
        println!("   (Use --force to start fresh and ignore existing state)");
        // Use state-based writing with resume support
        write_output_with_state(config, output_dir, index, files)
    } else {
        // Use traditional writing without state management
        write_output_without_state(config, output_dir, index, files)
    }
}

/// Write output files without state management (traditional approach)
fn write_output_without_state(
    config: &GeneratorConfig,
    output_dir: &Path,
    index: &IndexFile,
    files: &HashMap<String, FileMetadata>,
) -> Result<()> {
    // Write index.json
    let index_path = output_dir.join("index.json");
    let index_json = if config.minify_json {
        serde_json::to_string(index)?
    } else {
        serde_json::to_string_pretty(index)?
    };
    fs::write(&index_path, &index_json)?;
    report_progress(&format!("Wrote index.json ({} bytes)", index_json.len()));

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
        report_progress(&format!(
            "Wrote {} ({} bytes)",
            path,
            metadata.content.len()
        ));
    }

    Ok(())
}

/// Write output files with state management for resume support
fn write_output_with_state(
    config: &GeneratorConfig,
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
        if state::should_skip_file(&file_path) {
            report_progress(&format!("Skipping {path} (already completed)"));
            skipped += 1;
            continue;
        }

        // Create parent directory if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Parse JSON to get JdkMetadata for process_metadata_file
        match serde_json::from_str::<Vec<JdkMetadata>>(&metadata.content) {
            Ok(jdk_metadata) => {
                match state::process_metadata_file(&file_path, &jdk_metadata, config.minify_json) {
                    Ok(_) => {
                        report_progress(&format!(
                            "Wrote {} ({} bytes)",
                            path,
                            metadata.content.len()
                        ));
                        written += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to write {path}: {e}"));
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Failed to parse metadata for {path}: {e}"));
            }
        }
    }

    // Process index.json last
    let index_path = output_dir.join("index.json");
    let index_state_path = PathBuf::from(format!("{}.state", index_path.display()));

    if !state::should_skip_file(&index_path) {
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

        let index_json = if config.minify_json {
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
                state.checksum = Some(state::calculate_file_checksum(&index_path)?);
                fs::write(&index_state_path, serde_json::to_string(&state)?)?;
                report_progress(&format!("Wrote index.json ({} bytes)", index_json.len()));
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
        report_progress("Skipping index.json (already completed)");
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
        state::cleanup_state_files(output_dir)?;
        println!("🧹 Cleaned up state files");
    }

    Ok(())
}

/// Report progress
fn report_progress(message: &str) {
    println!("📦 {message}");
}
