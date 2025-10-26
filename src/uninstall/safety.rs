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

use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::storage::{InstalledJdk, JdkRepository};
use crate::version::VersionRequest;
use log::{debug, trace, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const GLOBAL_VERSION_FILENAME: &str = "version";
const KOPI_VERSION_FILE: &str = ".kopi-version";
const JAVA_VERSION_FILE: &str = ".java-version";

/// Perform safety checks before uninstalling a JDK.
///
/// The new active-use detection deliberately ignores the `KOPI_JAVA_VERSION`
/// environment variable for now (see T-s2g7h Phase 1 decision); only global and
/// project version files participate in uninstall blocking.
pub fn perform_safety_checks(
    config: &KopiConfig,
    _repository: &JdkRepository,
    jdk: &InstalledJdk,
    force: bool,
) -> Result<()> {
    debug!(
        "Performing safety checks for {}@{}",
        jdk.distribution, jdk.version
    );

    if force {
        debug!(
            "Skipping active-use detection for {}@{} due to force override",
            jdk.distribution, jdk.version
        );
    } else {
        if let Some(active) = detect_global_active_jdk(config, jdk)? {
            return Err(KopiError::ValidationError(format!(
                "Cannot uninstall {dist}@{ver} - it is currently active globally via {} \
                 (configured as {}). Use --force to override this check or run \
                 'kopi global unset' before uninstalling.",
                active.version_file.display(),
                active.request,
                dist = jdk.distribution,
                ver = jdk.version
            )));
        }

        if let Some(active) = detect_project_active_jdk(jdk)? {
            return Err(KopiError::ValidationError(format!(
                "Cannot uninstall {dist}@{ver} - it is configured for this project via {} \
                 (configured as {}). Use --force to override this check or update the \
                 project version file.",
                active.version_file.display(),
                active.request,
                dist = jdk.distribution,
                ver = jdk.version
            )));
        }
    }

    // Check for running Java processes (future enhancement)
    check_running_processes(&jdk.distribution, &jdk.version.to_string())?;

    Ok(())
}

fn detect_global_active_jdk(config: &KopiConfig, jdk: &InstalledJdk) -> Result<Option<ActiveUse>> {
    let version_file = config.kopi_home().join(GLOBAL_VERSION_FILENAME);
    if !version_file.exists() {
        return Ok(None);
    }

    match read_kopi_version_request(&version_file)? {
        Some(request) => {
            if request_matches_jdk(&request, jdk) {
                debug!(
                    "Global version file {} matches target {}@{} (request: {})",
                    version_file.display(),
                    jdk.distribution,
                    jdk.version,
                    request
                );
                Ok(Some(ActiveUse::new(version_file, request)))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

fn detect_project_active_jdk(jdk: &InstalledJdk) -> Result<Option<ActiveUse>> {
    let start_dir = env::current_dir().map_err(|e| {
        KopiError::SystemError(format!("Failed to determine current directory: {e}"))
    })?;

    let mut current = start_dir.as_path();
    loop {
        let kopi_version_file = current.join(KOPI_VERSION_FILE);
        if let Some(request) = read_kopi_version_request(&kopi_version_file)?
            && request_matches_jdk(&request, jdk)
        {
            debug!(
                "Project version file {} matches target {}@{} (request: {})",
                kopi_version_file.display(),
                jdk.distribution,
                jdk.version,
                request
            );
            return Ok(Some(ActiveUse::new(kopi_version_file, request)));
        }

        let java_version_file = current.join(JAVA_VERSION_FILE);
        if let Some(request) = read_java_version_request(&java_version_file)?
            && request_matches_jdk(&request, jdk)
        {
            debug!(
                "Java version file {} matches target {}@{} (request: {})",
                java_version_file.display(),
                jdk.distribution,
                jdk.version,
                request
            );
            return Ok(Some(ActiveUse::new(java_version_file, request)));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Ok(None)
}

fn read_kopi_version_request(path: &Path) -> Result<Option<VersionRequest>> {
    read_version_request(path, VersionFileKind::Kopi)
}

fn read_java_version_request(path: &Path) -> Result<Option<VersionRequest>> {
    read_version_request(path, VersionFileKind::Java)
}

fn read_version_request(path: &Path, kind: VersionFileKind) -> Result<Option<VersionRequest>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!(
                "Failed to read version file {}: {e}. Ignoring for active-use detection.",
                path.display()
            );
            return Ok(None);
        }
    };

    let trimmed = content.trim();
    if trimmed.is_empty() {
        trace!(
            "Version file {} is empty; ignoring for active-use detection",
            path.display()
        );
        return Ok(None);
    }

    let request_result = match kind {
        VersionFileKind::Kopi => VersionRequest::from_str(trimmed),
        VersionFileKind::Java => VersionRequest::new(trimmed.to_string()),
    };

    match request_result {
        Ok(request) => Ok(Some(request)),
        Err(e) => {
            warn!(
                "Ignoring version file {} due to parse error: {e}",
                path.display()
            );
            Ok(None)
        }
    }
}

fn request_matches_jdk(request: &VersionRequest, jdk: &InstalledJdk) -> bool {
    if request
        .distribution
        .as_ref()
        .is_some_and(|distribution| !distribution.eq_ignore_ascii_case(&jdk.distribution))
    {
        return false;
    }

    if request
        .javafx_bundled
        .is_some_and(|javafx| javafx != jdk.javafx_bundled)
    {
        return false;
    }

    jdk.version.matches_pattern(&request.version_pattern)
}

fn check_running_processes(_distribution: &str, _version: &str) -> Result<()> {
    // TODO: Future enhancement - check for running processes
    Ok(())
}

struct ActiveUse {
    version_file: PathBuf,
    request: VersionRequest,
}

impl ActiveUse {
    fn new(version_file: PathBuf, request: VersionRequest) -> Self {
        Self {
            version_file,
            request,
        }
    }
}

enum VersionFileKind {
    Kopi,
    Java,
}

/// Verify user has permission to remove the directory
pub fn verify_removal_permission(path: &Path) -> Result<()> {
    debug!("Verifying removal permission for {}", path.display());

    // Check if path exists
    if !path.exists() {
        return Err(KopiError::DirectoryNotFound(path.display().to_string()));
    }

    // Try to check if we can write to parent directory (proxy for removal permission)
    if let Some(parent) = path.parent() {
        match fs::metadata(parent) {
            Ok(metadata) => {
                if metadata.permissions().readonly() {
                    return Err(KopiError::PermissionDenied(format!(
                        "Parent directory is read-only: {}",
                        parent.display()
                    )));
                }
            }
            Err(e) => {
                return Err(KopiError::PermissionDenied(format!(
                    "Cannot access parent directory: {e}"
                )));
            }
        }
    }

    Ok(())
}

/// Check if other tools depend on this JDK
pub fn check_tool_dependencies(path: &Path) -> Result<()> {
    debug!("Checking tool dependencies for {}", path.display());

    // TODO: Future enhancement - check if other tools have hardcoded paths to this JDK
    // For now, just warn about potential issues
    if path.join("bin/java").exists() {
        warn!("Note: Other tools may have references to this JDK installation");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::paths::install;
    use crate::storage::JdkRepository;
    use crate::version::Version;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    struct TestFixture {
        temp_dir: TempDir,
        config: KopiConfig,
    }

    impl TestFixture {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
            fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();
            Self { temp_dir, config }
        }

        fn repository(&self) -> JdkRepository<'_> {
            JdkRepository::new(&self.config)
        }

        fn create_installed_jdk(&self, distribution: &str, version: &str) -> InstalledJdk {
            let jdk_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{distribution}-{version}"));
            fs::create_dir_all(&jdk_path).unwrap();

            let bin_dir = install::bin_directory(&jdk_path);
            fs::create_dir_all(&bin_dir).unwrap();
            fs::write(bin_dir.join("java"), "#!/bin/sh\necho mock java").unwrap();
            fs::write(
                jdk_path.join("release"),
                format!("JAVA_VERSION=\"{version}\""),
            )
            .unwrap();

            InstalledJdk::new(
                distribution.to_string(),
                Version::from_str(version).unwrap(),
                jdk_path,
                false,
            )
        }
    }

    #[test]
    fn safety_checks_allow_when_not_active() {
        let fixture = TestFixture::new();
        let repository = fixture.repository();
        let jdk = fixture.create_installed_jdk("temurin", "21.0.5+11");

        assert!(perform_safety_checks(&fixture.config, &repository, &jdk, false).is_ok());
    }

    #[test]
    fn safety_checks_block_global_default() {
        let fixture = TestFixture::new();
        let repository = fixture.repository();
        let jdk = fixture.create_installed_jdk("temurin", "21.0.5+11");

        let global_path = fixture.config.kopi_home().join(GLOBAL_VERSION_FILENAME);
        jdk.write_to(&global_path).unwrap();

        let result = perform_safety_checks(&fixture.config, &repository, &jdk, false);
        assert!(matches!(result, Err(KopiError::ValidationError(_))));

        // Force override should bypass the guard
        assert!(perform_safety_checks(&fixture.config, &repository, &jdk, true).is_ok());
    }

    #[test]
    #[serial]
    fn safety_checks_block_project_kopi_version() {
        let fixture = TestFixture::new();
        let repository = fixture.repository();
        let jdk = fixture.create_installed_jdk("temurin", "21.0.5+11");

        let project_dir = fixture.temp_dir.path().join("workspace/project");
        fs::create_dir_all(&project_dir).unwrap();
        jdk.write_to(&project_dir.join(KOPI_VERSION_FILE)).unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&project_dir).unwrap();
        let result = perform_safety_checks(&fixture.config, &repository, &jdk, false);
        env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, Err(KopiError::ValidationError(_))));
    }

    #[test]
    #[serial]
    fn safety_checks_block_project_java_version() {
        let fixture = TestFixture::new();
        let repository = fixture.repository();
        let jdk = fixture.create_installed_jdk("zulu", "17.0.9");

        let project_dir = fixture.temp_dir.path().join("workspace/java");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(JAVA_VERSION_FILE), "17.0.9\n").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&project_dir).unwrap();
        let result = perform_safety_checks(&fixture.config, &repository, &jdk, false);
        env::set_current_dir(original_dir).unwrap();

        assert!(matches!(result, Err(KopiError::ValidationError(_))));
    }

    #[test]
    #[serial]
    fn safety_checks_ignore_invalid_version_files() {
        let fixture = TestFixture::new();
        let repository = fixture.repository();
        let jdk = fixture.create_installed_jdk("temurin", "21.0.5+11");

        let project_dir = fixture.temp_dir.path().join("workspace/invalid");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(KOPI_VERSION_FILE), "not-a-version").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&project_dir).unwrap();
        let result = perform_safety_checks(&fixture.config, &repository, &jdk, false);
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_removal_permission() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_jdk");
        fs::create_dir(&test_path).unwrap();

        // Should succeed for existing directory
        assert!(verify_removal_permission(&test_path).is_ok());

        // Should fail for non-existent directory
        let non_existent = temp_dir.path().join("non_existent");
        assert!(verify_removal_permission(&non_existent).is_err());
    }

    #[test]
    fn test_check_tool_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path().join("jdk");

        // No warnings for empty directory
        fs::create_dir(&jdk_path).unwrap();
        assert!(check_tool_dependencies(&jdk_path).is_ok());

        // Should succeed even with java binary (just warns)
        let bin_dir = install::bin_directory(&jdk_path);
        fs::create_dir(&bin_dir).unwrap();
        fs::write(bin_dir.join("java"), "mock").unwrap();
        assert!(check_tool_dependencies(&jdk_path).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_permission_check_readonly_parent() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().join("readonly_parent");
        fs::create_dir(&parent).unwrap();

        let test_path = parent.join("jdk");
        fs::create_dir(&test_path).unwrap();

        // Make parent read-only
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&parent, perms).unwrap();

        // Should detect read-only parent
        let result = verify_removal_permission(&test_path);

        // Restore permissions before asserting (so cleanup works)
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&parent, perms).unwrap();

        assert!(result.is_err());
    }
}
