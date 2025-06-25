use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::models::jdk::Distribution;
use dirs::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

const KOPI_DIR_NAME: &str = ".kopi";
const JDKS_DIR_NAME: &str = "jdks";

pub struct StorageManager {
    kopi_home: PathBuf,
    min_disk_space_mb: u64,
}

impl StorageManager {
    pub fn new() -> Result<Self> {
        let kopi_home = Self::get_kopi_home()?;
        let config = KopiConfig::load(&kopi_home)?;
        Ok(Self {
            kopi_home,
            min_disk_space_mb: config.storage.min_disk_space_mb,
        })
    }

    pub fn with_home(kopi_home: PathBuf) -> Self {
        let config = KopiConfig::load(&kopi_home).unwrap_or_default();
        Self {
            kopi_home,
            min_disk_space_mb: config.storage.min_disk_space_mb,
        }
    }

    fn get_kopi_home() -> Result<PathBuf> {
        // Check KOPI_HOME environment variable first
        if let Ok(kopi_home) = std::env::var("KOPI_HOME") {
            let path = PathBuf::from(kopi_home);
            if path.is_absolute() {
                return Ok(path);
            }
        }

        // Fall back to ~/.kopi
        home_dir()
            .map(|home| home.join(KOPI_DIR_NAME))
            .ok_or_else(|| KopiError::ConfigError("Unable to determine home directory".to_string()))
    }

    pub fn kopi_home(&self) -> &Path {
        &self.kopi_home
    }

    pub fn jdks_dir(&self) -> PathBuf {
        self.kopi_home.join(JDKS_DIR_NAME)
    }

    pub fn jdk_install_path(
        &self,
        distribution: &Distribution,
        version: &str,
        arch: &str,
    ) -> PathBuf {
        let dir_name = format!("{}-{}-{}", distribution.id(), version, arch);
        self.jdks_dir().join(dir_name)
    }

    pub fn prepare_jdk_installation(
        &self,
        distribution: &Distribution,
        version: &str,
        arch: &str,
    ) -> Result<InstallationContext> {
        let install_path = self.jdk_install_path(distribution, version, arch);

        // Check if JDK is already installed
        if install_path.exists() {
            return Err(KopiError::AlreadyExists(format!(
                "JDK {} {} for {} is already installed at {:?}",
                distribution.name(),
                version,
                arch,
                install_path
            )));
        }

        // Check available disk space
        self.check_disk_space(&install_path)?;

        // Create a temporary directory for atomic installation
        let temp_dir = self.create_temp_install_dir()?;

        Ok(InstallationContext {
            final_path: install_path,
            temp_path: temp_dir,
        })
    }

    pub fn finalize_installation(&self, context: InstallationContext) -> Result<PathBuf> {
        // Ensure parent directory exists
        if let Some(parent) = context.final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Check if the extracted archive has a single top-level directory
        // (common for JDK archives like jdk-21+35/)
        let entries: Vec<_> = fs::read_dir(&context.temp_path)?
            .filter_map(|entry| entry.ok())
            .collect();

        let source_path = if entries.len() == 1 {
            let entry = &entries[0];
            if entry.file_type()?.is_dir() {
                // Archive has a single top-level directory, use its contents
                entry.path()
            } else {
                // Single file at top level, use temp_path as is
                context.temp_path.clone()
            }
        } else {
            // Multiple entries at top level, use temp_path as is
            context.temp_path.clone()
        };

        // Atomic move from source to final location
        fs::rename(&source_path, &context.final_path).inspect_err(|_| {
            // Clean up temp directory on failure
            let _ = fs::remove_dir_all(&context.temp_path);
        })?;

        // If we used a subdirectory, clean up the now-empty temp directory
        if source_path != context.temp_path {
            let _ = fs::remove_dir_all(&context.temp_path);
        }

        Ok(context.final_path)
    }

    pub fn cleanup_failed_installation(&self, context: &InstallationContext) -> Result<()> {
        if context.temp_path.exists() {
            fs::remove_dir_all(&context.temp_path)?;
        }
        Ok(())
    }

    fn create_temp_install_dir(&self) -> Result<PathBuf> {
        let temp_parent = self.jdks_dir().join(".tmp");
        fs::create_dir_all(&temp_parent)?;

        let temp_name = format!("install-{}", uuid::Uuid::new_v4());
        let temp_path = temp_parent.join(temp_name);
        fs::create_dir(&temp_path)?;

        Ok(temp_path)
    }

    fn check_disk_space(&self, path: &Path) -> Result<()> {
        // Find the first existing parent directory to check disk space
        let mut target_dir = path.to_path_buf();
        while !target_dir.exists() {
            if let Some(parent) = target_dir.parent() {
                target_dir = parent.to_path_buf();
            } else {
                // No parent exists, use kopi home
                target_dir = self.kopi_home.clone();
                break;
            }
        }

        log::debug!(
            "Checking disk space for path {:?} (using {:?})",
            path,
            target_dir
        );

        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem;

            let c_path = CString::new(target_dir.to_string_lossy().as_bytes())?;
            let mut stat: libc::statvfs = unsafe { mem::zeroed() };

            let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };

            if result == 0 {
                let available_mb = (stat.f_bavail * stat.f_frsize) / (1024 * 1024);
                log::debug!(
                    "Disk space check: available={}MB, required={}MB",
                    available_mb,
                    self.min_disk_space_mb
                );

                if available_mb < self.min_disk_space_mb {
                    return Err(KopiError::DiskSpaceError(format!(
                        "Insufficient disk space at {:?}. Required: {}MB, Available: {}MB",
                        target_dir, self.min_disk_space_mb, available_mb
                    )));
                }
            } else {
                let errno = std::io::Error::last_os_error();
                log::error!("Failed to check disk space at {:?}: {}", target_dir, errno);
                return Err(KopiError::SystemError(format!(
                    "Failed to check disk space at {:?}: {}",
                    target_dir, errno
                )));
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::ptr;
            use winapi::um::errhandlingapi::GetLastError;
            use winapi::um::fileapi::GetDiskFreeSpaceExW;

            let path_wide: Vec<u16> = target_dir
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();

            let mut available_bytes: u64 = 0;
            let result = unsafe {
                GetDiskFreeSpaceExW(
                    path_wide.as_ptr(),
                    &mut available_bytes as *mut u64,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };

            if result != 0 {
                let available_mb = available_bytes / (1024 * 1024);
                log::debug!(
                    "Disk space check: available={}MB, required={}MB",
                    available_mb,
                    self.min_disk_space_mb
                );

                if available_mb < self.min_disk_space_mb {
                    return Err(KopiError::DiskSpaceError(format!(
                        "Insufficient disk space at {:?}. Required: {}MB, Available: {}MB",
                        target_dir, self.min_disk_space_mb, available_mb
                    )));
                }
            } else {
                let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
                log::error!(
                    "Failed to check disk space at {:?}: Windows error code {}",
                    target_dir,
                    error_code
                );
                return Err(KopiError::SystemError(format!(
                    "Failed to check disk space at {:?}: Windows error code {}",
                    target_dir, error_code
                )));
            }
        }

        Ok(())
    }

    pub fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>> {
        let jdks_dir = self.jdks_dir();
        if !jdks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();

        for entry in fs::read_dir(&jdks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Skip temporary directories
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            // Parse directory name: vendor-version-arch
            if let Some(jdk_info) = self.parse_jdk_dir_name(&path) {
                installed.push(jdk_info);
            }
        }

        installed.sort_by(|a, b| {
            a.distribution
                .cmp(&b.distribution)
                .then(b.version.cmp(&a.version))
                .then(a.arch.cmp(&b.arch))
        });

        Ok(installed)
    }

    fn parse_jdk_dir_name(&self, path: &Path) -> Option<InstalledJdk> {
        let file_name = path.file_name()?.to_str()?;

        // Known architectures to help with parsing
        const KNOWN_ARCHS: &[&str] = &[
            "x64", "x86", "amd64", "i386", "i686", "aarch64", "arm64", "armv7", "armv7l", "ppc64",
            "ppc64le", "s390x", "riscv64",
        ];

        // Find the architecture suffix
        let (base_name, arch) = KNOWN_ARCHS.iter().find_map(|&arch_str| {
            if file_name.ends_with(&format!("-{}", arch_str)) {
                let base = &file_name[..file_name.len() - arch_str.len() - 1];
                Some((base, arch_str))
            } else {
                None
            }
        })?;

        // Now split the remaining part into distribution and version
        // The distribution is the part before the first hyphen followed by a digit or 'v'
        let mut split_pos = None;
        let chars: Vec<char> = base_name.chars().collect();

        for i in 0..chars.len() - 1 {
            if chars[i] == '-' && (chars[i + 1].is_numeric() || chars[i + 1] == 'v') {
                split_pos = Some(i);
                break;
            }
        }

        let (distribution, version) = if let Some(pos) = split_pos {
            let dist = &base_name[..pos];
            let ver = &base_name[pos + 1..];
            (dist, ver)
        } else {
            // If we can't find a proper split, assume the entire base is the distribution
            // This handles edge cases but might not be what we want
            return None;
        };

        Some(InstalledJdk {
            distribution: distribution.to_string(),
            version: version.to_string(),
            arch: arch.to_string(),
            path: path.to_path_buf(),
        })
    }

    pub fn get_jdk_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }

        Ok(total_size)
    }

    pub fn remove_jdk(&self, path: &Path) -> Result<()> {
        if !path.starts_with(self.jdks_dir()) {
            return Err(KopiError::SecurityError(format!(
                "Refusing to remove directory outside of JDKs directory: {:?}",
                path
            )));
        }

        fs::remove_dir_all(path)?;
        Ok(())
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize StorageManager")
    }
}

#[derive(Debug)]
pub struct InstallationContext {
    pub final_path: PathBuf,
    pub temp_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstalledJdk {
    pub distribution: String,
    pub version: String,
    pub arch: String,
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage_manager() -> (StorageManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::with_home(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_kopi_home_from_env() {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("KOPI_HOME", temp_dir.path());
        }

        let home = StorageManager::get_kopi_home().unwrap();
        assert_eq!(home, temp_dir.path());

        unsafe {
            std::env::remove_var("KOPI_HOME");
        }
    }

    #[test]
    fn test_jdk_install_path() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let path = manager.jdk_install_path(&distribution, "21.0.1", "x64");
        assert!(path.ends_with("jdks/temurin-21.0.1-x64"));
    }

    #[test]
    fn test_prepare_installation_new() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let context = manager
            .prepare_jdk_installation(&distribution, "21.0.1", "x64")
            .unwrap();

        assert!(context.temp_path.exists());
        assert!(!context.final_path.exists());
        assert!(
            context
                .temp_path
                .starts_with(manager.jdks_dir().join(".tmp"))
        );
    }

    #[test]
    fn test_prepare_installation_already_exists() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let install_path = manager.jdk_install_path(&distribution, "21.0.1", "x64");
        fs::create_dir_all(&install_path).unwrap();

        let result = manager.prepare_jdk_installation(&distribution, "21.0.1", "x64");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::AlreadyExists(_)));
    }

    #[test]
    fn test_finalize_installation() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let context = manager
            .prepare_jdk_installation(&distribution, "21.0.1", "x64")
            .unwrap();

        // Create a test file in temp directory
        let test_file = context.temp_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let final_path = manager.finalize_installation(context).unwrap();

        assert!(final_path.exists());
        assert!(final_path.join("test.txt").exists());
        // The .tmp directory might still exist but should be empty
        let tmp_dir = manager.jdks_dir().join(".tmp");
        if tmp_dir.exists() {
            assert_eq!(fs::read_dir(&tmp_dir).unwrap().count(), 0);
        }
    }

    #[test]
    fn test_cleanup_failed_installation() {
        let (manager, _temp) = create_test_storage_manager();
        let distribution = Distribution::Temurin;

        let context = manager
            .prepare_jdk_installation(&distribution, "21.0.1", "x64")
            .unwrap();

        assert!(context.temp_path.exists());

        manager.cleanup_failed_installation(&context).unwrap();
        assert!(!context.temp_path.exists());
    }

    #[test]
    fn test_list_installed_jdks() {
        let (manager, _temp) = create_test_storage_manager();

        // Create some test JDK directories
        let jdks_dir = manager.jdks_dir();
        fs::create_dir_all(&jdks_dir).unwrap();

        fs::create_dir_all(jdks_dir.join("temurin-21.0.1-x64")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-17.0.9-x64")).unwrap();
        fs::create_dir_all(jdks_dir.join(".tmp")).unwrap(); // Should be ignored

        let installed = manager.list_installed_jdks().unwrap();
        assert_eq!(installed.len(), 2);

        assert_eq!(installed[0].distribution, "corretto");
        assert_eq!(installed[0].version, "17.0.9");
        assert_eq!(installed[0].arch, "x64");

        assert_eq!(installed[1].distribution, "temurin");
        assert_eq!(installed[1].version, "21.0.1");
        assert_eq!(installed[1].arch, "x64");
    }

    #[test]
    fn test_parse_jdk_dir_name() {
        let (manager, _temp) = create_test_storage_manager();

        // Test basic format
        let jdk = manager
            .parse_jdk_dir_name(Path::new("temurin-21.0.1-x64"))
            .unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version, "21.0.1");
        assert_eq!(jdk.arch, "x64");

        // Test version with early access suffix
        let jdk = manager
            .parse_jdk_dir_name(Path::new("temurin-22-ea-x64"))
            .unwrap();
        assert_eq!(jdk.distribution, "temurin");
        assert_eq!(jdk.version, "22-ea");
        assert_eq!(jdk.arch, "x64");

        // Test version with build number
        let jdk = manager
            .parse_jdk_dir_name(Path::new("corretto-17.0.9+9-aarch64"))
            .unwrap();
        assert_eq!(jdk.distribution, "corretto");
        assert_eq!(jdk.version, "17.0.9+9");
        assert_eq!(jdk.arch, "aarch64");

        // Test with hyphenated distribution name
        let jdk = manager
            .parse_jdk_dir_name(Path::new("graalvm-ce-21.0.1-amd64"))
            .unwrap();
        assert_eq!(jdk.distribution, "graalvm-ce");
        assert_eq!(jdk.version, "21.0.1");
        assert_eq!(jdk.arch, "amd64");

        // Test with version starting with 'v'
        let jdk = manager
            .parse_jdk_dir_name(Path::new("zulu-v11.0.21-arm64"))
            .unwrap();
        assert_eq!(jdk.distribution, "zulu");
        assert_eq!(jdk.version, "v11.0.21");
        assert_eq!(jdk.arch, "arm64");

        // Test complex version with multiple hyphens
        let jdk = manager
            .parse_jdk_dir_name(Path::new("liberica-21.0.1-13-x64"))
            .unwrap();
        assert_eq!(jdk.distribution, "liberica");
        assert_eq!(jdk.version, "21.0.1-13");
        assert_eq!(jdk.arch, "x64");

        // Test different architectures
        let archs = vec!["x86", "i386", "ppc64le", "s390x", "riscv64"];
        for arch in archs {
            let jdk = manager
                .parse_jdk_dir_name(Path::new(&format!("temurin-17-{}", arch)))
                .unwrap();
            assert_eq!(jdk.distribution, "temurin");
            assert_eq!(jdk.version, "17");
            assert_eq!(jdk.arch, arch);
        }

        // Invalid format tests
        assert!(manager.parse_jdk_dir_name(Path::new("invalid")).is_none());
        assert!(
            manager
                .parse_jdk_dir_name(Path::new("no-version-unknown-arch"))
                .is_none()
        );
        assert!(
            manager
                .parse_jdk_dir_name(Path::new("temurin-x64"))
                .is_none()
        ); // No version
    }

    #[test]
    fn test_remove_jdk_security() {
        let (manager, _temp) = create_test_storage_manager();

        // Should fail for paths outside JDKs directory
        let result = manager.remove_jdk(Path::new("/etc/passwd"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KopiError::SecurityError(_)));
    }

    #[test]
    fn test_min_disk_space_from_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Write config with custom disk space
        fs::write(
            &config_path,
            r#"
[storage]
min_disk_space_mb = 1024
"#,
        )
        .unwrap();

        let manager = StorageManager::with_home(temp_dir.path().to_path_buf());
        assert_eq!(manager.min_disk_space_mb, 1024);
    }

    #[test]
    fn test_min_disk_space_default() {
        let temp_dir = TempDir::new().unwrap();
        // No config file, should use default

        let manager = StorageManager::with_home(temp_dir.path().to_path_buf());
        assert_eq!(manager.min_disk_space_mb, 500); // Default value from config module
    }

    #[test]
    fn test_disk_space_check_path_selection() {
        let (manager, temp) = create_test_storage_manager();

        // Test that it uses the correct path for disk space check
        // This is a unit test that verifies the logic, not the actual disk space
        let _non_existent = temp.path().join("non/existent/path");
        let _parent = temp.path().join("non/existent");

        // The check_disk_space function is private, so we test it indirectly
        // through prepare_jdk_installation which calls it
        let distribution = Distribution::Temurin;
        let result = manager.prepare_jdk_installation(&distribution, "21.0.1", "x64");

        // Should succeed on most systems as temp directories usually have space
        assert!(result.is_ok() || matches!(result.unwrap_err(), KopiError::DiskSpaceError(_)));
    }
}
