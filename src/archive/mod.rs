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

/// Represents the type of JDK directory structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JdkStructureType {
    /// Direct structure: bin/, lib/, conf/ at root
    Direct,
    /// Bundle structure: Contents/Home/bin/, Contents/Home/lib/, etc.
    Bundle,
    /// Hybrid structure: symlinks at root pointing to bundle structure
    Hybrid,
}

/// Result of JDK structure detection
#[derive(Debug, Clone)]
pub struct JdkStructureInfo {
    /// The actual root directory of the JDK where bin/java exists
    pub jdk_root: PathBuf,
    /// The type of structure detected
    pub structure_type: JdkStructureType,
    /// Suffix to append to installation directory to get to JDK root (e.g., "Contents/Home")
    pub java_home_suffix: String,
}

/// Detect the JDK root directory and its structure type from an extracted archive
///
/// This function analyzes the directory structure of an extracted JDK archive to determine
/// where the actual JDK files are located. On macOS, JDKs can have different structures:
/// - Direct: bin/, lib/, conf/ at the root
/// - Bundle: macOS application bundle with Contents/Home/
/// - Hybrid: symlinks at root pointing to bundle structure (e.g., Azul Zulu)
///
/// # Arguments
/// * `extracted_dir` - The directory where the archive was extracted
///
/// # Returns
/// * `Ok(JdkStructureInfo)` - Information about the detected JDK structure
/// * `Err(KopiError)` - If no valid JDK structure is found
pub fn detect_jdk_root(extracted_dir: &Path) -> Result<JdkStructureInfo> {
    log::debug!("Detecting JDK structure in: {}", extracted_dir.display());

    // Check for direct structure or hybrid (symlinks at root)
    if extracted_dir.join("bin").exists() {
        log::debug!("Found bin/ at root - checking if valid JDK");
        if validate_jdk_root(extracted_dir)? {
            // Check if this is a hybrid structure (symlinks pointing to bundle)
            let structure_type = if is_hybrid_structure(extracted_dir) {
                log::info!("Detected hybrid JDK structure (symlinks to bundle)");
                JdkStructureType::Hybrid
            } else {
                log::info!("Detected direct JDK structure");
                JdkStructureType::Direct
            };

            return Ok(JdkStructureInfo {
                jdk_root: extracted_dir.to_path_buf(),
                structure_type,
                java_home_suffix: String::new(),
            });
        }
    }

    // Check for macOS bundle structure at root
    #[cfg(target_os = "macos")]
    {
        let bundle_home = extracted_dir.join("Contents").join("Home");
        if bundle_home.exists() {
            log::debug!("Found Contents/Home/ - checking if valid JDK");
            if validate_jdk_root(&bundle_home)? {
                log::info!("Detected macOS bundle JDK structure");
                return Ok(JdkStructureInfo {
                    jdk_root: bundle_home,
                    structure_type: JdkStructureType::Bundle,
                    java_home_suffix: "Contents/Home".to_string(),
                });
            }
        }

        // Check for nested bundle structure (e.g., jdk-x.y.z.jdk/Contents/Home/)
        // This is common when the archive contains an extra directory level
        if let Ok(entries) = fs::read_dir(extracted_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    let nested_bundle = entry.path().join("Contents").join("Home");
                    if nested_bundle.exists() {
                        log::debug!(
                            "Found nested Contents/Home/ at {} - checking if valid JDK",
                            entry.path().display()
                        );
                        if validate_jdk_root(&nested_bundle)? {
                            log::info!(
                                "Detected nested macOS bundle JDK structure at {}",
                                entry.path().display()
                            );
                            // For nested bundles, we need to move the entire bundle directory
                            // So return the parent of Contents/Home which contains the full bundle
                            if let Some(bundle_dir) =
                                nested_bundle.parent().and_then(|p| p.parent())
                            {
                                return Ok(JdkStructureInfo {
                                    jdk_root: bundle_dir.to_path_buf(),
                                    structure_type: JdkStructureType::Bundle,
                                    java_home_suffix: "Contents/Home".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // For non-macOS platforms or if no bundle structure found, look for direct structure in subdirectories
    if let Ok(entries) = fs::read_dir(extracted_dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                let path = entry.path();
                if path.join("bin").exists() {
                    log::debug!(
                        "Found bin/ in subdirectory {} - checking if valid JDK",
                        path.display()
                    );
                    if validate_jdk_root(&path)? {
                        log::info!("Detected JDK in subdirectory: {}", path.display());
                        return Ok(JdkStructureInfo {
                            jdk_root: path,
                            structure_type: JdkStructureType::Direct,
                            java_home_suffix: String::new(),
                        });
                    }
                }
            }
        }
    }

    // No valid JDK structure found
    Err(KopiError::ValidationError(format!(
        "No valid JDK structure found in {}. Expected to find bin/java or Contents/Home/bin/java",
        extracted_dir.display()
    )))
}

/// Validate that a directory contains a valid JDK by checking for the java binary
fn validate_jdk_root(path: &Path) -> Result<bool> {
    let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
    let java_path = path.join("bin").join(java_binary);

    Ok(java_path.exists())
}

/// Check if a directory has a hybrid structure (symlinks pointing to bundle)
#[cfg(target_os = "macos")]
fn is_hybrid_structure(path: &Path) -> bool {
    // Check if bin is a symlink
    let bin_path = path.join("bin");
    if let Ok(metadata) = fs::symlink_metadata(&bin_path)
        && metadata.file_type().is_symlink()
    {
        // Try to read the symlink target
        if let Ok(target) = fs::read_link(&bin_path) {
            // Check if it points to a Contents/Home structure
            let target_str = target.to_string_lossy();
            if target_str.contains("Contents/Home") {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "macos"))]
fn is_hybrid_structure(_path: &Path) -> bool {
    false
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

    #[test]
    fn test_detect_jdk_root_direct_structure() -> Result<()> {
        // Create a temporary directory with direct JDK structure
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path();

        // Create JDK structure
        fs::create_dir_all(jdk_path.join("bin"))?;
        fs::create_dir_all(jdk_path.join("lib"))?;
        fs::create_dir_all(jdk_path.join("conf"))?;

        // Create java binary
        let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
        File::create(jdk_path.join("bin").join(java_binary))?;

        // Test detection
        let result = detect_jdk_root(jdk_path)?;
        assert_eq!(result.jdk_root, jdk_path);
        assert_eq!(result.structure_type, JdkStructureType::Direct);
        assert_eq!(result.java_home_suffix, "");

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_jdk_root_bundle_structure() -> Result<()> {
        // Create a temporary directory with macOS bundle structure
        let temp_dir = tempdir()?;
        let bundle_path = temp_dir.path();

        // Create bundle structure
        let contents_home = bundle_path.join("Contents").join("Home");
        fs::create_dir_all(contents_home.join("bin"))?;
        fs::create_dir_all(contents_home.join("lib"))?;
        fs::create_dir_all(bundle_path.join("Contents").join("MacOS"))?;

        // Create java binary
        File::create(contents_home.join("bin").join("java"))?;

        // Test detection
        let result = detect_jdk_root(bundle_path)?;
        assert_eq!(result.jdk_root, contents_home);
        assert_eq!(result.structure_type, JdkStructureType::Bundle);
        assert_eq!(result.java_home_suffix, "Contents/Home");

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_jdk_root_nested_bundle_structure() -> Result<()> {
        // Create a temporary directory with nested bundle structure
        let temp_dir = tempdir()?;
        let extracted_dir = temp_dir.path();

        // Create nested bundle structure (e.g., jdk-24.0.2+12.jdk/Contents/Home/)
        let jdk_dir = extracted_dir.join("jdk-24.0.2+12.jdk");
        let contents_home = jdk_dir.join("Contents").join("Home");
        fs::create_dir_all(contents_home.join("bin"))?;
        fs::create_dir_all(contents_home.join("lib"))?;
        fs::create_dir_all(jdk_dir.join("Contents").join("MacOS"))?;

        // Create java binary
        File::create(contents_home.join("bin").join("java"))?;

        // Test detection
        let result = detect_jdk_root(extracted_dir)?;
        assert_eq!(result.jdk_root, jdk_dir);
        assert_eq!(result.structure_type, JdkStructureType::Bundle);
        assert_eq!(result.java_home_suffix, "Contents/Home");

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_jdk_root_hybrid_structure() -> Result<()> {
        // Create a temporary directory with hybrid structure (symlinks to bundle)
        let temp_dir = tempdir()?;
        let hybrid_path = temp_dir.path();

        // Create the actual bundle structure
        let bundle_dir = hybrid_path.join("zulu-24.jdk");
        let contents_home = bundle_dir.join("Contents").join("Home");
        fs::create_dir_all(contents_home.join("bin"))?;
        fs::create_dir_all(contents_home.join("lib"))?;
        fs::create_dir_all(contents_home.join("conf"))?;

        // Create java binary in the bundle
        File::create(contents_home.join("bin").join("java"))?;

        // Create symlinks at root pointing to bundle
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(contents_home.join("bin"), hybrid_path.join("bin"))?;
            symlink(contents_home.join("lib"), hybrid_path.join("lib"))?;
            symlink(contents_home.join("conf"), hybrid_path.join("conf"))?;
        }

        // On non-Unix systems, create regular directories as fallback
        #[cfg(not(unix))]
        {
            fs::create_dir_all(hybrid_path.join("bin"))?;
            File::create(hybrid_path.join("bin").join("java"))?;
        }

        // Test detection
        let result = detect_jdk_root(hybrid_path)?;
        assert_eq!(result.jdk_root, hybrid_path);

        #[cfg(unix)]
        assert_eq!(result.structure_type, JdkStructureType::Hybrid);

        #[cfg(not(unix))]
        assert_eq!(result.structure_type, JdkStructureType::Direct);

        assert_eq!(result.java_home_suffix, "");

        Ok(())
    }

    #[test]
    fn test_detect_jdk_root_nested_direct_structure() -> Result<()> {
        // Create a temporary directory with JDK in a subdirectory
        let temp_dir = tempdir()?;
        let extracted_dir = temp_dir.path();

        // Create JDK in subdirectory
        let jdk_subdir = extracted_dir.join("graalvm-jdk-21.0.7+8.1");
        fs::create_dir_all(jdk_subdir.join("bin"))?;
        fs::create_dir_all(jdk_subdir.join("lib"))?;

        // Create java binary
        let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
        File::create(jdk_subdir.join("bin").join(java_binary))?;

        // Test detection
        let result = detect_jdk_root(extracted_dir)?;
        assert_eq!(result.jdk_root, jdk_subdir);
        assert_eq!(result.structure_type, JdkStructureType::Direct);
        assert_eq!(result.java_home_suffix, "");

        Ok(())
    }

    #[test]
    fn test_detect_jdk_root_invalid_structure() {
        // Create a temporary directory without valid JDK structure
        let temp_dir = tempdir().unwrap();
        let invalid_path = temp_dir.path();

        // Create some directories but no bin/java
        fs::create_dir_all(invalid_path.join("lib")).unwrap();
        fs::create_dir_all(invalid_path.join("conf")).unwrap();

        // Test detection - should fail
        let result = detect_jdk_root(invalid_path);
        assert!(result.is_err());

        if let Err(KopiError::ValidationError(msg)) = result {
            assert!(msg.contains("No valid JDK structure found"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_detect_jdk_root_missing_java_binary() {
        // Create a temporary directory with bin but no java
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path();

        // Create bin directory but no java binary
        fs::create_dir_all(path.join("bin")).unwrap();
        fs::create_dir_all(path.join("lib")).unwrap();

        // Test detection - should fail
        let result = detect_jdk_root(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jdk_root() -> Result<()> {
        // Test with valid JDK root
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path();
        fs::create_dir_all(jdk_path.join("bin"))?;

        let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
        File::create(jdk_path.join("bin").join(java_binary))?;

        assert!(validate_jdk_root(jdk_path)?);

        // Test with invalid JDK root (no java binary)
        let invalid_dir = tempdir()?;
        fs::create_dir_all(invalid_dir.path().join("bin"))?;

        assert!(!validate_jdk_root(invalid_dir.path())?);

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_hybrid_structure() -> Result<()> {
        // Test with actual hybrid structure
        let temp_dir = tempdir()?;
        let hybrid_path = temp_dir.path();

        // Create bundle structure
        let bundle_bin = hybrid_path.join("zulu-24.jdk/Contents/Home/bin");
        fs::create_dir_all(&bundle_bin)?;

        // Create symlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&bundle_bin, hybrid_path.join("bin"))?;
            assert!(is_hybrid_structure(hybrid_path));
        }

        // Test with direct structure (no symlinks)
        let direct_dir = tempdir()?;
        fs::create_dir_all(direct_dir.path().join("bin"))?;
        assert!(!is_hybrid_structure(direct_dir.path()));

        Ok(())
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_is_hybrid_structure_non_macos() {
        // On non-macOS platforms, should always return false
        let temp_dir = tempdir().unwrap();
        assert!(!is_hybrid_structure(temp_dir.path()));
    }

    #[test]
    fn test_jdk_structure_type_equality() {
        assert_eq!(JdkStructureType::Direct, JdkStructureType::Direct);
        assert_eq!(JdkStructureType::Bundle, JdkStructureType::Bundle);
        assert_eq!(JdkStructureType::Hybrid, JdkStructureType::Hybrid);
        assert_ne!(JdkStructureType::Direct, JdkStructureType::Bundle);
        assert_ne!(JdkStructureType::Bundle, JdkStructureType::Hybrid);
        assert_ne!(JdkStructureType::Direct, JdkStructureType::Hybrid);
    }

    #[test]
    fn test_jdk_structure_info_fields() {
        let info = JdkStructureInfo {
            jdk_root: PathBuf::from("/test/jdk"),
            structure_type: JdkStructureType::Bundle,
            java_home_suffix: "Contents/Home".to_string(),
        };

        assert_eq!(info.jdk_root, PathBuf::from("/test/jdk"));
        assert_eq!(info.structure_type, JdkStructureType::Bundle);
        assert_eq!(info.java_home_suffix, "Contents/Home");
    }
}
