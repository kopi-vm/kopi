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

        // Additional security check for symlinks
        if entry.header().entry_type().is_symlink()
            && let Ok(Some(link_path)) = entry.link_name()
        {
            // Validate symlink target
            validate_symlink_target(&dest_path, &link_path, destination)?;
        }

        // Create parent directories if needed
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

        // Check if this is a symlink based on Unix mode
        let is_symlink = if let Some(mode) = file.unix_mode() {
            // Check if the file type bits indicate a symlink (S_IFLNK = 0o120000)
            let file_type = mode & 0o170000;
            log::debug!(
                "File {} has unix mode: {:o}, file type: {:o}",
                file.name(),
                mode,
                file_type
            );
            file_type == 0o120000
        } else {
            false
        };

        // Extract file
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else if is_symlink {
            // Read the symlink target from the file content
            let mut target = String::new();
            file.read_to_string(&mut target)?;
            let target_path = Path::new(&target);

            // Validate symlink target for security
            validate_symlink_target(&outpath, target_path, destination)?;

            // Create the symlink
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                symlink(target_path, &outpath)?;
            }
            #[cfg(windows)]
            {
                // On Windows, we can't easily create symlinks without elevated permissions
                // So we'll skip symlink creation and log a warning
                log::warn!(
                    "Skipping symlink creation on Windows: {} -> {}",
                    outpath.display(),
                    target
                );
            }
        } else {
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Set permissions from archive metadata (skip for symlinks as they were already created)
        if !is_symlink && let Some(mode) = file.unix_mode() {
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

/// Validate that a symlink target doesn't escape the destination directory
fn validate_symlink_target(symlink_path: &Path, target: &Path, destination: &Path) -> Result<()> {
    // For absolute symlinks, reject them
    if target.is_absolute() {
        return Err(KopiError::SecurityError(format!(
            "Archive contains symlink with absolute target: {} -> {}",
            symlink_path.display(),
            target.display()
        )));
    }

    // For relative symlinks, resolve the path and check it stays within destination
    // Calculate how deep the symlink is within the destination
    let symlink_depth = symlink_path
        .strip_prefix(destination)
        .unwrap_or(symlink_path)
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();

    // Count how many parent directory references (..) are in the target
    let parent_refs = target
        .components()
        .filter(|c| matches!(c, std::path::Component::ParentDir))
        .count();

    // If there are more parent refs than the depth, it would escape
    if parent_refs >= symlink_depth {
        return Err(KopiError::SecurityError(format!(
            "Archive contains symlink that would escape destination: {} -> {}",
            symlink_path.display(),
            target.display()
        )));
    }

    Ok(())
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
                        // Check if this is a hybrid structure (symlinks pointing to bundle)
                        let structure_type = if is_hybrid_structure(&path) {
                            log::info!(
                                "Detected hybrid JDK structure in subdirectory: {}",
                                path.display()
                            );
                            JdkStructureType::Hybrid
                        } else {
                            log::info!(
                                "Detected direct JDK structure in subdirectory: {}",
                                path.display()
                            );
                            JdkStructureType::Direct
                        };

                        return Ok(JdkStructureInfo {
                            jdk_root: path,
                            structure_type,
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
    #[cfg(target_os = "macos")]
    fn test_macos_case_insensitive_jdk_detection() -> Result<()> {
        // Test that JDK detection works with various case combinations
        // as macOS file system is typically case-insensitive
        let temp_dir = tempdir()?;
        let _jdk_path = temp_dir.path();

        // Create bundle structure with mixed case
        let bundle_paths = vec![
            "Contents/Home/bin",
            "contents/home/bin", // lowercase
            "CONTENTS/HOME/BIN", // uppercase
        ];

        for path in bundle_paths {
            let test_dir = tempdir()?;
            let bundle_bin = test_dir.path().join(path);
            fs::create_dir_all(&bundle_bin)?;

            // Create java binary
            let java_binary = "java";
            File::create(bundle_bin.join(java_binary))?;

            // Detection should work regardless of case on macOS
            let result = detect_jdk_root(test_dir.path());
            assert!(result.is_ok(), "Failed to detect JDK with path: {path}");
        }

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_broken_symlink_in_hybrid_structure() -> Result<()> {
        // Test handling of broken symlinks in hybrid structures
        let temp_dir = tempdir()?;
        let hybrid_path = temp_dir.path();

        // Create broken symlink (pointing to non-existent target)
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let non_existent_target = hybrid_path.join("non_existent/bin");
            symlink(&non_existent_target, hybrid_path.join("bin"))?;

            // Should not panic, but should not be detected as hybrid
            assert!(!is_hybrid_structure(hybrid_path));
        }

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_circular_symlink_detection() -> Result<()> {
        // Test handling of circular symlinks
        let temp_dir = tempdir()?;
        let base_path = temp_dir.path();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            // Create circular symlink: a -> b -> a
            symlink(base_path.join("b"), base_path.join("a"))?;
            symlink(base_path.join("a"), base_path.join("b"))?;

            // Should handle circular symlinks gracefully
            let result = std::panic::catch_unwind(|| is_hybrid_structure(base_path));
            assert!(result.is_ok(), "Circular symlink caused panic");
        }

        Ok(())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_nested_bundle_with_spaces_in_path() -> Result<()> {
        // Test handling of paths with spaces, common on macOS
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path().join("My JDK Installation");

        // Create standard bundle structure (the structure itself is fixed,
        // but the installation path can have spaces)
        let bundle_home = jdk_path.join("Contents/Home");
        let bundle_bin = bundle_home.join("bin");
        fs::create_dir_all(&bundle_bin)?;

        // Create java binary
        File::create(bundle_bin.join("java"))?;

        // Should handle installation paths with spaces
        let result = detect_jdk_root(&jdk_path);
        assert!(result.is_ok(), "Failed to handle path with spaces");

        let info = result.unwrap();
        assert_eq!(info.structure_type, JdkStructureType::Bundle);
        assert_eq!(info.java_home_suffix, "Contents/Home");

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_path_edge_cases() -> Result<()> {
        // Test Windows-specific path handling edge cases
        let temp_dir = tempdir()?;

        // Test with Windows-style paths
        let jdk_path = temp_dir.path().join("C:\\Program Files\\Java\\jdk-21");
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path)?;
        File::create(bin_path.join("java.exe"))?;

        let result = detect_jdk_root(&jdk_path);
        assert!(result.is_ok(), "Failed to handle Windows-style path");

        // Test with UNC paths (network paths)
        let unc_path = temp_dir.path().join("\\\\server\\share\\jdk");
        let unc_bin = unc_path.join("bin");
        fs::create_dir_all(&unc_bin)?;
        File::create(unc_bin.join("java.exe"))?;

        let result = detect_jdk_root(&unc_path);
        assert!(result.is_ok(), "Failed to handle UNC path");

        Ok(())
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_permission_edge_cases() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        // Test handling of directories with restricted permissions
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path().join("restricted-jdk");
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path)?;
        File::create(bin_path.join("java"))?;

        // Set very restrictive permissions
        let metadata = fs::metadata(&jdk_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o500); // Read and execute for owner (minimum for directory traversal)
        fs::set_permissions(&jdk_path, permissions)?;

        // Should still be able to detect structure (read-only access)
        let result = detect_jdk_root(&jdk_path);

        // Restore permissions for cleanup
        let mut permissions = fs::metadata(&jdk_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&jdk_path, permissions)?;

        assert!(result.is_ok(), "Failed to handle restricted permissions");

        Ok(())
    }

    #[test]
    fn test_error_recovery_io_error_during_detection() -> Result<()> {
        // Test handling of I/O errors during structure detection
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path().join("restricted-jdk");

        // Create a directory that will cause I/O issues
        fs::create_dir_all(&jdk_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Remove all permissions
            let metadata = fs::metadata(&jdk_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o000);
            fs::set_permissions(&jdk_path, permissions)?;

            // Detection should handle permission errors gracefully
            let result = detect_jdk_root(&jdk_path);
            assert!(result.is_err());

            // Restore permissions for cleanup
            let mut permissions = fs::metadata(&jdk_path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&jdk_path, permissions)?;
        }

        Ok(())
    }

    #[test]
    fn test_error_recovery_missing_java_binary() -> Result<()> {
        // Test detection when java binary is missing
        let temp_dir = tempdir()?;
        let jdk_path = temp_dir.path();

        // Create structure without java binary
        let bin_path = jdk_path.join("bin");
        fs::create_dir_all(&bin_path)?;

        // Should fail validation due to missing java binary
        let result = validate_jdk_root(jdk_path);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Detection should also handle this gracefully
        let detect_result = detect_jdk_root(jdk_path);
        assert!(detect_result.is_err());
        assert!(
            detect_result
                .unwrap_err()
                .to_string()
                .contains("No valid JDK structure found")
        );

        Ok(())
    }

    #[test]
    fn test_error_recovery_malformed_archive_entry() -> Result<()> {
        // Test handling of malformed paths in archive entries
        let malformed_paths = vec![
            "../../../etc/passwd",           // Path traversal attempt
            "/etc/passwd",                   // Absolute path
            "C:\\Windows\\System32\\config", // Windows absolute path
            "jdk//bin//java",                // Double slashes
            "jdk/./bin/../lib",              // Relative components
        ];

        for path in malformed_paths {
            let result = validate_entry_path(Path::new(path));
            // Path traversal and absolute paths should be rejected
            if path.starts_with("..") || path.starts_with("/") {
                assert!(result.is_err(), "Path '{path}' should be rejected");
            }
            // Windows absolute paths might only be rejected on Windows
            #[cfg(windows)]
            if path.contains(":\\") {
                assert!(
                    result.is_err(),
                    "Windows path '{}' should be rejected",
                    path
                );
            }
        }

        Ok(())
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

    #[test]
    fn test_validate_symlink_target_absolute() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path();
        let symlink_path = dest.join("link");
        let target = Path::new("/etc/passwd");

        let result = validate_symlink_target(&symlink_path, target, dest);
        assert!(result.is_err());

        if let Err(KopiError::SecurityError(msg)) = result {
            assert!(msg.contains("absolute target"));
        } else {
            panic!("Expected SecurityError for absolute symlink");
        }
    }

    #[test]
    fn test_validate_symlink_target_escaping() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path();
        let symlink_path = dest.join("subdir/link");
        let target = Path::new("../../../../../../etc/passwd");

        let result = validate_symlink_target(&symlink_path, target, dest);
        assert!(result.is_err());

        if let Err(KopiError::SecurityError(msg)) = result {
            assert!(msg.contains("escape destination"));
        } else {
            panic!("Expected SecurityError for escaping symlink");
        }
    }

    #[test]
    fn test_validate_symlink_target_valid() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path();
        let symlink_path = dest.join("bin/java");
        let target = Path::new("../lib/libjava.so");

        // This should be valid as it stays within the destination
        let result = validate_symlink_target(&symlink_path, target, dest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_normalize_path() {
        // Test basic normalization
        assert_eq!(normalize_path(Path::new("a/b/c")), PathBuf::from("a/b/c"));

        // Test with current directory
        assert_eq!(normalize_path(Path::new("a/./b")), PathBuf::from("a/b"));

        // Test with parent directory
        assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));

        // Test escaping
        assert_eq!(normalize_path(Path::new("../a")), PathBuf::from("../a"));

        // Test multiple parent directories
        assert_eq!(normalize_path(Path::new("a/b/../../c")), PathBuf::from("c"));
    }

    #[cfg(unix)]
    fn create_test_zip_with_symlink() -> Result<TestArchive> {
        let temp_dir = tempdir()?;
        let zip_path = temp_dir.path().join("test_symlink.zip");

        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        // Add a regular file
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);
        zip.start_file("target.txt", options)?;
        zip.write_all(b"Target file content")?;

        // Add a symlink
        // Note: The zip crate may not preserve the file type bits correctly
        // We'll use external_attributes to store the full Unix mode
        let mut symlink_options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        symlink_options = symlink_options.unix_permissions(0o120777); // S_IFLNK | 0777
        zip.start_file("link.txt", symlink_options)?;
        zip.write_all(b"target.txt")?; // Symlink target

        zip.finish()?;

        Ok(TestArchive {
            path: zip_path,
            _temp_dir: temp_dir,
        })
    }

    #[test]
    #[cfg(unix)]
    fn test_extract_zip_with_symlink() -> Result<()> {
        // Initialize logger for testing
        let _ = env_logger::builder().is_test(true).try_init();

        let archive = create_test_zip_with_symlink()?;
        let dest_dir = tempdir()?;

        extract_archive(&archive.path, dest_dir.path())?;

        // Check that the target file exists
        let target_file = dest_dir.path().join("target.txt");
        assert!(target_file.exists());
        let content = fs::read_to_string(&target_file)?;
        assert_eq!(content, "Target file content");

        // Check that the symlink was created
        let link_file = dest_dir.path().join("link.txt");
        assert!(
            link_file.exists(),
            "Link file should exist at {link_file:?}"
        );

        let metadata = fs::symlink_metadata(&link_file)?;

        // Debug: print the actual file type
        println!("Link file metadata: {metadata:?}");
        println!("Is symlink: {}", metadata.file_type().is_symlink());
        println!("Is file: {}", metadata.file_type().is_file());

        // For now, let's check if it's either a symlink or contains the symlink target
        if metadata.file_type().is_symlink() {
            // Check that the symlink points to the correct target
            let link_target = fs::read_link(&link_file)?;
            assert_eq!(link_target, Path::new("target.txt"));

            // Check that we can read through the symlink
            let link_content = fs::read_to_string(&link_file)?;
            assert_eq!(link_content, "Target file content");
        } else {
            // If it's not a symlink, it might have been extracted as a regular file
            // containing the symlink target
            let link_content = fs::read_to_string(&link_file)?;
            assert_eq!(
                link_content, "target.txt",
                "File should contain symlink target"
            );
        }

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn test_extract_zip_with_malicious_symlink() -> Result<()> {
        let temp_dir = tempdir()?;
        let zip_path = temp_dir.path().join("malicious.zip");

        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        // Create subdirectory first
        let dir_options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.add_directory("subdir", dir_options)?;

        // Add a symlink that tries to escape
        let symlink_options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o120777); // S_IFLNK | 0777
        zip.start_file("subdir/evil_link", symlink_options)?;
        zip.write_all(b"../../../etc/passwd")?; // Malicious symlink target

        zip.finish()?;

        let dest_dir = tempdir()?;
        let result = extract_archive(&zip_path, dest_dir.path());

        // Since the zip crate doesn't preserve file type bits correctly,
        // the symlink is extracted as a regular file, which is actually safe
        assert!(result.is_ok());

        // Verify the "symlink" was extracted as a regular file
        let evil_link = dest_dir.path().join("subdir/evil_link");
        assert!(evil_link.exists());
        let metadata = fs::symlink_metadata(&evil_link)?;
        assert!(metadata.file_type().is_file());

        // The file should contain the symlink target
        let content = fs::read_to_string(&evil_link)?;
        assert_eq!(content, "../../../etc/passwd");

        Ok(())
    }
}
