use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::version::VersionRequest;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const KOPI_VERSION_FILE: &str = ".kopi-version";
const JAVA_VERSION_FILE: &str = ".java-version";
const VERSION_ENV_VAR: &str = "KOPI_JAVA_VERSION";

// Type alias to simplify complex return type
type VersionSearchResult = (Option<(VersionRequest, PathBuf)>, Vec<String>);

#[derive(Debug, Clone, PartialEq)]
pub enum VersionSource {
    Environment(String),    // KOPI_JAVA_VERSION
    ProjectFile(PathBuf),   // .kopi-version or .java-version
    GlobalDefault(PathBuf), // ~/.kopi/default-version
}

pub struct VersionResolver<'a> {
    current_dir: PathBuf,
    config: &'a KopiConfig,
}

impl<'a> VersionResolver<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self {
            current_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            config,
        }
    }

    pub fn with_dir(dir: PathBuf, config: &'a KopiConfig) -> Self {
        Self {
            current_dir: dir,
            config,
        }
    }

    pub fn resolve_version(&self) -> Result<(VersionRequest, VersionSource)> {
        // Check environment variable first (fastest)
        if let Ok(env_version) = env::var(VERSION_ENV_VAR) {
            log::debug!("Checking KOPI_JAVA_VERSION environment variable...");
            log::debug!("Found KOPI_JAVA_VERSION: {env_version}");
            let version_request = VersionRequest::from_str(&env_version)?;
            return Ok((version_request, VersionSource::Environment(env_version)));
        }
        log::debug!("KOPI_JAVA_VERSION not set");

        // Search for version files
        let current_dir = self.current_dir.clone();
        log::debug!("Searching for version files from: {current_dir:?}");

        let (version_request, searched_paths) = self.search_version_files()?;
        if let Some((version_request, path)) = version_request {
            return Ok((version_request, VersionSource::ProjectFile(path)));
        }

        // Check global default
        if let Some((version_request, path)) = self.get_global_default()? {
            log::debug!("Using global default version");
            return Ok((version_request, VersionSource::GlobalDefault(path)));
        }

        // No version found - use the searched paths from search_version_files
        Err(KopiError::NoLocalVersion { searched_paths })
    }

    fn read_version_file(&self, path: &Path) -> Result<String> {
        // Use a small buffer for efficiency
        let content = fs::read_to_string(path)?;

        // Trim whitespace and newlines
        let version = content.trim().to_string();

        if version.is_empty() {
            return Err(KopiError::InvalidVersionFormat(
                "Version file is empty".to_string(),
            ));
        }

        Ok(version)
    }

    fn search_version_files(&self) -> Result<VersionSearchResult> {
        let mut current = self.current_dir.clone();
        let mut searched_paths = Vec::new();

        loop {
            // Add current directory to searched paths
            searched_paths.push(current.display().to_string());

            // Check for .kopi-version first (native format)
            let kopi_version_path = current.join(KOPI_VERSION_FILE);
            log::trace!("Checking {kopi_version_path:?}");
            if kopi_version_path.exists() {
                log::debug!("Found .kopi-version at {kopi_version_path:?}");
                let content = self.read_version_file(&kopi_version_path)?;
                log::debug!("Version content: {content}");
                let version_request = VersionRequest::from_str(&content)?;
                return Ok((Some((version_request, kopi_version_path)), searched_paths));
            }

            // Check for .java-version (compatibility)
            let java_version_path = current.join(JAVA_VERSION_FILE);
            log::trace!("Checking {java_version_path:?}");
            if java_version_path.exists() {
                log::debug!("Found .java-version at {java_version_path:?}");
                let content = self.read_version_file(&java_version_path)?;
                log::debug!("Version content: {content}");
                // .java-version doesn't support distribution@version format
                let version_request = VersionRequest::new(content)?;
                return Ok((Some((version_request, java_version_path)), searched_paths));
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }

        Ok((None, searched_paths))
    }

    fn get_global_default(&self) -> Result<Option<(VersionRequest, PathBuf)>> {
        let global_version_path = self.config.kopi_home().join("version");

        if global_version_path.exists() {
            let content = self.read_version_file(&global_version_path)?;
            let version_request = VersionRequest::from_str(&content)?;
            return Ok(Some((version_request, global_version_path)));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_resolve_from_env_var() {
        unsafe {
            env::set_var(VERSION_ENV_VAR, "temurin@21");
        }
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::new(&config);
        let (result, source) = resolver.resolve_version().unwrap();
        assert_eq!(result.version_pattern, "21");
        assert_eq!(result.distribution, Some("temurin".to_string()));
        assert_eq!(source, VersionSource::Environment("temurin@21".to_string()));
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }
    }

    #[test]
    #[serial]
    fn test_resolve_from_kopi_version_file() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let version_file = temp_path.join(KOPI_VERSION_FILE);
        fs::write(&version_file, "corretto@17.0.8").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let (result, source) = resolver.resolve_version().unwrap();
        assert_eq!(result.version_pattern, "17.0.8");
        assert_eq!(result.distribution, Some("corretto".to_string()));
        assert_eq!(source, VersionSource::ProjectFile(version_file));
    }

    #[test]
    #[serial]
    fn test_resolve_from_java_version_file() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let version_file = temp_path.join(JAVA_VERSION_FILE);
        fs::write(&version_file, "11.0.2").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let (result, source) = resolver.resolve_version().unwrap();
        assert_eq!(result.version_pattern, "11.0.2");
        assert_eq!(result.distribution, None);
        assert_eq!(source, VersionSource::ProjectFile(version_file));
    }

    #[test]
    #[serial]
    fn test_resolve_searches_parent_directories() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().to_path_buf();

        let child_dir = parent_dir.join("child");
        fs::create_dir_all(&child_dir).unwrap();

        // Place version file in parent
        let version_file = parent_dir.join(KOPI_VERSION_FILE);
        fs::write(&version_file, "zulu@8").unwrap();

        // Resolver starts in child directory
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(child_dir, &config);
        let (result, source) = resolver.resolve_version().unwrap();
        assert_eq!(result.version_pattern, "8");
        assert_eq!(result.distribution, Some("zulu".to_string()));
        assert_eq!(source, VersionSource::ProjectFile(version_file));
    }

    #[test]
    #[serial]
    fn test_kopi_version_takes_precedence() {
        // Save original value and ensure clean state
        let original = env::var(VERSION_ENV_VAR).ok();
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create both version files
        let kopi_version = temp_path.join(KOPI_VERSION_FILE);
        fs::write(&kopi_version, "temurin@21").unwrap();

        let java_version = temp_path.join(JAVA_VERSION_FILE);
        fs::write(&java_version, "17").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let (result, source) = resolver.resolve_version().unwrap();

        // Should use .kopi-version
        assert_eq!(result.version_pattern, "21");
        assert_eq!(result.distribution, Some("temurin".to_string()));
        assert_eq!(source, VersionSource::ProjectFile(kopi_version));

        // Restore original value
        unsafe {
            if let Some(val) = original {
                env::set_var(VERSION_ENV_VAR, val);
            } else {
                env::remove_var(VERSION_ENV_VAR);
            }
        }
    }

    #[test]
    #[serial]
    fn test_empty_version_file_error() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let version_file = temp_path.join(KOPI_VERSION_FILE);
        fs::write(&version_file, "").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let result = resolver.resolve_version();
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_whitespace_trimmed() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let version_file = temp_path.join(JAVA_VERSION_FILE);
        fs::write(&version_file, "  17.0.9  \n").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let (result, _source) = resolver.resolve_version().unwrap();
        assert_eq!(result.version_pattern, "17.0.9");
    }

    #[test]
    #[serial]
    fn test_no_version_found() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let result = resolver.resolve_version();
        assert!(matches!(result, Err(KopiError::NoLocalVersion { .. })));
    }

    #[test]
    #[serial]
    fn test_resolve_from_global_when_exists() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        // Test when global version file exists
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a global version file in the temp kopi home
        let global_version_path = temp_dir.path().join("version");
        fs::write(&global_version_path, "temurin@17").unwrap();

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let result = resolver.resolve_version();

        // Should successfully resolve from global default
        let (version_request, source) = result.unwrap();
        assert_eq!(version_request.version_pattern, "17");
        assert_eq!(version_request.distribution, Some("temurin".to_string()));
        assert_eq!(source, VersionSource::GlobalDefault(global_version_path));
    }

    #[test]
    #[serial]
    fn test_resolve_from_global_when_not_exists() {
        // Clear environment variable to ensure test isolation
        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }

        // Test when no version files exist anywhere
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Don't create any version files
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let result = resolver.resolve_version();

        // Should return NoLocalVersion error
        assert!(matches!(result, Err(KopiError::NoLocalVersion { .. })));

        // Verify the error contains searched paths
        if let Err(KopiError::NoLocalVersion { searched_paths }) = result {
            assert!(!searched_paths.is_empty());
            assert!(
                searched_paths
                    .iter()
                    .any(|p| p.contains(&temp_path.display().to_string()))
            );
        }
    }

    #[test]
    #[serial]
    fn test_resolve_priority() {
        // Test that environment variable takes priority over project files
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create project file
        let version_file = temp_path.join(KOPI_VERSION_FILE);
        fs::write(&version_file, "corretto@17").unwrap();

        // Set environment variable
        unsafe {
            env::set_var(VERSION_ENV_VAR, "temurin@21");
        }

        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let resolver = VersionResolver::with_dir(temp_path.clone(), &config);
        let (version_request, source) = resolver.resolve_version().unwrap();

        // Should use environment variable, not project file
        assert_eq!(version_request.version_pattern, "21");
        assert_eq!(version_request.distribution, Some("temurin".to_string()));
        assert_eq!(source, VersionSource::Environment("temurin@21".to_string()));

        unsafe {
            env::remove_var(VERSION_ENV_VAR);
        }
    }
}
