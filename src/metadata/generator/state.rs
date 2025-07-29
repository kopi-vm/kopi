use crate::error::Result;
use crate::models::metadata::JdkMetadata;
use chrono::Utc;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{FileState, FileStatus};

/// Process a single metadata file with state management
pub fn process_metadata_file(
    path: &Path,
    metadata: &[JdkMetadata],
    minify_json: bool,
) -> Result<()> {
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
    let content = if minify_json {
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
            state.checksum = Some(calculate_file_checksum(path)?);
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
pub fn should_skip_file(json_path: &Path) -> bool {
    let state_path = PathBuf::from(format!("{}.state", json_path.display()));

    if let Ok(content) = fs::read_to_string(&state_path) {
        if let Ok(state) = serde_json::from_str::<FileState>(&content) {
            match state.status {
                FileStatus::Completed => {
                    // Validate the file still exists and matches checksum
                    if let Some(checksum) = state.checksum {
                        if json_path.exists() {
                            if let Ok(current_checksum) = calculate_file_checksum(json_path) {
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
pub fn detect_resume_state(output_dir: &Path) -> bool {
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
pub fn cleanup_state_files(output_dir: &Path) -> Result<()> {
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

/// Calculate SHA256 checksum of a file
pub fn calculate_file_checksum(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Calculate SHA256 checksum of content
pub fn calculate_sha256(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
