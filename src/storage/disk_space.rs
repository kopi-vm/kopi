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
use crate::storage::disk_probe;
use std::path::Path;

pub struct DiskSpaceChecker {
    min_disk_space_mb: u64,
}

impl DiskSpaceChecker {
    pub fn new(min_disk_space_mb: u64) -> Self {
        Self { min_disk_space_mb }
    }

    pub fn check_disk_space(&self, path: &Path, kopi_home: &Path) -> Result<()> {
        let mut target_dir = path.to_path_buf();
        while !target_dir.exists() {
            if let Some(parent) = target_dir.parent() {
                target_dir = parent.to_path_buf();
            } else {
                target_dir = kopi_home.to_path_buf();
                break;
            }
        }

        log::debug!("Checking disk space for path {path:?} (using {target_dir:?})");

        let available_bytes = disk_probe::available_bytes(&target_dir).map_err(|err| {
            log::error!("Failed to check disk space at {target_dir:?}: {err}");
            err
        })?;

        let available_mb = available_bytes / (1024 * 1024);
        log::debug!(
            "Disk space check: available={available_mb}MB, required={}MB",
            self.min_disk_space_mb
        );

        if available_mb < self.min_disk_space_mb {
            return Err(KopiError::DiskSpaceError(format!(
                "Insufficient disk space at {target_dir:?}. Required: {}MB, Available: \
                 {available_mb}MB",
                self.min_disk_space_mb
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_disk_space_check_path_selection() {
        let temp_dir = TempDir::new().unwrap();
        let checker = DiskSpaceChecker::new(500);

        let non_existent = temp_dir.path().join("non/existent/path");
        let result = checker.check_disk_space(&non_existent, temp_dir.path());

        assert!(result.is_ok() || matches!(result.unwrap_err(), KopiError::DiskSpaceError(_)));
    }
}
