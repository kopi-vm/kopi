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
use kopi::models::api::{Links, Package};
use kopi::models::distribution::Distribution;
use kopi::storage::JdkRepository;
use std::fs;
use tempfile::TempDir;

struct TestStorage {
    config: KopiConfig,
    _temp_dir: TempDir,
}

impl TestStorage {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        TestStorage {
            config,
            _temp_dir: temp_dir,
        }
    }

    fn manager(&self) -> JdkRepository<'_> {
        JdkRepository::new(&self.config)
    }
}

fn create_test_package() -> Package {
    Package {
        id: "test-package-id".to_string(),
        archive_type: "tar.gz".to_string(),
        distribution: "temurin".to_string(),
        major_version: 21,
        java_version: "21.0.1".to_string(),
        distribution_version: "21.0.1+35.1".to_string(),
        jdk_version: 21,
        directly_downloadable: true,
        filename: "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_35.1.tar.gz".to_string(),
        links: Links {
            pkg_download_redirect: "https://example.com/download".to_string(),
            pkg_info_uri: Some("https://example.com/info".to_string()),
        },
        free_use_in_production: true,
        tck_tested: "yes".to_string(),
        size: 190000000,
        operating_system: "linux".to_string(),
        architecture: Some("x64".to_string()),
        lib_c_type: Some("glibc".to_string()),
        package_type: "jdk".to_string(),
        javafx_bundled: false,
        term_of_support: None,
        release_status: None,
        latest_build_available: None,
    }
}

#[test]
fn test_full_installation_workflow() {
    let test_storage = TestStorage::new();
    let manager = test_storage.manager();
    let distribution = Distribution::Temurin;
    let version = "21.0.1+35.1";

    let context = manager
        .prepare_jdk_installation(&distribution, version)
        .unwrap();

    // Create multiple files at top level to test the multiple entries case
    let test_bin = context.temp_path.join("bin");
    fs::create_dir_all(&test_bin).unwrap();
    fs::write(test_bin.join("java"), "#!/bin/sh\necho java").unwrap();

    // Create another file at top level
    fs::write(context.temp_path.join("README"), "Test JDK").unwrap();

    let final_path = manager.finalize_installation(context).unwrap();
    assert!(final_path.exists());
    assert!(final_path.join("bin").join("java").exists());
    assert!(final_path.join("README").exists());

    let package = create_test_package();
    manager
        .save_jdk_metadata(&distribution, version, &package)
        .unwrap();

    let installed = manager.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].distribution, "temurin");
    assert_eq!(installed[0].version.to_string(), version);
}

#[test]
fn test_failed_installation_cleanup() {
    let test_storage = TestStorage::new();
    let manager = test_storage.manager();
    let distribution = Distribution::Corretto;
    let version = "17.0.9";

    let context = manager
        .prepare_jdk_installation(&distribution, version)
        .unwrap();

    fs::write(context.temp_path.join("partial_file.txt"), "incomplete").unwrap();

    manager.cleanup_failed_installation(&context).unwrap();
    assert!(!context.temp_path.exists());

    let installed = manager.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 0);
}

#[test]
fn test_multiple_jdk_installations() {
    let test_storage = TestStorage::new();
    let manager = test_storage.manager();

    let installations = vec![
        (Distribution::Temurin, "21.0.1"),
        (Distribution::Corretto, "17.0.9"),
        (Distribution::Zulu, "11.0.21"),
    ];

    for (dist, version) in &installations {
        let context = manager.prepare_jdk_installation(dist, version).unwrap();
        fs::create_dir_all(context.temp_path.join("bin")).unwrap();
        manager.finalize_installation(context).unwrap();
    }

    let installed = manager.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 3);

    assert_eq!(installed[0].distribution, "corretto");
    assert_eq!(installed[1].distribution, "temurin");
    assert_eq!(installed[2].distribution, "zulu");
}

#[test]
fn test_jdk_removal() {
    let test_storage = TestStorage::new();
    let manager = test_storage.manager();
    let distribution = Distribution::Temurin;
    let version = "21.0.1";

    let context = manager
        .prepare_jdk_installation(&distribution, version)
        .unwrap();
    fs::create_dir_all(context.temp_path.join("bin")).unwrap();
    let final_path = manager.finalize_installation(context).unwrap();

    let installed = manager.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 1);

    manager.remove_jdk(&final_path).unwrap();

    let installed = manager.list_installed_jdks().unwrap();
    assert_eq!(installed.len(), 0);
}

#[test]
fn test_archive_with_single_directory() {
    let test_storage = TestStorage::new();
    let manager = test_storage.manager();
    let distribution = Distribution::Temurin;
    let version = "21.0.1";

    let context = manager
        .prepare_jdk_installation(&distribution, version)
        .unwrap();

    let jdk_dir = context.temp_path.join("jdk-21.0.1");
    fs::create_dir_all(jdk_dir.join("bin")).unwrap();
    fs::write(jdk_dir.join("bin").join("java"), "java binary").unwrap();

    let final_path = manager.finalize_installation(context).unwrap();

    assert!(final_path.join("bin").join("java").exists());
    let content = fs::read_to_string(final_path.join("bin").join("java")).unwrap();
    assert_eq!(content, "java binary");
}
