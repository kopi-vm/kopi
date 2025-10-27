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

use crate::cache::MetadataCache;
use crate::error::{KopiError, Result};
use crate::locking::LockTimeoutValue;
use crate::platform;
use std::cmp::min;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const INITIAL_RENAME_BACKOFF: Duration = Duration::from_millis(50);
const MAX_RENAME_BACKOFF: Duration = Duration::from_millis(1_000);
const CACHE_TEMP_EXTENSION: &str = "tmp";

/// Load metadata cache from a file
pub fn load_cache(path: &Path) -> Result<MetadataCache> {
    let contents = fs::read_to_string(path)
        .map_err(|e| KopiError::ConfigError(format!("Failed to read cache file: {e}")))?;

    let cache: MetadataCache =
        serde_json::from_str(&contents).map_err(|_e| KopiError::InvalidMetadata)?;
    Ok(cache)
}

/// Save metadata cache to a file
pub fn save_cache(
    cache: &MetadataCache,
    path: &Path,
    timeout_budget: LockTimeoutValue,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            KopiError::ConfigError(format!("Failed to create cache directory: {e}"))
        })?;
    }

    let json = serde_json::to_vec_pretty(cache).map_err(|_e| KopiError::InvalidMetadata)?;

    // Write to temporary file first for atomic operation
    let temp_path = path.with_extension(CACHE_TEMP_EXTENSION);

    // Clean up any leftover temp file from previous failed attempts
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|e| {
            KopiError::ConfigError(format!(
                "Failed to remove stale cache temp file '{}': {e}",
                temp_path.display()
            ))
        })?;
    }

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }

    let mut temp_file = options.open(&temp_path).map_err(|e| {
        KopiError::ConfigError(format!(
            "Failed to create cache temp file '{}': {e}",
            temp_path.display()
        ))
    })?;

    #[cfg(not(unix))]
    {
        let mut permissions = temp_file.metadata().map_err(|e| {
            KopiError::ConfigError(format!("Failed to inspect cache temp file metadata: {e}"))
        })?;
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&temp_path, permissions).map_err(|e| {
            KopiError::ConfigError(format!(
                "Failed to set permissions for cache temp file '{}': {e}",
                temp_path.display()
            ))
        })?;
    }

    temp_file.write_all(&json).map_err(|e| {
        KopiError::ConfigError(format!(
            "Failed to write cache temp file '{}': {e}",
            temp_path.display()
        ))
    })?;

    temp_file.sync_all().map_err(|e| {
        KopiError::ConfigError(format!(
            "Failed to flush cache temp file '{}': {e}",
            temp_path.display()
        ))
    })?;
    drop(temp_file);

    // Use platform-specific atomic rename
    rename_with_retry(
        || platform::file_ops::atomic_rename(&temp_path, path),
        timeout_budget,
    )
    .map_err(|failure| {
        let timeout_hint = if timeout_budget.is_infinite() {
            "Configured lock timeout is infinite; ensure no other process holds the cache file open."
                .to_string()
        } else {
            let waited = format_duration(failure.elapsed);
            format!(
                "Waited {waited} while promoting the cache file. Increase the locking timeout or retry after other Kopi processes finish."
            )
        };
        KopiError::ConfigError(format!(
            "Failed to finalise cache write after {} attempt(s): {}. {timeout_hint}",
            failure.attempts.max(1),
            failure.error
        ))
    })?;

    Ok(())
}

struct RenameRetryFailure {
    error: io::Error,
    attempts: usize,
    elapsed: Duration,
}

impl RenameRetryFailure {
    fn new(error: io::Error, attempts: usize, elapsed: Duration) -> Self {
        Self {
            error,
            attempts,
            elapsed,
        }
    }
}

fn rename_with_retry<F>(
    mut rename_fn: F,
    timeout_budget: LockTimeoutValue,
) -> std::result::Result<(), RenameRetryFailure>
where
    F: FnMut() -> io::Result<()>,
{
    let start = Instant::now();
    let deadline = match timeout_budget {
        LockTimeoutValue::Finite(duration) => Some(start + duration),
        LockTimeoutValue::Infinite => None,
    };

    let mut attempts = 0usize;
    let mut backoff = INITIAL_RENAME_BACKOFF;

    loop {
        match rename_fn() {
            Ok(_) => return Ok(()),
            Err(err) => {
                if !should_retry(&err) {
                    return Err(RenameRetryFailure::new(err, attempts, start.elapsed()));
                }

                attempts += 1;

                let now = Instant::now();
                if let Some(deadline) = deadline {
                    if now >= deadline {
                        return Err(RenameRetryFailure::new(err, attempts, start.elapsed()));
                    }
                    let remaining = deadline.saturating_duration_since(now);
                    if remaining.is_zero() {
                        return Err(RenameRetryFailure::new(err, attempts, start.elapsed()));
                    }
                    thread::sleep(min(backoff, remaining));
                } else {
                    thread::sleep(backoff);
                }
                backoff = min(backoff.saturating_mul(2), MAX_RENAME_BACKOFF);
            }
        }
    }
}

fn should_retry(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(code) if code == ERROR_SHARING_VIOLATION || code == ERROR_LOCK_VIOLATION
    )
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::DistributionCache;
    use crate::models::distribution::Distribution as JdkDistribution;
    use tempfile::TempDir;

    #[test]
    fn test_load_nonexistent_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // load_cache should fail for non-existent files
        assert!(load_cache(&cache_path).is_err());
    }

    #[test]
    fn test_save_and_load_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut cache = MetadataCache::new();
        let dist = DistributionCache {
            distribution: JdkDistribution::Temurin,
            display_name: "Eclipse Temurin".to_string(),
            packages: Vec::new(),
        };
        cache.distributions.insert("temurin".to_string(), dist);

        save_cache(&cache, &cache_path, LockTimeoutValue::from_secs(2)).unwrap();

        let loaded_cache = load_cache(&cache_path).unwrap();
        assert_eq!(loaded_cache.version, cache.version);
        assert_eq!(loaded_cache.distributions.len(), 1);
        assert!(loaded_cache.distributions.contains_key("temurin"));

        assert!(
            !cache_path.with_extension(CACHE_TEMP_EXTENSION).exists(),
            "temporary cache file should be removed after successful rename"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&cache_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "cache file should be owner read/write");
        }

        // Ensure stale temp files from previous crashes are removed on subsequent writes.
        let stale_temp = cache_path.with_extension(CACHE_TEMP_EXTENSION);
        fs::write(&stale_temp, b"partial").unwrap();
        save_cache(&cache, &cache_path, LockTimeoutValue::from_secs(2)).unwrap();
        assert!(
            !stale_temp.exists(),
            "stale cache temp file should be removed before rewriting"
        );
    }

    #[test]
    fn rename_retries_on_sharing_violation() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = AtomicUsize::new(0);
        let result = rename_with_retry(
            || {
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(io::Error::from_raw_os_error(ERROR_SHARING_VIOLATION))
                } else {
                    Ok(())
                }
            },
            LockTimeoutValue::from_secs(1),
        );

        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn rename_respects_timeout_budget() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = AtomicUsize::new(0);
        let failure = rename_with_retry(
            || {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(io::Error::from_raw_os_error(ERROR_SHARING_VIOLATION))
            },
            LockTimeoutValue::from_secs(0),
        )
        .expect_err("rename should time out when budget is exhausted");

        assert!(failure.elapsed <= Duration::from_millis(5));
        assert!(
            failure.error.kind() == io::ErrorKind::PermissionDenied
                || failure.error.raw_os_error() == Some(ERROR_SHARING_VIOLATION)
        );
        assert!(attempts.load(Ordering::SeqCst) >= 1);
    }
}
