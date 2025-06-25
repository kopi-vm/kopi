use crate::error::{KopiError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const CHUNK_SIZE: usize = 8192;

pub struct SecurityManager {
    // Future: Could store trusted certificates or signatures
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn verify_checksum(&self, file_path: &Path, expected_checksum: &str) -> Result<()> {
        let actual = self.calculate_sha256(file_path)?;

        if actual != expected_checksum {
            return Err(KopiError::ValidationError(format!(
                "Checksum verification failed for {:?}. Expected: {}, Actual: {}",
                file_path, expected_checksum, actual
            )));
        }

        log::debug!("Checksum verified successfully for {:?}", file_path);
        Ok(())
    }

    pub fn calculate_sha256(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; CHUNK_SIZE];

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buffer[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn verify_https_security(&self, url: &str) -> Result<()> {
        if !url.starts_with("https://") {
            return Err(KopiError::SecurityError(format!(
                "Insecure URL: {}. Only HTTPS URLs are allowed for JDK downloads",
                url
            )));
        }

        // Additional URL validation
        if url.contains("..") || url.contains("://localhost") || url.contains("://127.0.0.1") {
            return Err(KopiError::SecurityError(format!(
                "Suspicious URL detected: {}",
                url
            )));
        }

        Ok(())
    }

    pub fn is_trusted_domain(&self, url: &str) -> bool {
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

    pub fn audit_log(&self, action: &str, details: &str) {
        // In production, this would write to a secure audit log
        log::info!("SECURITY AUDIT: {} - {}", action, details);
    }

    pub fn verify_file_permissions(&self, path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let metadata = std::fs::metadata(path)?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Check if file has dangerous permissions (world-writable)
            if mode & 0o002 != 0 {
                return Err(KopiError::SecurityError(format!(
                    "File {:?} has world-writable permissions",
                    path
                )));
            }
        }

        #[cfg(windows)]
        {
            use std::fs;

            let metadata = fs::metadata(path)?;

            // On Windows, check if the file is read-only and exists
            if !metadata.is_file() {
                return Err(KopiError::SecurityError(format!(
                    "Path {:?} is not a regular file",
                    path
                )));
            }

            // Check if the file has the read-only attribute
            // In Windows, files without read-only attribute are writable by the owner
            // For JDK files, we generally want them to be read-only after installation
            if metadata.permissions().readonly() {
                log::debug!("File {:?} is read-only (secure)", path);
            } else {
                log::warn!(
                    "File {:?} is writable - consider setting read-only for security",
                    path
                );
            }

            // Additional Windows-specific security checks could include:
            // - Checking ACLs (Access Control Lists) using Windows API
            // - Verifying file ownership
            // - Checking for alternate data streams
            // For now, we do basic checks that work with std::fs
        }

        Ok(())
    }

    pub fn sanitize_path(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Check for path traversal attempts
        if path_str.contains("..") || path_str.contains("~") {
            return Err(KopiError::SecurityError(format!(
                "Potential path traversal detected in: {:?}",
                path
            )));
        }

        // Check for absolute paths that might escape the kopi directory
        if path.is_absolute() {
            let path_str = path.to_string_lossy();
            if !path_str.contains(".kopi") {
                return Err(KopiError::SecurityError(format!(
                    "Path {:?} is outside of kopi directory",
                    path
                )));
            }
        }

        Ok(())
    }

    /// Set file permissions to read-only for security
    /// This is especially important for JDK files after installation
    pub fn secure_file_permissions(&self, path: &Path) -> Result<()> {
        let metadata = std::fs::metadata(path)?;
        let mut permissions = metadata.permissions();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Set to 644 (owner: read/write, group: read, others: read)
            // This prevents accidental modification while allowing execution
            permissions.set_mode(0o644);
        }

        #[cfg(windows)]
        {
            // Set the read-only attribute on Windows
            permissions.set_readonly(true);
        }

        std::fs::set_permissions(path, permissions)?;

        self.audit_log(
            "SECURE_PERMISSIONS",
            &format!("Set secure permissions on {:?}", path),
        );

        Ok(())
    }

    /// Recursively secure all files in a directory
    pub fn secure_directory_permissions(&self, dir: &Path) -> Result<()> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                self.secure_file_permissions(path)?;
            }
        }

        Ok(())
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify the checksum of a file
pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
    let manager = SecurityManager::new();
    manager.verify_checksum(file_path, expected_checksum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_sha256() {
        let security = SecurityManager::new();

        // Create a temporary file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, World!").unwrap();
        temp_file.flush().unwrap();

        let checksum = security.calculate_sha256(temp_file.path()).unwrap();

        // Known SHA256 of "Hello, World!"
        assert_eq!(
            checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_verify_checksum_success() {
        let security = SecurityManager::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content").unwrap();
        temp_file.flush().unwrap();

        let expected = "9d9595c5d94fb65b824f56e9999527dba9542481580d69feb89056aabaa0aa87";

        assert!(security.verify_checksum(temp_file.path(), expected).is_ok());
    }

    #[test]
    fn test_verify_checksum_failure() {
        let security = SecurityManager::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content").unwrap();
        temp_file.flush().unwrap();

        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";

        assert!(
            security
                .verify_checksum(temp_file.path(), wrong_checksum)
                .is_err()
        );
    }

    #[test]
    fn test_verify_https_security() {
        let security = SecurityManager::new();

        // Valid HTTPS URLs
        assert!(
            security
                .verify_https_security("https://example.com/file.tar.gz")
                .is_ok()
        );
        assert!(
            security
                .verify_https_security("https://api.foojay.io/v3/")
                .is_ok()
        );

        // Invalid URLs
        assert!(
            security
                .verify_https_security("http://example.com/file.tar.gz")
                .is_err()
        );
        assert!(
            security
                .verify_https_security("ftp://example.com/file.tar.gz")
                .is_err()
        );
        assert!(
            security
                .verify_https_security("https://localhost/file.tar.gz")
                .is_err()
        );
        assert!(
            security
                .verify_https_security("https://127.0.0.1/file.tar.gz")
                .is_err()
        );
        assert!(
            security
                .verify_https_security("https://example.com/../etc/passwd")
                .is_err()
        );
    }

    #[test]
    fn test_is_trusted_domain() {
        let security = SecurityManager::new();

        // Trusted domains
        assert!(security.is_trusted_domain("https://api.foojay.io/v3/packages"));
        assert!(security.is_trusted_domain("https://download.oracle.com/java/21/"));
        assert!(security.is_trusted_domain("https://github.com/adoptium/releases"));
        assert!(security.is_trusted_domain("https://corretto.aws/downloads/"));
        assert!(security.is_trusted_domain("https://cdn.azul.com/zulu/bin/"));

        // Untrusted domains
        assert!(!security.is_trusted_domain("https://example.com/java"));
        assert!(!security.is_trusted_domain("https://malicious.site/jdk"));
        assert!(!security.is_trusted_domain("http://api.foojay.io/v3/"));
    }

    #[test]
    fn test_sanitize_path() {
        let security = SecurityManager::new();

        // Valid paths
        assert!(security.sanitize_path(Path::new("jdk-21")).is_ok());
        assert!(security.sanitize_path(Path::new("vendors/temurin")).is_ok());

        // Invalid paths
        assert!(security.sanitize_path(Path::new("../etc/passwd")).is_err());
        assert!(security.sanitize_path(Path::new("~/sensitive")).is_err());
        assert!(
            security
                .sanitize_path(Path::new("vendors/../../../etc"))
                .is_err()
        );

        // Absolute paths
        assert!(
            security
                .sanitize_path(Path::new("/home/user/.kopi/jdks"))
                .is_ok()
        );
        assert!(security.sanitize_path(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_verify_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let security = SecurityManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        // Set safe permissions (644)
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o644);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        assert!(security.verify_file_permissions(temp_file.path()).is_ok());

        // Set unsafe permissions (world-writable)
        perms.set_mode(0o666);
        temp_file.as_file().set_permissions(perms).unwrap();

        assert!(security.verify_file_permissions(temp_file.path()).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_verify_file_permissions_windows() {
        use std::fs;

        let security = SecurityManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        // By default, temp files are writable
        assert!(security.verify_file_permissions(temp_file.path()).is_ok());

        // Set file as read-only
        let mut perms = temp_file.as_file().metadata().unwrap().permissions();
        perms.set_readonly(true);
        temp_file.as_file().set_permissions(perms.clone()).unwrap();

        // Should still be OK (read-only is more secure)
        assert!(security.verify_file_permissions(temp_file.path()).is_ok());

        // Test with a directory (should fail)
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(security.verify_file_permissions(temp_dir.path()).is_err());
    }
}
