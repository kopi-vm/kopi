use crate::error::{KopiError, Result};
use std::fs::{self, File};
use std::path::Path;
use tar::Archive as TarArchive;
use zip::ZipArchive;

pub enum ArchiveType {
    TarGz,
    Zip,
}

pub struct ArchiveHandler {
    preserve_permissions: bool,
}

impl ArchiveHandler {
    pub fn new() -> Self {
        Self {
            preserve_permissions: true,
        }
    }

    pub fn extract(&self, archive_path: &Path, destination: &Path) -> Result<()> {
        // Ensure destination directory exists
        fs::create_dir_all(destination)?;

        // Detect archive type based on file extension
        let archive_type = self.detect_archive_type(archive_path)?;

        // Verify archive integrity before extraction
        self.verify_integrity(archive_path, &archive_type)?;

        match archive_type {
            ArchiveType::TarGz => self.extract_tar_gz(archive_path, destination),
            ArchiveType::Zip => self.extract_zip(archive_path, destination),
        }
    }

    fn detect_archive_type(&self, path: &Path) -> Result<ArchiveType> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match extension.as_deref() {
            Some("gz") => {
                // Check if it's a .tar.gz file
                if path.to_string_lossy().ends_with(".tar.gz") {
                    Ok(ArchiveType::TarGz)
                } else {
                    Err(KopiError::ValidationError(
                        "Unsupported archive format: .gz without .tar".to_string(),
                    ))
                }
            }
            Some("zip") => Ok(ArchiveType::Zip),
            _ => Err(KopiError::ValidationError(format!(
                "Unsupported archive format: {:?}",
                path
            ))),
        }
    }

    fn verify_integrity(&self, archive_path: &Path, archive_type: &ArchiveType) -> Result<()> {
        match archive_type {
            ArchiveType::TarGz => {
                // Basic verification: try to read the archive header
                let file = File::open(archive_path)?;
                let gz = flate2::read::GzDecoder::new(file);
                let mut archive = TarArchive::new(gz);

                // Try to list entries to verify the archive is valid
                let mut entries = archive.entries()?;
                if let Some(entry) = entries.next() {
                    let _ = entry?;
                }
                Ok(())
            }
            ArchiveType::Zip => {
                // Basic verification: try to open the archive
                let file = File::open(archive_path)?;
                let archive = ZipArchive::new(file)?;

                // Check if archive is not empty
                if archive.is_empty() {
                    return Err(KopiError::ValidationError(
                        "Zip archive is empty".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    fn extract_tar_gz(&self, archive_path: &Path, destination: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = TarArchive::new(gz);

        // Configure archive extraction
        archive.set_preserve_permissions(self.preserve_permissions);
        archive.set_preserve_mtime(true);
        archive.set_overwrite(true);

        // Track extracted files for verification
        let mut extracted_count = 0;
        let entries = archive.entries()?;

        for entry in entries {
            let mut entry = entry?;
            let path = entry.path()?;

            // Security check: ensure paths don't escape destination
            self.validate_entry_path(&path, destination)?;

            // Extract entry
            let dest_path = destination.join(&path);
            entry.unpack(&dest_path)?;
            extracted_count += 1;

            // Log extraction progress for large archives
            if extracted_count % 100 == 0 {
                log::debug!("Extracted {} files...", extracted_count);
            }
        }

        log::info!("Extracted {} files from tar.gz archive", extracted_count);
        Ok(())
    }

    fn extract_zip(&self, archive_path: &Path, destination: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        let total_files = archive.len();

        for i in 0..total_files {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => {
                    // Security check: ensure paths don't escape destination
                    self.validate_entry_path(&path, destination)?;
                    destination.join(path)
                }
                None => {
                    log::warn!("Skipping file with invalid name at index {}", i);
                    continue;
                }
            };

            // Create parent directories if needed
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }

            // Extract file
            if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }

            // Log extraction progress for large archives
            if (i + 1) % 100 == 0 {
                log::debug!("Extracted {}/{} files...", i + 1, total_files);
            }
        }

        log::info!("Extracted {} files from zip archive", total_files);
        Ok(())
    }

    fn validate_entry_path(&self, entry_path: &Path, destination: &Path) -> Result<()> {
        // Ensure the entry path doesn't contain any parent directory references
        for component in entry_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(KopiError::SecurityError(format!(
                        "Archive contains path traversal: {:?}",
                        entry_path
                    )));
                }
                std::path::Component::RootDir => {
                    return Err(KopiError::SecurityError(format!(
                        "Archive contains absolute path: {:?}",
                        entry_path
                    )));
                }
                _ => {}
            }
        }

        // Verify the extracted path would be within destination
        let full_path = destination.join(entry_path);
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.to_path_buf());
        let canonical_full = full_path.canonicalize().unwrap_or(full_path.clone());

        if !canonical_full.starts_with(&canonical_dest) {
            return Err(KopiError::SecurityError(format!(
                "Archive entry would extract outside destination: {:?}",
                entry_path
            )));
        }

        Ok(())
    }

    pub fn get_archive_info(&self, archive_path: &Path) -> Result<ArchiveInfo> {
        let archive_type = self.detect_archive_type(archive_path)?;
        let file_count = self.count_files(archive_path, &archive_type)?;
        let uncompressed_size = self.calculate_uncompressed_size(archive_path, &archive_type)?;

        Ok(ArchiveInfo {
            archive_type,
            file_count,
            uncompressed_size,
        })
    }

    fn count_files(&self, archive_path: &Path, archive_type: &ArchiveType) -> Result<usize> {
        match archive_type {
            ArchiveType::TarGz => {
                let file = File::open(archive_path)?;
                let gz = flate2::read::GzDecoder::new(file);
                let mut archive = TarArchive::new(gz);
                Ok(archive.entries()?.count())
            }
            ArchiveType::Zip => {
                let file = File::open(archive_path)?;
                let archive = ZipArchive::new(file)?;
                Ok(archive.len())
            }
        }
    }

    fn calculate_uncompressed_size(
        &self,
        archive_path: &Path,
        archive_type: &ArchiveType,
    ) -> Result<u64> {
        match archive_type {
            ArchiveType::TarGz => {
                let file = File::open(archive_path)?;
                let gz = flate2::read::GzDecoder::new(file);
                let mut archive = TarArchive::new(gz);
                let mut total_size = 0u64;

                for entry in archive.entries()? {
                    let entry = entry?;
                    total_size += entry.header().size()?;
                }

                Ok(total_size)
            }
            ArchiveType::Zip => {
                let file = File::open(archive_path)?;
                let mut archive = ZipArchive::new(file)?;
                let mut total_size = 0u64;

                for i in 0..archive.len() {
                    let file = archive.by_index(i)?;
                    total_size += file.size();
                }

                Ok(total_size)
            }
        }
    }
}

impl Default for ArchiveHandler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArchiveInfo {
    pub archive_type: ArchiveType,
    pub file_count: usize,
    pub uncompressed_size: u64,
}

/// Extract a JDK archive to the specified destination
pub fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let handler = ArchiveHandler::new();
    handler.extract(archive_path, destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_tar_gz() -> Result<PathBuf> {
        let temp_dir = tempdir()?;
        let tar_path = temp_dir.path().join("test.tar.gz");

        let file = File::create(&tar_path)?;
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);

        // Add a test file
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt")?;
        header.set_size(11);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, &b"Hello World"[..])?;

        builder.finish()?;

        // Keep the temp directory alive by leaking it
        std::mem::forget(temp_dir);
        Ok(tar_path)
    }

    fn create_test_zip() -> Result<PathBuf> {
        let temp_dir = tempdir()?;
        let zip_path = temp_dir.path().join("test.zip");

        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);

        zip.start_file("test.txt", options)?;
        zip.write_all(b"Hello World")?;
        zip.finish()?;

        // Keep the temp directory alive by leaking it
        std::mem::forget(temp_dir);
        Ok(zip_path)
    }

    #[test]
    fn test_detect_archive_type() {
        let handler = ArchiveHandler::new();

        assert!(matches!(
            handler
                .detect_archive_type(Path::new("file.tar.gz"))
                .unwrap(),
            ArchiveType::TarGz
        ));
        assert!(matches!(
            handler.detect_archive_type(Path::new("file.zip")).unwrap(),
            ArchiveType::Zip
        ));
        assert!(handler.detect_archive_type(Path::new("file.txt")).is_err());
        assert!(handler.detect_archive_type(Path::new("file.gz")).is_err());
    }

    #[test]
    fn test_extract_tar_gz() -> Result<()> {
        let handler = ArchiveHandler::new();
        let archive_path = create_test_tar_gz()?;
        let dest_dir = tempdir()?;

        handler.extract(&archive_path, dest_dir.path())?;

        let extracted_file = dest_dir.path().join("test.txt");
        assert!(extracted_file.exists());

        let content = fs::read_to_string(extracted_file)?;
        assert_eq!(content, "Hello World");

        Ok(())
    }

    #[test]
    fn test_extract_zip() -> Result<()> {
        let handler = ArchiveHandler::new();
        let archive_path = create_test_zip()?;
        let dest_dir = tempdir()?;

        handler.extract(&archive_path, dest_dir.path())?;

        let extracted_file = dest_dir.path().join("test.txt");
        assert!(extracted_file.exists());

        let content = fs::read_to_string(extracted_file)?;
        assert_eq!(content, "Hello World");

        Ok(())
    }

    #[test]
    fn test_validate_entry_path() {
        let handler = ArchiveHandler::new();
        let destination = Path::new("/tmp/kopi");

        // Valid paths
        assert!(
            handler
                .validate_entry_path(Path::new("jdk/bin/java"), destination)
                .is_ok()
        );
        assert!(
            handler
                .validate_entry_path(Path::new("lib/modules"), destination)
                .is_ok()
        );

        // Invalid paths
        assert!(
            handler
                .validate_entry_path(Path::new("../etc/passwd"), destination)
                .is_err()
        );
        assert!(
            handler
                .validate_entry_path(Path::new("/etc/passwd"), destination)
                .is_err()
        );
    }

    #[test]
    fn test_archive_info() -> Result<()> {
        let handler = ArchiveHandler::new();

        // Test tar.gz
        let tar_path = create_test_tar_gz()?;
        let tar_info = handler.get_archive_info(&tar_path)?;
        assert!(matches!(tar_info.archive_type, ArchiveType::TarGz));
        assert_eq!(tar_info.file_count, 1);
        assert_eq!(tar_info.uncompressed_size, 11);

        // Test zip
        let zip_path = create_test_zip()?;
        let zip_info = handler.get_archive_info(&zip_path)?;
        assert!(matches!(zip_info.archive_type, ArchiveType::Zip));
        assert_eq!(zip_info.file_count, 1);
        assert_eq!(zip_info.uncompressed_size, 11);

        Ok(())
    }
}
