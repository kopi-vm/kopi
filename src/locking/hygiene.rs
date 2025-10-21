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

//! Hygiene routines for cleaning up fallback lock artifacts.
//!
//! The hygiene runner executes during CLI startup and removes stale fallback
//! lock files and markers to ensure reliability after crashes.

use crate::config::LockingConfig;
use crate::error::Result;
use crate::locking::fallback::{MARKER_SUFFIX, STAGING_SEGMENT};
use crate::paths::locking::locks_root;
use log::{debug, warn};
use std::cmp;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Summary of a hygiene sweep.
#[derive(Debug, Default, Clone)]
pub struct LockHygieneReport {
    pub removed_locks: usize,
    pub removed_markers: usize,
    pub removed_staging: usize,
    pub errors: usize,
    pub duration: Duration,
}

/// Executes cleanup of stale fallback lock artifacts.
#[derive(Debug, Clone)]
pub struct LockHygieneRunner {
    root: PathBuf,
    age_threshold: Duration,
}

impl LockHygieneRunner {
    pub fn new(root: PathBuf, age_threshold: Duration) -> Self {
        Self {
            root,
            age_threshold,
        }
    }

    /// Derives a conservative age threshold from the configured timeout.
    pub fn default_threshold(timeout: Duration) -> Duration {
        let minimum = Duration::from_secs(600);
        timeout
            .checked_add(Duration::from_secs(60))
            .map(|candidate| cmp::max(candidate, minimum))
            .unwrap_or(minimum)
    }

    pub fn run(&self) -> Result<LockHygieneReport> {
        self.run_with_now(SystemTime::now())
    }

    pub(crate) fn run_with_now(&self, now: SystemTime) -> Result<LockHygieneReport> {
        let start = Instant::now();
        let mut report = LockHygieneReport::default();

        if !self.root.exists() {
            report.duration = start.elapsed();
            return Ok(report);
        }

        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(err) => {
                    warn!("Failed to read lock directory {}: {err}", dir.display());
                    report.errors += 1;
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        warn!("Failed to read entry in {}: {err}", dir.display());
                        report.errors += 1;
                        continue;
                    }
                };
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(err) => {
                        warn!(
                            "Failed to determine file type for {}: {err}",
                            path.display()
                        );
                        report.errors += 1;
                        continue;
                    }
                };

                if file_type.is_dir() {
                    stack.push(path);
                    continue;
                }

                if !file_type.is_file() {
                    continue;
                }

                if is_marker(&path) {
                    process_marker(&path, now, self.age_threshold, &mut report);
                } else if is_staging(&path) {
                    process_staging(&path, now, self.age_threshold, &mut report);
                }
            }
        }

        report.duration = start.elapsed();
        debug!(
            "Lock hygiene sweep removed {} lock(s), {} marker(s), {} staging file(s) in {:.3}s (errors: {})",
            report.removed_locks,
            report.removed_markers,
            report.removed_staging,
            report.duration.as_secs_f64(),
            report.errors
        );
        Ok(report)
    }
}

pub fn run_startup_hygiene(kopi_home: &Path, locking: &LockingConfig) -> Result<LockHygieneReport> {
    let root = locks_root(kopi_home);
    let threshold = LockHygieneRunner::default_threshold(locking.timeout());
    let runner = LockHygieneRunner::new(root, threshold);
    runner.run()
}

fn process_marker(
    path: &Path,
    now: SystemTime,
    threshold: Duration,
    report: &mut LockHygieneReport,
) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            warn!(
                "Failed to read metadata for marker {}: {err}",
                path.display()
            );
            report.errors += 1;
            return;
        }
    };

    if !is_stale(&metadata, now, threshold) {
        return;
    }

    let lock_path = marker_to_lock_path(path);
    match remove_file_if_exists(&lock_path) {
        Ok(true) => report.removed_locks += 1,
        Ok(false) => {}
        Err(err) => {
            warn!(
                "Failed to remove fallback lock file {}: {err}",
                lock_path.display()
            );
            report.errors += 1;
        }
    }

    match remove_file_if_exists(path) {
        Ok(true) => report.removed_markers += 1,
        Ok(false) => {}
        Err(err) => {
            warn!("Failed to remove fallback marker {}: {err}", path.display());
            report.errors += 1;
        }
    }
}

fn process_staging(
    path: &Path,
    now: SystemTime,
    threshold: Duration,
    report: &mut LockHygieneReport,
) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            warn!(
                "Failed to read metadata for staging file {}: {err}",
                path.display()
            );
            report.errors += 1;
            return;
        }
    };

    if !is_stale(&metadata, now, threshold) {
        return;
    }

    match remove_file_if_exists(path) {
        Ok(true) => report.removed_staging += 1,
        Ok(false) => {}
        Err(err) => {
            warn!(
                "Failed to remove fallback staging file {}: {err}",
                path.display()
            );
            report.errors += 1;
        }
    }
}

fn is_marker(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(MARKER_SUFFIX))
        .unwrap_or(false)
}

fn is_staging(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.contains(STAGING_SEGMENT))
        .unwrap_or(false)
}

fn is_stale(metadata: &fs::Metadata, now: SystemTime, threshold: Duration) -> bool {
    match metadata.modified() {
        Ok(modified) => match now.duration_since(modified) {
            Ok(age) => age >= threshold,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn marker_to_lock_path(marker: &Path) -> PathBuf {
    let file_name = marker
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let trimmed = file_name.trim_end_matches(MARKER_SUFFIX);
    marker.with_file_name(trimmed)
}

fn remove_file_if_exists(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::locking;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_root() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let root = locking::locks_root(temp.path());
        fs::create_dir_all(&root).unwrap();
        (temp, root)
    }

    #[test]
    fn stale_fallback_artifacts_are_removed() {
        let (_temp, root) = setup_root();
        let lock_path = root.join("cache.lock");
        let marker_path = root.join("cache.lock.marker");
        write_file(&lock_path, b"fallback");
        write_file(&marker_path, b"marker");

        let runner = LockHygieneRunner::new(root.clone(), Duration::from_secs(1));
        let now = SystemTime::now() + Duration::from_secs(5);
        let report = runner.run_with_now(now).unwrap();

        assert_eq!(report.removed_locks, 1);
        assert_eq!(report.removed_markers, 1);
        assert!(!lock_path.exists());
        assert!(!marker_path.exists());
    }

    #[test]
    fn fresh_artifacts_are_preserved() {
        let (_temp, root) = setup_root();
        let lock_path = root.join("install.lock");
        let marker_path = root.join("install.lock.marker");
        write_file(&lock_path, b"fallback");
        write_file(&marker_path, b"marker");

        let runner = LockHygieneRunner::new(root.clone(), Duration::from_secs(10));
        let now = SystemTime::now() + Duration::from_secs(5);
        let report = runner.run_with_now(now).unwrap();

        assert_eq!(report.removed_locks, 0);
        assert_eq!(report.removed_markers, 0);
        assert!(lock_path.exists());
        assert!(marker_path.exists());
    }

    #[test]
    fn stale_staging_files_are_removed() {
        let (_temp, root) = setup_root();
        let staging_path = root.join("cache.lock.staging-1234");
        write_file(&staging_path, b"pending");

        let runner = LockHygieneRunner::new(root.clone(), Duration::from_secs(1));
        let now = SystemTime::now() + Duration::from_secs(5);
        let report = runner.run_with_now(now).unwrap();

        assert_eq!(report.removed_staging, 1);
        assert!(!staging_path.exists());
    }

    fn write_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
    }
}
