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
use crate::models::metadata::JdkMetadata;
use std::fs;
use std::path::Path;

/// Validate metadata directory structure
pub fn validate(input_dir: &Path) -> Result<()> {
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
