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
use crate::models::package::ChecksumType;
use crate::platform::file_ops;
use digest::{Digest, DynDigest};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const CHUNK_SIZE: usize = 8192;

pub fn verify_checksum(
    file_path: &Path,
    expected_checksum: &str,
    checksum_type: ChecksumType,
) -> Result<()> {
    let actual = calculate_checksum(file_path, checksum_type)?;

    if actual != expected_checksum {
        return Err(KopiError::ValidationError(format!(
            "Checksum verification failed for {file_path:?}. Expected: {expected_checksum}, \
             Actual: {actual}"
        )));
    }

    log::debug!("Checksum verified successfully for {file_path:?} using {checksum_type:?}");
    Ok(())
}

pub fn calculate_checksum(file_path: &Path, checksum_type: ChecksumType) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut buffer = vec![0; CHUNK_SIZE];

    // Create appropriate hasher based on checksum type
    let mut hasher: Box<dyn DynDigest> = match checksum_type {
        ChecksumType::Sha1 => Box::new(Sha1::new()),
        ChecksumType::Sha256 => Box::new(Sha256::new()),
        ChecksumType::Sha512 => Box::new(Sha512::new()),
        ChecksumType::Md5 => {
            // MD5 requires special handling because md5 crate doesn't implement DynDigest
            let mut file_contents = Vec::new();
            file.read_to_end(&mut file_contents)?;
            let digest = md5::compute(&file_contents);
            return Ok(hex::encode(digest.0));
        }
    };

    // Process file in chunks
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => DynDigest::update(&mut *hasher, &buffer[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }

    // Finalize and format the digest
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn verify_https_security(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(KopiError::SecurityError(format!(
            "Insecure URL: {url}. Only HTTPS URLs are allowed for JDK downloads"
        )));
    }

    // Additional URL validation
    if url.contains("..") || url.contains("://localhost") || url.contains("://127.0.0.1") {
        return Err(KopiError::SecurityError(format!(
            "Suspicious URL detected: {url}"
        )));
    }

    Ok(())
}

pub fn is_trusted_domain(url: &str) -> bool {
    // List of trusted JDK download domains
    const TRUSTED_DOMAINS: &[&str] = &[
        "https://api.foojay.io",
        "https://download.oracle.com",
        "https://github.com/adoptium",
        "https://github.com/AdoptOpenJDK",
        "https://corretto.aws",
        "https://cdn.azul.com",
        "https://download.java.net",
        "https://downloads.gradle.org",
        "https://download.bell-sw.com",
        "https://github.com/bell-sw",
        "https://github.com/graalvm",
        "https://download.graalvm.org",
        "https://builds.openlogic.com",
        "https://github.com/dragonwell-project",
        "https://github.com/SAP",
        "https://github.com/SapMachine",
        "https://download.eclipse.org",
        "https://adoptium.net",
    ];

    TRUSTED_DOMAINS
        .iter()
        .any(|&domain| url.starts_with(domain))
}

pub fn audit_log(action: &str, details: &str) {
    // In production, this would write to a secure audit log
    log::info!("SECURITY AUDIT: {action} - {details}");
}

pub fn verify_file_permissions(path: &Path) -> Result<()> {
    let is_secure = file_ops::check_file_permissions(path)?;

    if !is_secure {
        return Err(KopiError::SecurityError(format!(
            "File {path:?} has insecure permissions"
        )));
    }

    Ok(())
}

pub fn sanitize_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Check for path traversal attempts
    if path_str.contains("..") || path_str.contains("~") {
        return Err(KopiError::SecurityError(format!(
            "Potential path traversal detected in: {path:?}"
        )));
    }

    // Check for absolute paths that might escape the kopi directory
    if path.is_absolute() {
        let path_str = path.to_string_lossy();
        if !path_str.contains(".kopi") {
            return Err(KopiError::SecurityError(format!(
                "Path {path:?} is outside of kopi directory"
            )));
        }
    }

    Ok(())
}

/// Set file permissions to read-only for security
/// This is especially important for JDK files after installation
pub fn secure_file_permissions(path: &Path) -> Result<()> {
    file_ops::set_secure_permissions(path)?;

    audit_log(
        "SECURE_PERMISSIONS",
        &format!("Set secure permissions on {path:?}"),
    );

    Ok(())
}

/// Recursively secure all files in a directory
pub fn secure_directory_permissions(dir: &Path) -> Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            secure_file_permissions(path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_checksum_with_different_types() {
        // Create a temporary file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, World!").unwrap();
        temp_file.flush().unwrap();

        // Test SHA1
        let sha1_checksum = calculate_checksum(temp_file.path(), ChecksumType::Sha1).unwrap();
        assert_eq!(sha1_checksum, "0a0a9f2a6772942557ab5355d76af442f8f65e01");

        // Test SHA256
        let sha256_checksum = calculate_checksum(temp_file.path(), ChecksumType::Sha256).unwrap();
        assert_eq!(
            sha256_checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );

        // Test SHA512
        let sha512_checksum = calculate_checksum(temp_file.path(), ChecksumType::Sha512).unwrap();
        assert_eq!(
            sha512_checksum,
            "374d794a95cdcfd8b35993185fef9ba368f160d8daf432d08ba9f1ed1e5abe6cc69291e0fa2fe0006a52570ef18c19def4e617c33ce52ef0a6e5fbe318cb0387"
        );

        // Test MD5
        let md5_checksum = calculate_checksum(temp_file.path(), ChecksumType::Md5).unwrap();
        assert_eq!(md5_checksum, "65a8e27d8879283831b664bd8b7f0ad4");
    }

    #[test]
    fn test_verify_checksum_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content").unwrap();
        temp_file.flush().unwrap();

        let expected = "9d9595c5d94fb65b824f56e9999527dba9542481580d69feb89056aabaa0aa87";

        assert!(verify_checksum(temp_file.path(), expected, ChecksumType::Sha256).is_ok());
    }

    #[test]
    fn test_verify_checksum_failure() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content").unwrap();
        temp_file.flush().unwrap();

        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";

        assert!(verify_checksum(temp_file.path(), wrong_checksum, ChecksumType::Sha256).is_err());
    }

    #[test]
    fn test_verify_checksum_with_different_algorithms() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content").unwrap();
        temp_file.flush().unwrap();

        // Test with SHA1
        assert!(
            verify_checksum(
                temp_file.path(),
                "bca20547e94049e1ffea27223581c567022a5774",
                ChecksumType::Sha1
            )
            .is_ok()
        );

        // Test with SHA256
        assert!(
            verify_checksum(
                temp_file.path(),
                "9d9595c5d94fb65b824f56e9999527dba9542481580d69feb89056aabaa0aa87",
                ChecksumType::Sha256
            )
            .is_ok()
        );

        // Test with SHA512
        assert!(verify_checksum(
            temp_file.path(),
            "8ac28e9332997358babeb15653920d584d3e1ba14977c137dae6cad5e67ca41accd58ef4fcdcbeff396ff1c720b811445b51a5656f33aada0ed1d7317081caaa",
            ChecksumType::Sha512
        )
        .is_ok());

        // Test with MD5
        assert!(
            verify_checksum(
                temp_file.path(),
                "8bfa8e0684108f419933a5995264d150",
                ChecksumType::Md5
            )
            .is_ok()
        );
    }

    #[test]
    fn test_verify_https_security() {
        // Valid HTTPS URLs
        assert!(verify_https_security("https://example.com/file.tar.gz").is_ok());
        assert!(verify_https_security("https://api.foojay.io/v3/").is_ok());

        // Invalid URLs
        assert!(verify_https_security("http://example.com/file.tar.gz").is_err());
        assert!(verify_https_security("ftp://example.com/file.tar.gz").is_err());
        assert!(verify_https_security("https://localhost/file.tar.gz").is_err());
        assert!(verify_https_security("https://127.0.0.1/file.tar.gz").is_err());
        assert!(verify_https_security("https://example.com/../etc/passwd").is_err());
    }

    #[test]
    fn test_is_trusted_domain() {
        // Trusted domains
        assert!(is_trusted_domain("https://api.foojay.io/v3/packages"));
        assert!(is_trusted_domain("https://download.oracle.com/java/21/"));
        assert!(is_trusted_domain("https://github.com/adoptium/releases"));
        assert!(is_trusted_domain("https://corretto.aws/downloads/"));
        assert!(is_trusted_domain("https://cdn.azul.com/zulu/bin/"));

        // Untrusted domains
        assert!(!is_trusted_domain("https://example.com/java"));
        assert!(!is_trusted_domain("https://malicious.site/jdk"));
        assert!(!is_trusted_domain("http://api.foojay.io/v3/"));
    }

    #[test]
    fn test_sanitize_path() {
        // Valid paths
        assert!(sanitize_path(Path::new("jdk-21")).is_ok());
        assert!(sanitize_path(Path::new("vendors/temurin")).is_ok());

        // Invalid paths
        assert!(sanitize_path(Path::new("../etc/passwd")).is_err());
        assert!(sanitize_path(Path::new("~/sensitive")).is_err());
        assert!(sanitize_path(Path::new("vendors/../../../etc")).is_err());

        // Platform-specific absolute paths
        #[cfg(unix)]
        {
            assert!(sanitize_path(Path::new("/home/user/.kopi/jdks")).is_ok());
            assert!(sanitize_path(Path::new("/etc/passwd")).is_err());
        }

        #[cfg(windows)]
        {
            assert!(sanitize_path(Path::new("C:\\Users\\user\\.kopi\\jdks")).is_ok());
            assert!(sanitize_path(Path::new("C:\\Windows\\System32")).is_err());
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_verify_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();

        // Set safe permissions (644)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        assert!(verify_file_permissions(temp_file.path()).is_ok());

        // Set unsafe permissions (world-writable)
        perms.set_mode(0o666);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert!(verify_file_permissions(temp_file.path()).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_verify_file_permissions_windows() {
        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files are writable
        assert!(verify_file_permissions(temp_file.path()).is_ok());

        // Set file as read-only
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_readonly(true);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        // Should still be OK (read-only is more secure)
        assert!(verify_file_permissions(temp_file.path()).is_ok());

        // Test with a directory (should fail)
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(verify_file_permissions(temp_dir.path()).is_err());
    }
}
