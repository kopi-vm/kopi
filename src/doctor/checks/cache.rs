use crate::cache::MetadataCache;
use crate::config::KopiConfig;
use crate::doctor::{CheckCategory, CheckResult, CheckStatus, DiagnosticCheck};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MAX_CACHE_SIZE_MB: u64 = 50; // Warn if cache is larger than 50MB

pub struct CacheFileCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> CacheFileCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn get_cache_path(&self) -> PathBuf {
        self.config.kopi_home().join("cache").join("metadata.json")
    }
}

impl<'a> DiagnosticCheck for CacheFileCheck<'a> {
    fn name(&self) -> &str {
        "Cache File Existence"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();
        let cache_path = self.get_cache_path();

        if cache_path.exists() {
            match fs::metadata(&cache_path) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Pass,
                            "Cache file exists",
                            duration,
                        )
                        .with_details(format!("Path: {}", cache_path.display()))
                    } else {
                        CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Fail,
                            "Cache path exists but is not a file",
                            duration,
                        )
                        .with_details(format!("Path: {}", cache_path.display()))
                        .with_suggestion(
                            "Remove the directory and run 'kopi cache update' to recreate cache",
                        )
                    }
                }
                Err(e) => CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Warning,
                    format!("Cache file exists but cannot read metadata: {e}"),
                    duration,
                )
                .with_suggestion("Check file permissions"),
            }
        } else {
            CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                "Cache file does not exist",
                duration,
            )
            .with_details(format!("Expected at: {}", cache_path.display()))
            .with_suggestion("Run 'kopi cache update' to create cache")
        }
    }
}

pub struct CachePermissionsCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> CachePermissionsCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn get_cache_path(&self) -> PathBuf {
        self.config.kopi_home().join("cache").join("metadata.json")
    }
}

impl<'a> DiagnosticCheck for CachePermissionsCheck<'a> {
    fn name(&self) -> &str {
        "Cache File Permissions"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();
        let cache_path = self.get_cache_path();

        if !cache_path.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cache file does not exist",
                duration,
            );
        }

        // Use platform-independent file readability check
        match crate::platform::file_ops::check_file_readable(&cache_path) {
            Ok(is_readable) => {
                if is_readable {
                    // Get permissions string for details
                    let permissions_str =
                        crate::platform::file_ops::get_file_permissions_string(&cache_path)
                            .unwrap_or_else(|_| "unknown".to_string());

                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        "Cache file has correct permissions",
                        duration,
                    )
                    .with_details(format!("Permissions: {permissions_str}"))
                } else {
                    // Get permissions string for details
                    let permissions_str =
                        crate::platform::file_ops::get_file_permissions_string(&cache_path)
                            .unwrap_or_else(|_| "unknown".to_string());

                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Fail,
                        "Cache file is not readable",
                        duration,
                    )
                    .with_details(format!("Permissions: {permissions_str}"))
                    .with_suggestion(if cfg!(unix) {
                        "Run: chmod 644 ~/.kopi/cache/metadata.json"
                    } else {
                        "Check file permissions in Windows Security settings"
                    })
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Cannot check cache permissions: {e}"),
                duration,
            ),
        }
    }
}

pub struct CacheFormatCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> CacheFormatCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn get_cache_path(&self) -> PathBuf {
        self.config.kopi_home().join("cache").join("metadata.json")
    }
}

impl<'a> DiagnosticCheck for CacheFormatCheck<'a> {
    fn name(&self) -> &str {
        "Cache Format Validation"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();
        let cache_path = self.get_cache_path();

        if !cache_path.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cache file does not exist",
                duration,
            );
        }

        match fs::read_to_string(&cache_path) {
            Ok(content) => match serde_json::from_str::<MetadataCache>(&content) {
                Ok(cache) => {
                    let dist_count = cache.distributions.len();
                    let total_packages: usize =
                        cache.distributions.values().map(|d| d.packages.len()).sum();

                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        "Cache format is valid",
                        duration,
                    )
                    .with_details(format!(
                        "Version: {}, Distributions: {}, Total packages: {}",
                        cache.version, dist_count, total_packages
                    ))
                }
                Err(e) => CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Fail,
                    "Cache file has invalid JSON format",
                    duration,
                )
                .with_details(format!("Parse error: {e}"))
                .with_suggestion("Delete cache and run 'kopi cache update' to regenerate"),
            },
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Fail,
                format!("Cannot read cache file: {e}"),
                duration,
            ),
        }
    }
}

pub struct CacheStalenessCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> CacheStalenessCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn get_cache_path(&self) -> PathBuf {
        self.config.kopi_home().join("cache").join("metadata.json")
    }
}

impl<'a> DiagnosticCheck for CacheStalenessCheck<'a> {
    fn name(&self) -> &str {
        "Cache Staleness"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();
        let cache_path = self.get_cache_path();

        if !cache_path.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cache file does not exist",
                duration,
            );
        }

        match fs::read_to_string(&cache_path) {
            Ok(content) => match serde_json::from_str::<MetadataCache>(&content) {
                Ok(cache) => {
                    // Use configured max age from config.cache.max_age_hours
                    let max_age = Duration::from_secs(self.config.cache.max_age_hours * 60 * 60);
                    let max_age_days = self.config.cache.max_age_hours / 24;

                    if cache.is_stale(max_age) {
                        let age_days = chrono::Utc::now()
                            .signed_duration_since(cache.last_updated)
                            .num_days();

                        CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Warning,
                            format!("Cache is {age_days} days old (max age: {max_age_days} days)"),
                            duration,
                        )
                        .with_details(format!(
                            "Last updated: {}",
                            cache.last_updated.format("%Y-%m-%d %H:%M:%S UTC")
                        ))
                        .with_suggestion("Run 'kopi cache update' to refresh cache")
                    } else {
                        let age_days = chrono::Utc::now()
                            .signed_duration_since(cache.last_updated)
                            .num_days();

                        CheckResult::new(
                            self.name(),
                            category,
                            CheckStatus::Pass,
                            format!("Cache is {age_days} days old"),
                            duration,
                        )
                        .with_details(format!(
                            "Last updated: {} (max age: {} days)",
                            cache.last_updated.format("%Y-%m-%d %H:%M:%S UTC"),
                            max_age_days
                        ))
                    }
                }
                Err(_) => CheckResult::new(
                    self.name(),
                    category,
                    CheckStatus::Skip,
                    "Cannot parse cache to check staleness",
                    duration,
                ),
            },
            Err(_) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cannot read cache file",
                duration,
            ),
        }
    }
}

pub struct CacheSizeCheck<'a> {
    config: &'a KopiConfig,
}

impl<'a> CacheSizeCheck<'a> {
    pub fn new(config: &'a KopiConfig) -> Self {
        Self { config }
    }

    fn get_cache_path(&self) -> PathBuf {
        self.config.kopi_home().join("cache").join("metadata.json")
    }
}

impl<'a> DiagnosticCheck for CacheSizeCheck<'a> {
    fn name(&self) -> &str {
        "Cache Size Analysis"
    }

    fn run(&self, start: Instant, category: CheckCategory) -> CheckResult {
        let duration = start.elapsed();
        let cache_path = self.get_cache_path();

        if !cache_path.exists() {
            return CheckResult::new(
                self.name(),
                category,
                CheckStatus::Skip,
                "Cache file does not exist",
                duration,
            );
        }

        match fs::metadata(&cache_path) {
            Ok(metadata) => {
                let size_bytes = metadata.len();
                let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

                if size_mb > MAX_CACHE_SIZE_MB as f64 {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Warning,
                        format!("Cache file is unusually large: {size_mb:.2} MB"),
                        duration,
                    )
                    .with_details(format!("Size: {size_bytes} bytes"))
                    .with_suggestion(
                        "Consider clearing and regenerating cache with 'kopi cache update'",
                    )
                } else {
                    CheckResult::new(
                        self.name(),
                        category,
                        CheckStatus::Pass,
                        format!("Cache size is reasonable: {size_mb:.2} MB"),
                        duration,
                    )
                    .with_details(format!("Size: {size_bytes} bytes"))
                }
            }
            Err(e) => CheckResult::new(
                self.name(),
                category,
                CheckStatus::Warning,
                format!("Cannot check cache size: {e}"),
                duration,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> KopiConfig {
        KopiConfig::new(temp_dir.to_path_buf()).unwrap()
    }

    #[test]
    fn test_cache_check_names() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());

        let file_check = CacheFileCheck::new(&config);
        assert_eq!(file_check.name(), "Cache File Existence");

        let perm_check = CachePermissionsCheck::new(&config);
        assert_eq!(perm_check.name(), "Cache File Permissions");

        let format_check = CacheFormatCheck::new(&config);
        assert_eq!(format_check.name(), "Cache Format Validation");

        let stale_check = CacheStalenessCheck::new(&config);
        assert_eq!(stale_check.name(), "Cache Staleness");

        let size_check = CacheSizeCheck::new(&config);
        assert_eq!(size_check.name(), "Cache Size Analysis");
    }

    #[test]
    fn test_cache_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let check = CacheFileCheck::new(&config);

        let result = check.run(Instant::now(), CheckCategory::Cache);
        assert_eq!(result.status, CheckStatus::Warning);
        assert!(result.message.contains("does not exist"));
    }

    #[test]
    fn test_skip_checks_when_no_cache() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());

        let perm_check = CachePermissionsCheck::new(&config);
        let result = perm_check.run(Instant::now(), CheckCategory::Cache);
        assert_eq!(result.status, CheckStatus::Skip);

        let format_check = CacheFormatCheck::new(&config);
        let result = format_check.run(Instant::now(), CheckCategory::Cache);
        assert_eq!(result.status, CheckStatus::Skip);
    }
}
