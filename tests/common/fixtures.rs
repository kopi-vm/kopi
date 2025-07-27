/// Shared test fixtures for creating test JDK instances
use kopi::storage::InstalledJdk;
use kopi::version::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Creates a test JDK with automatic path generation
///
/// # Arguments
/// * `distribution` - The distribution name (e.g., "temurin", "corretto")
/// * `version` - The version string (e.g., "21.0.5+11", "17.0.9+9")
///
/// # Returns
/// An InstalledJdk instance with a generated path under `/test/jdks/`
pub fn create_test_jdk(distribution: &str, version: &str) -> InstalledJdk {
    InstalledJdk {
        distribution: distribution.to_string(),
        version: Version::from_str(version).unwrap(),
        path: PathBuf::from(format!("/test/jdks/{distribution}-{version}")),
    }
}

/// Creates a test JDK with a custom path
///
/// # Arguments
/// * `distribution` - The distribution name (e.g., "temurin", "corretto")
/// * `version` - The version string (e.g., "21.0.5+11", "17.0.9+9")
/// * `path` - The custom path for the JDK installation
///
/// # Returns
/// An InstalledJdk instance with the specified path
pub fn create_test_jdk_with_path(distribution: &str, version: &str, path: &str) -> InstalledJdk {
    InstalledJdk {
        distribution: distribution.to_string(),
        version: Version::from_str(version).unwrap(),
        path: PathBuf::from(path),
    }
}

/// Creates multiple test JDKs with different distributions and versions
///
/// # Returns
/// A vector of InstalledJdk instances representing a typical test setup
pub fn create_test_jdk_collection() -> Vec<InstalledJdk> {
    vec![
        create_test_jdk("temurin", "21.0.5+11"),
        create_test_jdk("temurin", "17.0.9+9"),
        create_test_jdk("corretto", "21.0.1"),
        create_test_jdk("corretto", "17.0.5"),
        create_test_jdk("zulu", "11.0.25"),
    ]
}

/// Creates a test JDK with actual filesystem structure
///
/// # Arguments
/// * `kopi_home` - The KOPI_HOME directory path
/// * `distribution` - The distribution name (e.g., "temurin", "corretto")
/// * `version` - The version string (e.g., "21.0.5+11", "17.0.9+9")
///
/// # Returns
/// The path to the created JDK directory
#[allow(dead_code)]
pub fn create_test_jdk_fs(kopi_home: &Path, distribution: &str, version: &str) -> PathBuf {
    let jdk_path = kopi_home
        .join("jdks")
        .join(format!("{distribution}-{version}"));

    let bin_dir = jdk_path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create test tools
    for tool in &["java", "javac", "jar", "jshell", "jdeps", "keytool"] {
        let tool_path = if cfg!(windows) {
            bin_dir.join(format!("{tool}.exe"))
        } else {
            bin_dir.join(tool)
        };
        fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&tool_path).unwrap();
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tool_path, perms).unwrap();
        }
    }

    // Create metadata file
    let metadata = serde_json::json!({
        "distribution": distribution,
        "version": version,
    });
    let metadata_path = jdk_path.join("kopi-metadata.json");
    fs::write(&metadata_path, serde_json::to_string(&metadata).unwrap()).unwrap();

    jdk_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_jdk() {
        let jdk = create_test_jdk("temurin", "21.0.5+11");
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version.to_string(), "21.0.5+11");
        assert_eq!(jdk.path, PathBuf::from("/test/jdks/temurin-21.0.5+11"));
    }

    #[test]
    fn test_create_test_jdk_with_path() {
        let jdk = create_test_jdk_with_path("corretto", "17.0.9", "/custom/path");
        assert_eq!(jdk.distribution, "corretto");
        assert_eq!(jdk.version.to_string(), "17.0.9");
        assert_eq!(jdk.path, PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_create_test_jdk_collection() {
        let jdks = create_test_jdk_collection();
        assert_eq!(jdks.len(), 5);
        assert!(jdks.iter().any(|jdk| jdk.distribution == "temurin"));
        assert!(jdks.iter().any(|jdk| jdk.distribution == "corretto"));
        assert!(jdks.iter().any(|jdk| jdk.distribution == "zulu"));
    }
}
