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
use tempfile::TempDir;

#[test]
fn test_uninstall_javafx_jdk() {
    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
    let repository = JdkRepository::new(&config);
    let handler = UninstallHandler::new(&repository);

    // Create JDK directories with JavaFX suffix
    let jdks_dir = config.jdks_dir().unwrap();
    fs::create_dir_all(&jdks_dir).unwrap();

    // Create regular liberica JDK
    let regular_jdk_path = jdks_dir.join("liberica-21.0.5");
    fs::create_dir_all(&regular_jdk_path).unwrap();
    fs::write(regular_jdk_path.join("release"), "JAVA_VERSION=\"21.0.5\"").unwrap();

    // Create JavaFX liberica JDK
    let javafx_jdk_path = jdks_dir.join("liberica-21.0.5-fx");
    fs::create_dir_all(&javafx_jdk_path).unwrap();
    fs::write(javafx_jdk_path.join("release"), "JAVA_VERSION=\"21.0.5\"").unwrap();

    // Verify both JDKs are listed
    let installed = repository.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 2);

    // Resolve JDKs for "liberica@21+fx" - should find the JavaFX one
    let matches = handler.resolve_jdks_to_uninstall("liberica@21+fx").unwrap();
    assert_eq!(matches.len(), 1, "Should find exactly one JavaFX JDK");
    assert_eq!(matches[0].distribution, "liberica");
    assert!(matches[0].javafx_bundled, "Should be JavaFX bundled");
    assert!(matches[0].path.ends_with("liberica-21.0.5-fx"));

    // Resolve JDKs for "liberica@21" without +fx - should find both
    let matches = handler.resolve_jdks_to_uninstall("liberica@21").unwrap();
    assert_eq!(
        matches.len(),
        2,
        "Should find both JDKs when +fx not specified"
    );
    assert_eq!(matches[0].distribution, "liberica");
    assert_eq!(matches[1].distribution, "liberica");

    // Test uninstalling the JavaFX version
    let result = handler.uninstall_jdk("liberica@21+fx", false);
    assert!(result.is_ok(), "Should successfully uninstall JavaFX JDK");

    // Verify JavaFX JDK was removed
    assert!(!javafx_jdk_path.exists(), "JavaFX JDK should be removed");
    assert!(regular_jdk_path.exists(), "Regular JDK should still exist");

    // Verify only one JDK remains
    let remaining = repository.list_installed_jdks().unwrap();
    assert_eq!(remaining.len(), 1);
    assert!(!remaining[0].javafx_bundled);
}

#[test]
fn test_uninstall_javafx_with_simplified_version() {
    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
    let repository = JdkRepository::new(&config);
    let handler = UninstallHandler::new(&repository);

    // Create JDK directory with JavaFX suffix but simpler version
    let jdks_dir = config.jdks_dir().unwrap();
    fs::create_dir_all(&jdks_dir).unwrap();

    // Create JavaFX liberica JDK with simplified version
    let javafx_jdk_path = jdks_dir.join("liberica-21-fx");
    fs::create_dir_all(&javafx_jdk_path).unwrap();
    fs::write(javafx_jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();

    // Verify JDK is listed
    let installed = repository.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].version.to_string(), "21");
    assert!(installed[0].javafx_bundled);

    // Resolve JDKs for "liberica@21+fx" - should find it
    let matches = handler.resolve_jdks_to_uninstall("liberica@21+fx").unwrap();
    assert_eq!(matches.len(), 1, "Should find the JavaFX JDK");
    assert!(matches[0].javafx_bundled);

    // Test uninstalling
    let result = handler.uninstall_jdk("liberica@21+fx", false);
    assert!(result.is_ok(), "Should successfully uninstall JavaFX JDK");

    // Verify JDK was removed
    assert!(!javafx_jdk_path.exists(), "JavaFX JDK should be removed");
}

#[test]
fn test_uninstall_with_fx_no_match() {
    let temp_dir = TempDir::new().unwrap();
    let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
    let repository = JdkRepository::new(&config);
    let handler = UninstallHandler::new(&repository);

    // Create regular JDK without JavaFX
    let jdks_dir = config.jdks_dir().unwrap();
    fs::create_dir_all(&jdks_dir).unwrap();

    let regular_jdk_path = jdks_dir.join("liberica-21.0.5");
    fs::create_dir_all(&regular_jdk_path).unwrap();
    fs::write(regular_jdk_path.join("release"), "JAVA_VERSION=\"21.0.5\"").unwrap();

    // Try to resolve JavaFX version that doesn't exist
    let matches = handler.resolve_jdks_to_uninstall("liberica@21+fx").unwrap();
    assert_eq!(matches.len(), 0, "Should not find any JavaFX JDK");

    // Try to uninstall JavaFX version that doesn't exist
    let result = handler.uninstall_jdk("liberica@21+fx", false);
    assert!(
        result.is_err(),
        "Should fail to uninstall non-existent JavaFX JDK"
    );
}
