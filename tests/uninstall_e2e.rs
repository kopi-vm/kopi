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

use kopi::config::KopiConfig;
use kopi::storage::JdkRepository;
use kopi::uninstall::UninstallHandler;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// End-to-end integration tests for complete uninstall workflows
#[cfg(test)]
mod e2e_tests {
    use super::*;

    struct E2ETestSetup {
        _temp_dir: TempDir,
        config: KopiConfig,
    }

    impl E2ETestSetup {
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let config =
                KopiConfig::new(temp_dir.path().to_path_buf()).expect("Failed to create config");

            // Create necessary directories
            fs::create_dir_all(config.jdks_dir().unwrap()).expect("Failed to create jdks dir");
            fs::create_dir_all(config.cache_dir().unwrap()).expect("Failed to create cache dir");

            Self {
                _temp_dir: temp_dir,
                config,
            }
        }

        fn get_repository(&self) -> JdkRepository<'_> {
            JdkRepository::new(&self.config)
        }

        fn create_full_jdk(&self, distribution: &str, version: &str) -> std::path::PathBuf {
            let jdk_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{distribution}-{version}"));
            fs::create_dir_all(&jdk_path).expect("Failed to create JDK directory");

            // Create a complete JDK structure
            self.create_jdk_structure(&jdk_path, version);

            jdk_path
        }

        fn create_jdk_structure(&self, jdk_path: &Path, version: &str) {
            // Create release file
            let release_content = format!(
                r#"JAVA_VERSION="{version}"
JAVA_VERSION_DATE="2024-01-16"
JAVA_VENDOR="Eclipse Adoptium"
IMPLEMENTOR="Eclipse Adoptium"
JAVA_RUNTIME_VERSION="{version}"
"#
            );
            fs::write(jdk_path.join("release"), release_content)
                .expect("Failed to write release file");

            // Create bin directory with Java executable
            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).expect("Failed to create bin directory");

            #[cfg(windows)]
            let java_exe = "java.exe";
            #[cfg(not(windows))]
            let java_exe = "java";

            let java_content = if cfg!(windows) {
                "REM Mock Java executable\necho Mock Java"
            } else {
                "#!/bin/sh\necho Mock Java"
            };

            fs::write(bin_dir.join(java_exe), java_content)
                .expect("Failed to write Java executable");

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(bin_dir.join(java_exe))
                    .expect("Failed to get metadata")
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(bin_dir.join(java_exe), perms)
                    .expect("Failed to set permissions");
            }

            // Create lib directory with some jar files
            let lib_dir = jdk_path.join("lib");
            fs::create_dir_all(&lib_dir).expect("Failed to create lib directory");
            fs::write(lib_dir.join("rt.jar"), "Mock JAR content")
                .expect("Failed to write JAR file");
            fs::write(lib_dir.join("tools.jar"), "Mock tools JAR")
                .expect("Failed to write tools JAR");

            // Create include directory
            let include_dir = jdk_path.join("include");
            fs::create_dir_all(&include_dir).expect("Failed to create include directory");
            fs::write(include_dir.join("jni.h"), "/* Mock JNI header */")
                .expect("Failed to write JNI header");

            // Create some additional files to test removal
            let conf_dir = jdk_path.join("conf");
            fs::create_dir_all(&conf_dir).expect("Failed to create conf directory");
            fs::write(conf_dir.join("logging.properties"), "# Mock logging config")
                .expect("Failed to write logging config");

            // Create a large file to test progress reporting
            let large_file_path = jdk_path.join("large_file.dat");
            let large_content = vec![0u8; 10 * 1024 * 1024]; // 10MB
            fs::write(large_file_path, large_content).expect("Failed to write large file");
        }

        fn create_partial_jdk(&self, distribution: &str, version: &str) -> std::path::PathBuf {
            let jdk_path = self
                .config
                .jdks_dir()
                .unwrap()
                .join(format!("{distribution}-{version}"));
            fs::create_dir_all(&jdk_path).expect("Failed to create JDK directory");

            // Create only partial structure (missing bin/java)
            fs::write(
                jdk_path.join("release"),
                format!("JAVA_VERSION=\"{version}\""),
            )
            .expect("Failed to write release file");

            let bin_dir = jdk_path.join("bin");
            fs::create_dir_all(&bin_dir).expect("Failed to create bin directory");
            // Note: NOT creating java executable to simulate partial removal

            jdk_path
        }

        fn create_temp_removal_dir(&self, distribution: &str, version: &str) -> std::path::PathBuf {
            let temp_name = format!(".{distribution}-{version}.removing");
            let temp_path = self.config.jdks_dir().unwrap().join(temp_name);
            fs::create_dir_all(&temp_path).expect("Failed to create temp removal directory");

            // Add some content
            fs::write(temp_path.join("test.txt"), "temp content")
                .expect("Failed to write temp file");

            temp_path
        }

        fn verify_complete_removal(&self, jdk_path: &Path) {
            assert!(
                !jdk_path.exists(),
                "JDK directory should be completely removed"
            );

            // Check that no temporary files remain
            let parent = jdk_path.parent().unwrap();
            if let Ok(entries) = fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();
                    assert!(
                        !name.contains(".removing"),
                        "Temporary removal directory should be cleaned up"
                    );
                }
            }
        }
    }

    #[test]
    fn test_complete_uninstall_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create a complete JDK
        let jdk_path = setup.create_full_jdk("temurin", "21.0.5+11");
        assert!(jdk_path.exists());

        // Verify JDK is listed as installed
        let repository = setup.get_repository();
        let installed_jdks = repository.list_installed_jdks().unwrap();
        assert_eq!(installed_jdks.len(), 1);
        assert_eq!(installed_jdks[0].distribution, "temurin");
        assert_eq!(installed_jdks[0].version.to_string(), "21.0.5+11");

        // Perform uninstall
        let result = handler.uninstall_jdk("temurin@21.0.5+11", false);
        assert!(result.is_ok(), "Uninstall should succeed: {result:?}");

        // Verify complete removal
        setup.verify_complete_removal(&jdk_path);

        // Verify JDK is no longer listed
        let repository = setup.get_repository();
        let installed_jdks = repository.list_installed_jdks().unwrap();
        assert!(installed_jdks.is_empty());
    }

    #[test]
    fn test_dry_run_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create a JDK
        let jdk_path = setup.create_full_jdk("corretto", "17.0.9");
        assert!(jdk_path.exists());

        // Perform dry run
        let result = handler.uninstall_jdk("corretto@17.0.9", true);
        assert!(result.is_ok(), "Dry run should succeed: {result:?}");

        // Verify JDK still exists
        assert!(jdk_path.exists(), "JDK should still exist after dry run");

        let repository = setup.get_repository();
        let installed_jdks = repository.list_installed_jdks().unwrap();
        assert_eq!(installed_jdks.len(), 1);
    }

    #[test]
    fn test_multiple_jdk_error_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create multiple JDKs with same major version
        let jdk1_path = setup.create_full_jdk("temurin", "21.0.1");
        let jdk2_path = setup.create_full_jdk("temurin", "21.0.2");
        let jdk3_path = setup.create_full_jdk("corretto", "21.0.1");

        // Try to uninstall with ambiguous version
        let result = handler.uninstall_jdk("21", false);
        assert!(result.is_err(), "Should fail with multiple matches");

        // Verify all JDKs still exist
        assert!(jdk1_path.exists());
        assert!(jdk2_path.exists());
        assert!(jdk3_path.exists());

        // Verify specific uninstall works
        let result = handler.uninstall_jdk("temurin@21.0.1", false);
        assert!(
            result.is_ok(),
            "Specific uninstall should succeed: {result:?}"
        );

        // Verify only the specific JDK was removed
        assert!(!jdk1_path.exists());
        assert!(jdk2_path.exists());
        assert!(jdk3_path.exists());
    }

    #[test]
    fn test_nonexistent_jdk_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Try to uninstall non-existent JDK
        let result = handler.uninstall_jdk("nonexistent@1.0.0", false);
        assert!(result.is_err(), "Should fail for non-existent JDK");

        // Verify error is appropriate
        match result.unwrap_err() {
            kopi::error::KopiError::JdkNotInstalled { .. } => {
                // Expected error type
            }
            other => panic!("Unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn test_recovery_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create scenarios that need recovery
        let partial_jdk = setup.create_partial_jdk("temurin", "21.0.1");
        let temp_removal = setup.create_temp_removal_dir("corretto", "17.0.9");

        // Verify problematic state
        assert!(partial_jdk.exists());
        assert!(temp_removal.exists());

        // Perform recovery
        let result = handler.recover_from_failures(false);
        assert!(result.is_ok(), "Recovery should succeed: {result:?}");

        // Verify cleanup
        assert!(!partial_jdk.exists(), "Partial JDK should be cleaned up");
        assert!(
            !temp_removal.exists(),
            "Temp removal directory should be cleaned up"
        );
    }

    #[test]
    fn test_force_recovery_workflow() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create a partial JDK
        let partial_jdk = setup.create_partial_jdk("temurin", "21.0.1");
        assert!(partial_jdk.exists());

        // Make files read-only to simulate stubborn files
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&partial_jdk).unwrap().permissions();
            perms.set_mode(0o555); // Read-only but traversable
            fs::set_permissions(&partial_jdk, perms).unwrap();
        }

        #[cfg(windows)]
        {
            // Set read-only attribute on Windows using WinAPI
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use winapi::um::fileapi::{
                GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
            };
            use winapi::um::winnt::FILE_ATTRIBUTE_READONLY;

            fn set_readonly_windows(path: &std::path::Path) -> std::io::Result<()> {
                let path_wide: Vec<u16> = OsStr::new(path)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                unsafe {
                    // Get current attributes
                    let current_attrs = GetFileAttributesW(path_wide.as_ptr());
                    if current_attrs == INVALID_FILE_ATTRIBUTES {
                        return Err(std::io::Error::last_os_error());
                    }

                    // Add READ_ONLY attribute
                    let new_attrs = current_attrs | FILE_ATTRIBUTE_READONLY;

                    // Set attributes
                    if SetFileAttributesW(path_wide.as_ptr(), new_attrs) == 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }

                Ok(())
            }

            // Set read-only on the directory itself
            set_readonly_windows(&partial_jdk).unwrap();

            // Also set read-only on all files within the directory to make removal more difficult
            if let Ok(entries) = fs::read_dir(&partial_jdk) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = set_readonly_windows(&path); // Ignore individual failures
                    }
                }
            }
        }

        // Perform force recovery
        let result = handler.recover_from_failures(true);
        assert!(result.is_ok(), "Force recovery should succeed: {result:?}");

        // Verify cleanup
        assert!(!partial_jdk.exists(), "Partial JDK should be force-cleaned");
    }

    #[test]
    fn test_concurrent_operation_handling() {
        let setup = E2ETestSetup::new();

        // Create a JDK
        let jdk_path = setup.create_full_jdk("temurin", "21.0.1");
        assert!(jdk_path.exists());

        // Simulate concurrent operations by creating multiple handlers
        let repository = setup.get_repository();
        let handler1 = UninstallHandler::new(&repository);
        let handler2 = UninstallHandler::new(&repository);

        // Try to perform operations simultaneously
        // Note: This is a simplified test - real concurrent testing would require threading
        let result1 = handler1.uninstall_jdk("temurin@21.0.1", false);
        let result2 = handler2.uninstall_jdk("temurin@21.0.1", false);

        // One should succeed, one should fail
        let successes = [&result1, &result2].iter().filter(|r| r.is_ok()).count();

        // At least one should succeed (the first one to complete)
        assert!(successes >= 1, "At least one operation should succeed");

        // Verify JDK is removed
        assert!(
            !jdk_path.exists(),
            "JDK should be removed after successful operation"
        );
    }

    #[test]
    fn test_platform_specific_scenarios() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create a JDK with platform-specific content
        let jdk_path = setup.create_full_jdk("temurin", "21.0.1");

        // Add platform-specific files
        #[cfg(unix)]
        {
            // Create symbolic links on Unix
            let link_path = jdk_path.join("current_link");
            std::os::unix::fs::symlink(&jdk_path, &link_path).unwrap();
            assert!(link_path.exists());
        }

        #[cfg(windows)]
        {
            // Create files with special attributes on Windows
            let special_file = jdk_path.join("special.txt");
            fs::write(&special_file, "special content").unwrap();

            // Set file attributes (this would be more complex in real scenarios)
            assert!(special_file.exists());
        }

        // Perform uninstall
        let result = handler.uninstall_jdk("temurin@21.0.1", false);
        assert!(
            result.is_ok(),
            "Platform-specific uninstall should succeed: {result:?}"
        );

        // Verify complete removal including platform-specific cleanup
        setup.verify_complete_removal(&jdk_path);

        #[cfg(unix)]
        {
            // Verify symbolic link cleanup
            let parent = jdk_path.parent().unwrap();
            if let Ok(entries) = fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(metadata) = fs::symlink_metadata(&path)
                        && metadata.file_type().is_symlink()
                    {
                        // Check if it's an orphaned symlink
                        assert!(
                            fs::metadata(&path).is_ok(),
                            "No orphaned symlinks should remain"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_large_jdk_removal_with_progress() {
        let setup = E2ETestSetup::new();
        let repository = setup.get_repository();
        let handler = UninstallHandler::new(&repository);

        // Create a large JDK (the create_full_jdk already includes a 10MB file)
        let jdk_path = setup.create_full_jdk("temurin", "21.0.1");

        // Add more large files to exceed 100MB threshold for progress bar
        for i in 0..12 {
            let large_file = jdk_path.join(format!("large_{i}.dat"));
            let content = vec![0u8; 10 * 1024 * 1024]; // 10MB each
            fs::write(large_file, content).unwrap();
        }

        // Verify size is large
        let repository = setup.get_repository();
        let size = repository.get_jdk_size(&jdk_path).unwrap();
        assert!(size > 100 * 1024 * 1024, "JDK should be larger than 100MB");

        // Perform uninstall (should show progress bar)
        let result = handler.uninstall_jdk("temurin@21.0.1", false);
        assert!(
            result.is_ok(),
            "Large JDK uninstall should succeed: {result:?}"
        );

        // Verify complete removal
        setup.verify_complete_removal(&jdk_path);
    }
}
