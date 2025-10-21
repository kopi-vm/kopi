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
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct InstallationContext {
    pub final_path: PathBuf,
    pub temp_path: PathBuf,
}

pub struct JdkInstaller;

impl JdkInstaller {
    pub fn prepare_installation(
        jdks_dir: &Path,
        install_path: &Path,
    ) -> Result<InstallationContext> {
        if install_path.exists() {
            let distribution_info = install_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            return Err(KopiError::AlreadyExists(format!(
                "JDK {distribution_info} is already installed at {install_path:?}"
            )));
        }

        let temp_dir = Self::create_temp_install_dir(jdks_dir)?;

        Ok(InstallationContext {
            final_path: install_path.to_path_buf(),
            temp_path: temp_dir,
        })
    }

    pub fn finalize_installation(context: InstallationContext) -> Result<PathBuf> {
        if let Some(parent) = context.final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let entries: Vec<_> = fs::read_dir(&context.temp_path)?
            .filter_map(|entry| entry.ok())
            .collect();

        let source_path = if entries.len() == 1 {
            let entry = &entries[0];
            if entry.file_type()?.is_dir() {
                entry.path()
            } else {
                context.temp_path.clone()
            }
        } else {
            context.temp_path.clone()
        };

        fs::rename(&source_path, &context.final_path).inspect_err(|_| {
            let _ = fs::remove_dir_all(&context.temp_path);
        })?;

        if source_path != context.temp_path {
            let _ = fs::remove_dir_all(&context.temp_path);
        }

        Ok(context.final_path)
    }

    pub fn cleanup_failed_installation(context: &InstallationContext) -> Result<()> {
        if context.temp_path.exists() {
            fs::remove_dir_all(&context.temp_path)?;
        }
        Ok(())
    }

    fn create_temp_install_dir(jdks_dir: &Path) -> Result<PathBuf> {
        let temp_parent = jdks_dir.join(".tmp");
        fs::create_dir_all(&temp_parent)?;

        let temp_name = format!("install-{}", uuid::Uuid::new_v4());
        let temp_path = temp_parent.join(temp_name);
        fs::create_dir(&temp_path)?;

        Ok(temp_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::install;
    use tempfile::TempDir;

    #[test]
    fn test_prepare_installation_new() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = install::ensure_installations_root(temp_dir.path()).unwrap();
        let install_path = jdks_dir.join("temurin-21.0.1");

        let context = JdkInstaller::prepare_installation(&jdks_dir, &install_path).unwrap();

        assert!(context.temp_path.exists());
        assert!(!context.final_path.exists());
        let temp_root = install::temp_staging_directory(temp_dir.path());
        assert!(context.temp_path.starts_with(&temp_root));
    }

    #[test]
    fn test_prepare_installation_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = install::ensure_installations_root(temp_dir.path()).unwrap();
        let install_path = jdks_dir.join("temurin-21.0.1");
        fs::create_dir_all(&install_path).unwrap();

        let result = JdkInstaller::prepare_installation(&jdks_dir, &install_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::AlreadyExists(_)));
    }

    #[test]
    fn test_finalize_installation() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = install::ensure_installations_root(temp_dir.path()).unwrap();
        let install_path = jdks_dir.join("temurin-21.0.1");

        let context = JdkInstaller::prepare_installation(&jdks_dir, &install_path).unwrap();

        let test_file = context.temp_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let final_path = JdkInstaller::finalize_installation(context).unwrap();

        assert!(final_path.exists());
        assert!(final_path.join("test.txt").exists());
    }

    #[test]
    fn test_cleanup_failed_installation() {
        let temp_dir = TempDir::new().unwrap();
        let jdks_dir = install::ensure_installations_root(temp_dir.path()).unwrap();
        let install_path = jdks_dir.join("temurin-21.0.1");

        let context = JdkInstaller::prepare_installation(&jdks_dir, &install_path).unwrap();

        assert!(context.temp_path.exists());

        JdkInstaller::cleanup_failed_installation(&context).unwrap();
        assert!(!context.temp_path.exists());
    }
}
