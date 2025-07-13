use kopi::config::KopiConfig;
use kopi::storage::JdkRepository;
use kopi::uninstall::batch::BatchUninstaller;
use kopi::uninstall::selection::JdkSelector;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct TestSetup {
    _temp_dir: TempDir,
    config: KopiConfig,
}

impl TestSetup {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();

        // Create jdks directory
        fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();

        TestSetup {
            _temp_dir: temp_dir,
            config,
        }
    }

    fn create_mock_jdk(&self, distribution: &str, version: &str) -> PathBuf {
        let jdk_path = self
            .config
            .jdks_dir()
            .unwrap()
            .join(format!("{distribution}-{version}"));
        fs::create_dir_all(&jdk_path).unwrap();

        // Create some mock files
        fs::write(jdk_path.join("release"), "JAVA_VERSION=\"21\"").unwrap();
        fs::create_dir_all(jdk_path.join("bin")).unwrap();
        fs::write(jdk_path.join("bin/java"), "#!/bin/sh\necho mock java").unwrap();

        // Create larger files for size testing
        fs::write(jdk_path.join("lib.jar"), vec![0u8; 10 * 1024 * 1024]).unwrap(); // 10MB

        jdk_path
    }
}

#[test]
fn test_batch_uninstall_multiple_jdks() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);
    let batch_uninstaller = BatchUninstaller::new(&repository);

    // Create multiple test JDKs
    let jdk1_path = setup.create_mock_jdk("temurin", "21.0.5+11");
    let jdk2_path = setup.create_mock_jdk("temurin", "17.0.9+9");
    let jdk3_path = setup.create_mock_jdk("corretto", "21.0.1");

    // Verify JDKs exist
    assert!(jdk1_path.exists());
    assert!(jdk2_path.exists());
    assert!(jdk3_path.exists());

    // Get installed JDKs
    let jdks = repository.list_installed_jdks().unwrap();
    assert_eq!(jdks.len(), 3);

    // Perform batch removal (force=true to skip confirmation)
    let result = batch_uninstaller.uninstall_batch(jdks, true, false);
    assert!(result.is_ok());

    // Verify all JDKs are removed
    assert!(!jdk1_path.exists());
    assert!(!jdk2_path.exists());
    assert!(!jdk3_path.exists());

    // Verify listing shows no JDKs
    let remaining_jdks = repository.list_installed_jdks().unwrap();
    assert!(remaining_jdks.is_empty());
}

#[test]
fn test_batch_uninstall_dry_run() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);
    let batch_uninstaller = BatchUninstaller::new(&repository);

    // Create test JDKs
    let jdk1_path = setup.create_mock_jdk("temurin", "21.0.5+11");
    let jdk2_path = setup.create_mock_jdk("corretto", "17.0.9");

    // Get installed JDKs
    let jdks = repository.list_installed_jdks().unwrap();
    assert_eq!(jdks.len(), 2);

    // Perform dry run
    let result = batch_uninstaller.uninstall_batch(jdks, true, true);
    assert!(result.is_ok());

    // Verify JDKs still exist
    assert!(jdk1_path.exists());
    assert!(jdk2_path.exists());
}

#[test]
fn test_uninstall_all_by_distribution() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);
    let batch_uninstaller = BatchUninstaller::new(&repository);

    // Create JDKs from different distributions
    let temurin1 = setup.create_mock_jdk("temurin", "21.0.5+11");
    let temurin2 = setup.create_mock_jdk("temurin", "17.0.9+9");
    let corretto = setup.create_mock_jdk("corretto", "21.0.1");

    // Uninstall all temurin JDKs
    let result = batch_uninstaller.uninstall_all(Some("temurin"), true, false);
    assert!(result.is_ok());

    // Verify only temurin JDKs are removed
    assert!(!temurin1.exists());
    assert!(!temurin2.exists());
    assert!(corretto.exists());

    // Verify corretto is still listed
    let remaining_jdks = repository.list_installed_jdks().unwrap();
    assert_eq!(remaining_jdks.len(), 1);
    assert_eq!(remaining_jdks[0].distribution, "corretto");
}

#[test]
fn test_uninstall_all_empty() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);
    let batch_uninstaller = BatchUninstaller::new(&repository);

    // Attempt to uninstall when no JDKs are installed
    let result = batch_uninstaller.uninstall_all(None, true, false);
    assert!(result.is_err());
}

#[test]
fn test_selection_filter_by_distribution() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);

    // Create JDKs from different distributions
    setup.create_mock_jdk("temurin", "21.0.5+11");
    setup.create_mock_jdk("temurin", "17.0.9+9");
    setup.create_mock_jdk("corretto", "21.0.1");
    setup.create_mock_jdk("zulu", "21.0.0");

    // Get all installed JDKs
    let all_jdks = repository.list_installed_jdks().unwrap();
    assert_eq!(all_jdks.len(), 4);

    // Filter by distribution
    let temurin_jdks = JdkSelector::filter_by_distribution(all_jdks.clone(), "temurin");
    assert_eq!(temurin_jdks.len(), 2);
    assert!(temurin_jdks.iter().all(|jdk| jdk.distribution == "temurin"));

    let corretto_jdks = JdkSelector::filter_by_distribution(all_jdks.clone(), "corretto");
    assert_eq!(corretto_jdks.len(), 1);
    assert_eq!(corretto_jdks[0].distribution, "corretto");

    let nonexistent = JdkSelector::filter_by_distribution(all_jdks, "nonexistent");
    assert!(nonexistent.is_empty());
}

#[test]
fn test_partial_failure_recovery() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);
    let batch_uninstaller = BatchUninstaller::new(&repository);

    // Create test JDKs
    let jdk1_path = setup.create_mock_jdk("temurin", "21.0.5+11");
    let jdk2_path = setup.create_mock_jdk("corretto", "17.0.9");

    // Make one JDK read-only to simulate permission error
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&jdk2_path).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&jdk2_path, perms).unwrap();
    }

    // Get installed JDKs
    let jdks = repository.list_installed_jdks().unwrap();

    // Attempt batch removal
    let _result = batch_uninstaller.uninstall_batch(jdks, true, false);

    // Should succeed partially
    assert!(!jdk1_path.exists());

    // Cleanup - restore permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&jdk2_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&jdk2_path, perms).unwrap();
    }
}

#[test]
fn test_size_calculation() {
    let setup = TestSetup::new();
    let repository = JdkRepository::new(&setup.config);

    // Create JDKs with known sizes
    setup.create_mock_jdk("temurin", "21.0.5+11");
    setup.create_mock_jdk("corretto", "17.0.9");

    let jdks = repository.list_installed_jdks().unwrap();

    // Calculate total size
    let mut total_size = 0u64;
    for jdk in &jdks {
        total_size += repository.get_jdk_size(&jdk.path).unwrap();
    }

    // Each JDK has ~10MB from lib.jar plus other files
    assert!(total_size > 20 * 1024 * 1024); // At least 20MB total
}

// User interaction tests would require mocking stdin/stdout
// These are better handled in unit tests with mocks
