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

use kopi::uninstall::progress::ProgressReporter;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_uninstall_progress_reporter_basic() {
    let mut reporter = ProgressReporter::new();

    // Test creating a spinner
    let spinner = reporter.create_spinner("Processing files");
    assert!(!spinner.is_finished());
    spinner.finish();
    assert!(spinner.is_finished());
}

#[test]
fn test_uninstall_progress_bar_with_increments() {
    let mut reporter = ProgressReporter::new();

    // Create a progress bar with total of 10
    let bar = reporter.create_bar(10, "items");
    assert!(!bar.is_finished());

    // Increment progress
    for _ in 0..10 {
        bar.inc(1);
    }

    bar.finish();
    assert!(bar.is_finished());
}

#[test]
fn test_jdk_removal_spinner() {
    let mut reporter = ProgressReporter::new();

    let spinner = reporter.create_jdk_removal_spinner("/opt/java/jdk-21", "1.2 GB");
    assert!(!spinner.is_finished());

    // Simulate some work
    thread::sleep(Duration::from_millis(10));

    spinner.finish_with_message("JDK removed successfully".to_string());
    assert!(spinner.is_finished());
}

#[test]
fn test_batch_removal_progress() {
    let mut reporter = ProgressReporter::new_batch();

    let bar = reporter.create_batch_removal_bar(5);
    assert!(!bar.is_finished());

    // Simulate removing 5 JDKs
    for i in 1..=5 {
        bar.inc(1);
        if i < 5 {
            assert!(!bar.is_finished());
        }
    }

    bar.finish_and_clear();
    assert!(bar.is_finished());
}

#[test]
fn test_concurrent_progress_handles() {
    let mut reporter = ProgressReporter::new();

    // Create multiple progress indicators
    let spinner1 = reporter.create_spinner("Task 1");
    let spinner2 = reporter.create_spinner("Task 2");
    let bar = reporter.create_bar(100, "items");

    // All should be independent
    assert!(!spinner1.is_finished());
    assert!(!spinner2.is_finished());
    assert!(!bar.is_finished());

    // Finish them independently
    spinner1.finish();
    assert!(spinner1.is_finished());
    assert!(!spinner2.is_finished());
    assert!(!bar.is_finished());

    bar.finish();
    assert!(bar.is_finished());
    assert!(!spinner2.is_finished());

    spinner2.finish();
    assert!(spinner2.is_finished());
}

#[test]
fn test_progress_handle_thread_safety() {
    let mut reporter = ProgressReporter::new();
    let bar = Arc::new(reporter.create_bar(100, "items"));

    let bar_clone = Arc::clone(&bar);
    let handle = thread::spawn(move || {
        for _ in 0..50 {
            bar_clone.inc(1);
            thread::sleep(Duration::from_micros(100));
        }
    });

    // Main thread also increments
    for _ in 0..50 {
        bar.inc(1);
        thread::sleep(Duration::from_micros(100));
    }

    handle.join().unwrap();
    bar.finish();
    assert!(bar.is_finished());
}

#[test]
fn test_progress_with_error_message() {
    let mut reporter = ProgressReporter::new();
    let spinner = reporter.create_spinner("Risky operation");

    // Simulate an error occurring
    spinner.finish_with_message("Error: Operation failed".to_string());
    assert!(spinner.is_finished());
}

#[test]
fn test_steady_tick_compatibility() {
    let mut reporter = ProgressReporter::new();
    let spinner = reporter.create_spinner("Long operation");

    // This should not panic (compatibility method)
    spinner.enable_steady_tick(Duration::from_millis(100));

    thread::sleep(Duration::from_millis(50));
    spinner.finish();
    assert!(spinner.is_finished());
}
