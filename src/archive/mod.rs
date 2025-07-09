use crate::error::{KopiError, Result};
use crate::platform::file_ops;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive as TarArchive;
use zip::ZipArchive;

pub enum ArchiveType {
    TarGz,
    Zip,
}

pub struct ArchiveInfo {
    pub archive_type: ArchiveType,
    pub file_count: usize,
    pub uncompressed_size: u64,
}

/// Extract a JDK archive to the specified destination
pub fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    // Ensure destination directory exists
    fs::create_dir_all(destination)?;

    // Detect archive type based on file extension
    let archive_type = detect_archive_type(archive_path)?;

    // Verify archive integrity before extraction
    verify_integrity(archive_path, &archive_type)?;

    match archive_type {
        ArchiveType::TarGz => extract_tar_gz(archive_path, destination),
        ArchiveType::Zip => extract_zip(archive_path, destination),
    }
}

fn detect_archive_type(path: &Path) -> Result<ArchiveType> {
    // First try by extension
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
        return Ok(ArchiveType::TarGz);
    }
    if path_str.ends_with(".zip") {
        return Ok(ArchiveType::Zip);
    }

    // If extension doesn't match, try to detect by file content
    detect_by_content(path)
}

fn detect_by_content(path: &Path) -> Result<ArchiveType> {
    let mut file = File::open(path)?;
    let mut magic_bytes = [0u8; 4];
    file.read_exact(&mut magic_bytes).map_err(|_| {
        KopiError::ValidationError(format!(
            "Cannot read file to determine archive type: {path:?}"
        ))
    })?;

    // Check for gzip magic bytes (1f 8b)
    if magic_bytes[0] == 0x1f && magic_bytes[1] == 0x8b {
        // It's a gzip file, assume it's tar.gz
        return Ok(ArchiveType::TarGz);
    }

    // Check for ZIP magic bytes (50 4b 03 04 or 50 4b 05 06 or 50 4b 07 08)
    if magic_bytes[0] == 0x50
        && magic_bytes[1] == 0x4b
        && (magic_bytes[2] == 0x03 || magic_bytes[2] == 0x05 || magic_bytes[2] == 0x07)
    {
        return Ok(ArchiveType::Zip);
    }

    Err(KopiError::ValidationError(format!(
        "Unsupported archive format. File does not appear to be tar.gz or zip: {path:?}"
    )))
}

fn verify_integrity(archive_path: &Path, archive_type: &ArchiveType) -> Result<()> {
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

fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = TarArchive::new(gz);

    // Configure archive extraction
    archive.set_preserve_permissions(true);
    archive.set_preserve_mtime(true);
    archive.set_overwrite(true);

    // Track extracted files for verification
    let mut extracted_count = 0;
    let entries = archive.entries()?;

    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;

        // Security check: ensure paths don't escape destination
        validate_entry_path(&path)?;

        // Extract entry
        let dest_path = destination.join(&path);

        // Create parent directories if needed (same as zip extraction)
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        entry.unpack(&dest_path)?;
        extracted_count += 1;

        // Log extraction progress for large archives
        if extracted_count % 100 == 0 {
            log::debug!("Extracted {extracted_count} files...");
        }
    }

    log::info!("Extracted {extracted_count} files from tar.gz archive");
    Ok(())
}

fn extract_zip(archive_path: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    let total_files = archive.len();

    for i in 0..total_files {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => {
                // Security check: ensure paths don't escape destination
                validate_entry_path(&path)?;
                destination.join(path)
            }
            None => {
                log::warn!("Skipping file with invalid name at index {i}");
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

        // Set permissions from archive metadata
        if let Some(mode) = file.unix_mode() {
            file_ops::set_permissions_from_mode(&outpath, mode)?;
        }

        // Log extraction progress for large archives
        if (i + 1) % 100 == 0 {
            log::debug!("Extracted {}/{} files...", i + 1, total_files);
        }
    }

    log::info!("Extracted {total_files} files from zip archive");
    Ok(())
}

fn validate_entry_path(entry_path: &Path) -> Result<()> {
    // Ensure the entry path doesn't contain any parent directory references
    for component in entry_path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(KopiError::SecurityError(format!(
                    "Archive contains path traversal: {entry_path:?}"
                )));
            }
            std::path::Component::RootDir => {
                return Err(KopiError::SecurityError(format!(
                    "Archive contains absolute path: {entry_path:?}"
                )));
            }
            _ => {}
        }
    }

    // Additional check: normalize the path and verify it doesn't escape
    let normalized = normalize_path(entry_path);
    if normalized.starts_with("..") || normalized.starts_with("/") || normalized.starts_with("\\") {
        return Err(KopiError::SecurityError(format!(
            "Archive entry would extract outside destination: {entry_path:?}"
        )));
    }

    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Normal(c) => normalized.push(c),
            std::path::Component::CurDir => {} // Skip current directory markers
            std::path::Component::ParentDir => {
                // Remove last component if it exists, otherwise it's trying to escape
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            std::path::Component::RootDir => normalized.push("/"),
            std::path::Component::Prefix(_) => {} // Windows drive letters - ignore
        }
    }

    normalized
}

pub fn get_archive_info(archive_path: &Path) -> Result<ArchiveInfo> {
    let archive_type = detect_archive_type(archive_path)?;
    let file_count = count_files(archive_path, &archive_type)?;
    let uncompressed_size = calculate_uncompressed_size(archive_path, &archive_type)?;

    Ok(ArchiveInfo {
        archive_type,
        file_count,
        uncompressed_size,
    })
}

fn count_files(archive_path: &Path, archive_type: &ArchiveType) -> Result<usize> {
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

fn calculate_uncompressed_size(archive_path: &Path, archive_type: &ArchiveType) -> Result<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    struct TestArchive {
        path: PathBuf,
        _temp_dir: tempfile::TempDir,
    }

    fn create_test_tar_gz() -> Result<TestArchive> {
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

        Ok(TestArchive {
            path: tar_path,
            _temp_dir: temp_dir,
        })
    }

    fn create_test_zip() -> Result<TestArchive> {
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

        Ok(TestArchive {
            path: zip_path,
            _temp_dir: temp_dir,
        })
    }

    #[test]
    fn test_detect_archive_type() {
        assert!(matches!(
            detect_archive_type(Path::new("file.tar.gz")).unwrap(),
            ArchiveType::TarGz
        ));
        assert!(matches!(
            detect_archive_type(Path::new("file.tgz")).unwrap(),
            ArchiveType::TarGz
        ));
        assert!(matches!(
            detect_archive_type(Path::new("file.zip")).unwrap(),
            ArchiveType::Zip
        ));

        // Test actual files with content detection
        let tar_archive = create_test_tar_gz().unwrap();
        assert!(matches!(
            detect_archive_type(&tar_archive.path).unwrap(),
            ArchiveType::TarGz
        ));

        let zip_archive = create_test_zip().unwrap();
        assert!(matches!(
            detect_archive_type(&zip_archive.path).unwrap(),
            ArchiveType::Zip
        ));
    }

    #[test]
    fn test_extract_tar_gz() -> Result<()> {
        let archive = create_test_tar_gz()?;
        let dest_dir = tempdir()?;

        extract_archive(&archive.path, dest_dir.path())?;

        let extracted_file = dest_dir.path().join("test.txt");
        assert!(extracted_file.exists());

        let content = fs::read_to_string(extracted_file)?;
        assert_eq!(content, "Hello World");

        Ok(())
    }

    #[test]
    fn test_extract_zip() -> Result<()> {
        let archive = create_test_zip()?;
        let dest_dir = tempdir()?;

        extract_archive(&archive.path, dest_dir.path())?;

        let extracted_file = dest_dir.path().join("test.txt");
        assert!(extracted_file.exists());

        let content = fs::read_to_string(extracted_file)?;
        assert_eq!(content, "Hello World");

        Ok(())
    }

    #[test]
    fn test_validate_entry_path() {
        // Valid paths
        assert!(validate_entry_path(Path::new("jdk/bin/java")).is_ok());
        assert!(validate_entry_path(Path::new("lib/modules")).is_ok());

        // Invalid paths
        assert!(validate_entry_path(Path::new("../etc/passwd")).is_err());
        assert!(validate_entry_path(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn test_archive_info() -> Result<()> {
        // Test tar.gz
        let tar_archive = create_test_tar_gz()?;
        let tar_info = get_archive_info(&tar_archive.path)?;
        assert!(matches!(tar_info.archive_type, ArchiveType::TarGz));
        assert_eq!(tar_info.file_count, 1);
        assert_eq!(tar_info.uncompressed_size, 11);

        // Test zip
        let zip_archive = create_test_zip()?;
        let zip_info = get_archive_info(&zip_archive.path)?;
        assert!(matches!(zip_info.archive_type, ArchiveType::Zip));
        assert_eq!(zip_info.file_count, 1);
        assert_eq!(zip_info.uncompressed_size, 11);

        Ok(())
    }

    #[test]
    fn test_tar_gz_with_nested_directories() -> Result<()> {
        // This test verifies that tar.gz files with nested directories
        // are extracted correctly (with the fix that creates parent directories)

        // Create a tar.gz with files in nested directories
        let temp_dir = tempdir()?;
        let tar_path = temp_dir.path().join("nested.tar.gz");

        let file = File::create(&tar_path)?;
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);

        // Add a file in root directory
        let mut header1 = tar::Header::new_gnu();
        header1.set_path("root.txt")?;
        header1.set_size(4);
        header1.set_mode(0o644);
        header1.set_cksum();
        builder.append(&header1, &b"root"[..])?;

        // Add a file in a subdirectory (like graalvm-jdk-21.0.7+8.1/license-information-user-manual.zip)
        let mut header2 = tar::Header::new_gnu();
        header2.set_path("graalvm-jdk-21.0.7+8.1/license-information-user-manual.zip")?;
        header2.set_size(6);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder.append(&header2, &b"nested"[..])?;

        // Add a file in a deeper nested directory
        let mut header3 = tar::Header::new_gnu();
        header3.set_path("jdk/bin/java")?;
        header3.set_size(4);
        header3.set_mode(0o755);
        header3.set_cksum();
        builder.append(&header3, &b"java"[..])?;

        builder.finish()?;
        drop(builder);

        // Extract the archive
        let dest_dir = tempdir()?;
        extract_archive(&tar_path, dest_dir.path())?;

        // Verify all files were extracted correctly
        let root_file = dest_dir.path().join("root.txt");
        assert!(root_file.exists());
        assert_eq!(fs::read_to_string(&root_file)?, "root");

        let license_file = dest_dir
            .path()
            .join("graalvm-jdk-21.0.7+8.1/license-information-user-manual.zip");
        assert!(license_file.exists());
        assert_eq!(fs::read_to_string(&license_file)?, "nested");

        let java_file = dest_dir.path().join("jdk/bin/java");
        assert!(java_file.exists());
        assert_eq!(fs::read_to_string(&java_file)?, "java");

        // Verify directory structure
        assert!(dest_dir.path().join("graalvm-jdk-21.0.7+8.1").is_dir());
        assert!(dest_dir.path().join("jdk").is_dir());
        assert!(dest_dir.path().join("jdk/bin").is_dir());

        Ok(())
    }
}
