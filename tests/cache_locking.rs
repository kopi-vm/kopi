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

use chrono::Utc;
use kopi::cache::{DistributionCache, MetadataCache, convert_package_to_jdk_metadata, load_cache};
use kopi::config::{KopiConfig, LockingMode};
use kopi::error::KopiError;
use kopi::indicator::{ProgressIndicator, SilentProgress, StatusReporter};
use kopi::locking::{CacheWriterLockGuard, LockBackend, LockTimeoutValue};
use kopi::models::api::DistributionMetadata;
use kopi::models::distribution::Distribution;
use kopi::version::Version;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex as StdMutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const SAMPLE_FOOJAY_PACKAGE: &str = r#"{
  "distribution": {
    "id": "temurin",
    "name": "Eclipse Temurin",
    "api_parameter": "temurin",
    "maintained": true,
    "available": true,
    "build_of_openjdk": true,
    "build_of_graalvm": false,
    "official_uri": "https://adoptium.net",
    "free_use_in_production": true,
    "synonyms": ["temurin", "adoptium"],
    "versions": ["21.0.3+9"]
  },
  "packages": [
    {
      "id": "temurin-21",
      "archive_type": "tar.gz",
      "distribution": "temurin",
      "major_version": 21,
      "java_version": "21.0.3",
      "distribution_version": "21.0.3+9",
      "jdk_version": 21,
      "directly_downloadable": true,
      "filename": "OpenJDK21U-jdk_x64_linux_hotspot_21.0.3_9.tar.gz",
      "links": {
        "pkg_download_redirect": "https://api.foojay.io/download/temurin-21",
        "pkg_info_uri": "https://api.foojay.io/info/temurin-21"
      },
      "free_use_in_production": true,
      "tck_tested": "yes",
      "size": 192000000,
      "operating_system": "linux",
      "architecture": "x64",
      "lib_c_type": "glibc",
      "package_type": "JDK",
      "javafx_bundled": false,
      "term_of_support": "lts",
      "release_status": "ga",
      "latest_build_available": true
    }
  ]
}"#;

#[test]
fn concurrent_cache_writers_block_until_release() {
    let temp_home = TempDir::new().unwrap();
    let mut config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
    config
        .locking
        .set_timeout_value(LockTimeoutValue::from_secs(2));

    let holder_config = config.clone();
    let barrier = Arc::new(Barrier::new(2));
    let barrier_holder = Arc::clone(&barrier);
    let indicator = Arc::new(StdMutex::new(
        Box::new(SilentProgress::new()) as Box<dyn ProgressIndicator>
    ));
    let indicator_holder = Arc::clone(&indicator);

    let holder = thread::spawn(move || {
        barrier_holder.wait();
        let guard =
            CacheWriterLockGuard::acquire_with_feedback(&holder_config, indicator_holder).unwrap();
        thread::sleep(Duration::from_millis(200));
        drop(guard);
    });

    barrier.wait();
    thread::sleep(Duration::from_millis(20));
    let reporter = StatusReporter::new(true);
    let start = Instant::now();
    let contender = CacheWriterLockGuard::acquire_with_status_reporter(&config, &reporter).unwrap();
    let waited = start.elapsed();
    holder.join().unwrap();

    assert_eq!(contender.backend(), LockBackend::Advisory);
    let contender_wait = contender.waited();
    assert!(
        contender_wait >= Duration::from_millis(180),
        "expected the contender to wait at least 180ms, waited {contender_wait:?}"
    );
    assert!(
        waited >= Duration::from_millis(180),
        "expected total wait >= 180ms but was {waited:?}"
    );
}

#[test]
fn cache_writer_respects_timeout_budget() {
    let temp_home = TempDir::new().unwrap();
    let mut config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
    config
        .locking
        .set_timeout_value(LockTimeoutValue::from_secs(0));

    let holder_config = config.clone();
    let barrier = Arc::new(Barrier::new(2));
    let barrier_holder = Arc::clone(&barrier);
    let indicator = Arc::new(StdMutex::new(
        Box::new(SilentProgress::new()) as Box<dyn ProgressIndicator>
    ));
    let indicator_holder = Arc::clone(&indicator);

    let holder = thread::spawn(move || {
        barrier_holder.wait();
        let guard =
            CacheWriterLockGuard::acquire_with_feedback(&holder_config, indicator_holder).unwrap();
        thread::sleep(Duration::from_millis(150));
        drop(guard);
    });

    barrier.wait();
    thread::sleep(Duration::from_millis(20));
    let err = match CacheWriterLockGuard::acquire_with_feedback(&config, indicator) {
        Ok(_) => panic!("second writer should respect zero timeout and fail"),
        Err(err) => err,
    };
    holder.join().unwrap();

    assert!(matches!(err, KopiError::LockingTimeout { .. }));
}

#[test]
fn readers_observe_consistent_cache_during_writes() {
    let temp_home = TempDir::new().unwrap();
    let mut config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
    config
        .locking
        .set_timeout_value(LockTimeoutValue::from_secs(2));

    let cache_path = config.metadata_cache_path().unwrap();
    std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();

    let mut cache = MetadataCache::new();
    cache
        .distributions
        .insert("temurin".to_string(), sample_distribution("bootstrap"));
    cache
        .save(&cache_path, config.locking.timeout_value())
        .unwrap();

    let reader_stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&reader_stop);
    let writer_path = cache_path.clone();
    let writer_config = config.clone();

    let writer = thread::spawn(move || {
        for iteration in 0..100 {
            let reporter = StatusReporter::new(true);
            let guard =
                CacheWriterLockGuard::acquire_with_status_reporter(&writer_config, &reporter)
                    .unwrap();

            let mut refreshed = MetadataCache::new();
            refreshed.last_updated = Utc::now();
            refreshed.distributions.insert(
                "temurin".to_string(),
                sample_distribution(&format!("pkg-{iteration}")),
            );
            refreshed
                .save(&writer_path, writer_config.locking.timeout_value())
                .unwrap();
            drop(guard);
        }
        stop_signal.store(true, Ordering::SeqCst);
    });

    while !reader_stop.load(Ordering::SeqCst) {
        let cache = load_cache(&cache_path).unwrap();
        let dist = cache
            .distributions
            .get("temurin")
            .expect("distribution should remain readable");
        assert!(
            !dist.packages.is_empty(),
            "expected non-empty package list during concurrent writes"
        );
        thread::sleep(Duration::from_millis(5));
    }

    writer.join().unwrap();
}

#[test]
fn fallback_backend_serialises_writers() {
    let temp_home = TempDir::new().unwrap();
    let mut config = KopiConfig::new(temp_home.path().to_path_buf()).unwrap();
    config.locking.mode = LockingMode::Fallback;
    config
        .locking
        .set_timeout_value(LockTimeoutValue::from_secs(2));

    let indicator = Arc::new(StdMutex::new(
        Box::new(SilentProgress::new()) as Box<dyn ProgressIndicator>
    ));
    let guard =
        CacheWriterLockGuard::acquire_with_feedback(&config, Arc::clone(&indicator)).unwrap();
    assert_eq!(guard.backend(), LockBackend::Fallback);

    // While the fallback lock is held, another attempt should time out quickly.
    let mut second_config = config.clone();
    second_config
        .locking
        .set_timeout_value(LockTimeoutValue::from_secs(0));

    let err = match CacheWriterLockGuard::acquire_with_feedback(&second_config, indicator) {
        Ok(_) => panic!("exclusive fallback writer should prevent secondary acquisition"),
        Err(err) => err,
    };
    assert!(matches!(err, KopiError::LockingTimeout { .. }));
    drop(guard);
}

fn foojay_sample_distribution() -> DistributionCache {
    let metadata: DistributionMetadata =
        serde_json::from_str(SAMPLE_FOOJAY_PACKAGE).expect("valid foojay sample");
    let distribution_name = metadata.distribution.name;
    let packages = metadata
        .packages
        .into_iter()
        .map(|package| convert_package_to_jdk_metadata(package).expect("convert sample package"))
        .collect::<Vec<_>>();

    DistributionCache {
        distribution: Distribution::Temurin,
        display_name: distribution_name,
        packages,
    }
}

fn sample_distribution(id_suffix: &str) -> DistributionCache {
    let mut distribution = foojay_sample_distribution();
    if let Some(package) = distribution.packages.first_mut() {
        package.id = format!("temurin-{id_suffix}");
        package.download_url = Some(format!("https://example.com/temurin-{id_suffix}.tar.gz"));
        package.checksum = Some(format!("checksum-{id_suffix}"));
        package.distribution_version = Version::new(21, 0, 1);
    }

    distribution
}
