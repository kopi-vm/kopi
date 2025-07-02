use crate::error::{KopiError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const CHECKSUM_CHUNK_SIZE: usize = 8192;

pub fn verify_checksum(file_path: &Path, expected: &str) -> Result<()> {
    let calculated = calculate_sha256(file_path)?;

    if calculated != expected {
        return Err(KopiError::ValidationError(format!(
            "Checksum mismatch: expected {expected}, got {calculated}"
        )));
    }

    Ok(())
}

pub fn calculate_sha256(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHECKSUM_CHUNK_SIZE];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_sha256() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Hello, World!")?;

        let checksum = calculate_sha256(temp_file.path())?;

        // Expected SHA256 of "Hello, World!"
        assert_eq!(
            checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );

        Ok(())
    }

    #[test]
    fn test_verify_checksum_success() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Test content")?;

        let expected = "9d9595c5d94fb65b824f56e9999527dba9542481580d69feb89056aabaa0aa87";

        assert!(verify_checksum(temp_file.path(), expected).is_ok());

        Ok(())
    }

    #[test]
    fn test_verify_checksum_mismatch() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Test content")?;

        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = verify_checksum(temp_file.path(), wrong_checksum);
        assert!(result.is_err());

        match result {
            Err(KopiError::ValidationError(msg)) => {
                assert!(msg.contains("Checksum mismatch"));
            }
            _ => panic!("Expected ValidationError"),
        }

        Ok(())
    }
}
