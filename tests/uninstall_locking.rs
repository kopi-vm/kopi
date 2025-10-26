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

mod common;

use assert_cmd::Command;
use common::TestHomeGuard;
use kopi::config::KopiConfig;
use kopi::locking::{
    InstalledScopeResolver, LockController, ScopedPackageLockGuard,
    installation_lock_scope_from_package,
};
use kopi::models::api::{Links, Package};
use kopi::storage::{InstallationMetadata, JdkMetadataWithInstallation, JdkRepository};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};

struct InstalledFixture {
    spec: String,
    install_path: PathBuf,
    metadata_path: PathBuf,
    metadata: JdkMetadataWithInstallation,
}

#[test]
#[serial]
fn uninstall_requires_force_for_active_global() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home().to_path_buf();

    let fixture = provision_installed_jdk(&kopi_home, "temurin", "21.0.5+11");
    let global_file = kopi_home.join("version");
    fs::write(&global_file, &fixture.spec).unwrap();

    let mut blocked = test_command(&kopi_home);
    blocked.arg("uninstall").arg(&fixture.spec);

    blocked
        .write_stdin("y\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("active globally"))
        .stderr(predicate::str::contains("Use --force"));

    let mut forced = test_command(&kopi_home);
    forced.arg("uninstall").arg(&fixture.spec).arg("--force");

    forced
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Proceeding with --force: global default set via",
        ))
        .stdout(predicate::str::contains("configured as temurin@21.0.5+11"));

    assert!(
        !fixture.install_path.exists(),
        "installation directory should be removed when forcing uninstall"
    );
    assert!(
        !fixture.metadata_path.exists(),
        "metadata should be removed when forcing uninstall"
    );
}

#[test]
#[serial]
fn uninstall_requires_force_for_active_project() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home().to_path_buf();

    let fixture = provision_installed_jdk(&kopi_home, "temurin", "17.0.9+9");
    let project_dir = kopi_home.join("projects/sample");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join(".kopi-version"), &fixture.spec).unwrap();

    let mut blocked = test_command(&kopi_home);
    blocked
        .current_dir(&project_dir)
        .arg("uninstall")
        .arg(&fixture.spec);

    blocked
        .write_stdin("y\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("configured for this project"))
        .stderr(predicate::str::contains("Use --force"));

    let mut forced = test_command(&kopi_home);
    forced
        .current_dir(&project_dir)
        .arg("uninstall")
        .arg(&fixture.spec)
        .arg("--force");

    forced
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Proceeding with --force: project default set via",
        ))
        .stdout(predicate::str::contains("configured as temurin@17.0.9+9"));

    assert!(
        !fixture.install_path.exists(),
        "installation directory should be removed when forcing uninstall"
    );
    assert!(
        !fixture.metadata_path.exists(),
        "metadata should be removed when forcing uninstall"
    );
}

#[test]
#[serial]
fn uninstall_blocks_when_peer_uninstall_holds_lock() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home().to_path_buf();

    let fixture = provision_installed_jdk(&kopi_home, "temurin", "21.0.2+12");

    let config = KopiConfig::new(kopi_home.clone()).unwrap();
    let repository = JdkRepository::new(&config);
    let installed = repository
        .list_installed_jdks()
        .unwrap()
        .into_iter()
        .find(|jdk| jdk.distribution == "temurin")
        .expect("temurin install should be present");

    let resolver = InstalledScopeResolver::new(&repository);
    let lock_scope = resolver.resolve(&installed).unwrap();
    let scope_label = lock_scope.label();

    let controller =
        LockController::with_default_inspector(config.kopi_home().to_path_buf(), &config.locking);
    let guard =
        ScopedPackageLockGuard::new(&controller, controller.acquire(lock_scope.clone()).unwrap());

    let mut blocked = test_command(&kopi_home);
    blocked
        .arg("uninstall")
        .arg(&fixture.spec)
        .arg("--force")
        .env("KOPI_LOCK_TIMEOUT", "0");

    blocked
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Failed to acquire"))
        .stderr(predicate::str::contains(&scope_label));

    drop(guard);

    assert!(
        fixture.install_path.exists(),
        "installation directory should remain when uninstall fails to acquire lock"
    );
    assert!(
        fixture.metadata_path.exists(),
        "metadata should remain when uninstall fails to acquire lock"
    );

    let mut retry = test_command(&kopi_home);
    retry
        .arg("uninstall")
        .arg(&fixture.spec)
        .arg("--force")
        .env_remove("KOPI_LOCK_TIMEOUT");

    retry
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully uninstalled"));

    assert!(
        !fixture.install_path.exists(),
        "installation directory should be removed after lock release"
    );
    assert!(
        !fixture.metadata_path.exists(),
        "metadata file should be removed after successful uninstall"
    );
}

#[test]
#[serial]
fn uninstall_blocks_when_install_lock_is_active() {
    let test_home = TestHomeGuard::new();
    test_home.setup_kopi_structure();
    let kopi_home = test_home.kopi_home().to_path_buf();

    let fixture = provision_installed_jdk(&kopi_home, "temurin", "21.0.3+13");

    let config = KopiConfig::new(kopi_home.clone()).unwrap();
    let controller =
        LockController::with_default_inspector(config.kopi_home().to_path_buf(), &config.locking);

    let install_scope = installation_lock_scope_from_package(&fixture.metadata.package).unwrap();
    let scope_label = install_scope.label();

    let guard = ScopedPackageLockGuard::new(
        &controller,
        controller.acquire(install_scope.clone()).unwrap(),
    );

    let mut blocked = test_command(&kopi_home);
    blocked
        .arg("uninstall")
        .arg(&fixture.spec)
        .arg("--force")
        .env("KOPI_LOCK_TIMEOUT", "0");

    blocked
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Failed to acquire"))
        .stderr(predicate::str::contains(&scope_label));

    drop(guard);

    assert!(
        fixture.install_path.exists(),
        "installation should remain when install lock blocks uninstall"
    );
    assert!(
        fixture.metadata_path.exists(),
        "metadata should remain when install lock blocks uninstall"
    );

    let mut retry = test_command(&kopi_home);
    retry
        .arg("uninstall")
        .arg(&fixture.spec)
        .arg("--force")
        .env_remove("KOPI_LOCK_TIMEOUT");

    retry
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully uninstalled"));

    assert!(
        !fixture.install_path.exists(),
        "installation directory should be removed after install lock release"
    );
    assert!(
        !fixture.metadata_path.exists(),
        "metadata file should be removed after successful uninstall"
    );
}

fn provision_installed_jdk(
    kopi_home: &Path,
    distribution: &str,
    version: &str,
) -> InstalledFixture {
    let slug = format!("{distribution}-{version}");
    let install_path = kopi_home.join("jdks").join(&slug);
    fs::create_dir_all(&install_path).unwrap();
    fs::create_dir_all(install_path.join("bin")).unwrap();
    fs::create_dir_all(install_path.join("lib")).unwrap();
    fs::write(
        install_path.join("release"),
        format!("JAVA_VERSION=\"{version}\""),
    )
    .unwrap();
    fs::write(
        install_path.join("bin").join("java"),
        "#!/bin/sh\necho mock java\n",
    )
    .unwrap();

    let metadata = build_metadata(distribution, version);
    let metadata_path = kopi_home.join("jdks").join(format!("{slug}.meta.json"));
    fs::write(
        &metadata_path,
        format!("{}\n", serde_json::to_string_pretty(&metadata).unwrap()),
    )
    .unwrap();

    InstalledFixture {
        spec: format!("{distribution}@{version}"),
        install_path,
        metadata_path,
        metadata,
    }
}

fn build_metadata(distribution: &str, version: &str) -> JdkMetadataWithInstallation {
    let package = Package {
        id: format!("{distribution}-{version}"),
        archive_type: "tar.gz".to_string(),
        distribution: distribution.to_string(),
        major_version: parse_major_version(version),
        java_version: version.to_string(),
        distribution_version: version.to_string(),
        jdk_version: parse_major_version(version),
        directly_downloadable: true,
        filename: format!("{distribution}-{version}.tar.gz"),
        links: Links {
            pkg_download_redirect: "https://example.com/download".to_string(),
            pkg_info_uri: Some("https://example.com/info".to_string()),
        },
        free_use_in_production: true,
        tck_tested: "yes".to_string(),
        size: 1024,
        operating_system: "linux".to_string(),
        architecture: Some("x64".to_string()),
        lib_c_type: Some("gnu".to_string()),
        package_type: "JDK".to_string(),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    let installation_metadata = InstallationMetadata {
        java_home_suffix: String::new(),
        structure_type: "direct".to_string(),
        platform: "linux_x64".to_string(),
        metadata_version: 1,
    };

    JdkMetadataWithInstallation {
        package,
        installation_metadata,
    }
}

fn parse_major_version(version: &str) -> u32 {
    version
        .split(['.', '+', '-'])
        .find(|segment| !segment.is_empty())
        .and_then(|segment| segment.parse::<u32>().ok())
        .unwrap_or(21)
}

fn test_command(kopi_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", kopi_home);
    if let Some(home) = kopi_home.parent() {
        cmd.env("HOME", home);
    }
    cmd.env_remove("KOPI_JAVA_VERSION");
    cmd
}
