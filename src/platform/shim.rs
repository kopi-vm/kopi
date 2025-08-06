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
use std::fs;
use std::path::Path;

/// Verify that a shim file is valid and not corrupted
#[cfg(unix)]
pub fn verify_shim(shim_path: &Path) -> Result<()> {
    use crate::platform::symlink;

    // Check if it's a symlink
    if !symlink::is_symlink(shim_path)? {
        return Err(KopiError::SystemError("Not a symlink".to_string()));
    }

    // Check if target is the kopi-shim binary
    let target = fs::read_link(shim_path)?;
    if !target.ends_with("kopi-shim") {
        return Err(KopiError::SystemError("Invalid symlink target".to_string()));
    }

    // Check if symlink target exists
    // Note: We resolve the target path relative to the shim's directory if it's relative
    let target_path = if target.is_relative() {
        shim_path.parent().unwrap().join(&target)
    } else {
        target.clone()
    };

    if !target_path.exists() {
        return Err(KopiError::SystemError("Broken symlink".to_string()));
    }

    Ok(())
}

/// Verify that a shim file is valid and not corrupted
#[cfg(windows)]
pub fn verify_shim(shim_path: &Path) -> Result<()> {
    // On Windows, shims are copies of kopi-shim.exe
    if !shim_path.exists() {
        return Err(KopiError::SystemError("Shim file missing".to_string()));
    }

    // Check if it's a regular file
    let metadata = fs::metadata(shim_path)?;
    if !metadata.is_file() {
        return Err(KopiError::SystemError("Not a regular file".to_string()));
    }

    // Check if the file size is reasonable for an executable
    // kopi-shim.exe should be at least several KB
    if metadata.len() < 1024 {
        return Err(KopiError::SystemError(
            "Shim file too small - likely corrupted".to_string(),
        ));
    }

    // Optionally verify it's a valid PE executable by checking the DOS header
    // PE files start with "MZ" (0x4D5A)
    let mut file = fs::File::open(shim_path)?;
    let mut header = [0u8; 2];
    use std::io::Read;
    if file.read_exact(&mut header).is_ok() && header != [0x4D, 0x5A] {
        return Err(KopiError::SystemError(
            "Invalid executable format - not a PE file".to_string(),
        ));
    }

    Ok(())
}
