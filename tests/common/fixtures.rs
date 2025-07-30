/// Shared test fixtures for creating test JDK filesystem structures
use std::fs;
use std::path::{Path, PathBuf};

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
