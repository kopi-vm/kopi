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

//! Test to verify MultiProgress ordering fix

use kopi::indicator::{ProgressConfig, ProgressFactory, ProgressStyle};
use std::thread;
use std::time::Duration;

#[test]
fn test_multiprogress_add_before_styling() {
    // Create an IndicatifProgress (with MultiProgress)
    let mut parent = ProgressFactory::create(false);

    // Start parent progress
    let config = ProgressConfig::new("Installing", "temurin@21".to_string(), ProgressStyle::Count)
        .with_total(10);
    parent.start(config);

    // Create a child progress
    let mut child = parent.create_child();

    // Start child progress (this will test the fixed ordering)
    let child_config = ProgressConfig::new(
        "Downloading",
        "package.tar.gz".to_string(),
        ProgressStyle::Bytes,
    )
    .with_total(1024 * 1024);
    child.start(child_config);

    // Update both progress bars
    for i in 0..5 {
        parent.update(i * 2, Some(10));
        child.update(i * 200_000, Some(1024 * 1024));
        thread::sleep(Duration::from_millis(10));
    }

    // Complete child first
    child.complete(Some("Download complete".to_string()));

    // Complete parent
    parent.complete(Some("Installation complete".to_string()));
}

#[test]
fn test_multiprogress_spinner_with_child() {
    // Test spinner (no total) with child progress
    let mut parent = ProgressFactory::create(false);

    // Start parent as spinner
    let config = ProgressConfig::new("Processing", "metadata".to_string(), ProgressStyle::Count); // No total = spinner
    parent.start(config);

    parent.set_message("Fetching from API...".to_string());

    // Create child with determinate progress
    let mut child = parent.create_child();
    let child_config =
        ProgressConfig::new("Fetching", "packages".to_string(), ProgressStyle::Count)
            .with_total(100);
    child.start(child_config);

    // Update child progress
    for i in 0..10 {
        child.update(i * 10, Some(100));
        thread::sleep(Duration::from_millis(10));
    }

    child.complete(Some("Fetched 100 packages".to_string()));
    parent.complete(Some("Processing complete".to_string()));
}

#[test]
fn test_multiprogress_println_and_suspend() {
    // Test println and suspend methods with MultiProgress
    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig::new("Testing", "operations".to_string(), ProgressStyle::Count)
        .with_total(5);
    progress.start(config);

    // Test println - should work without corrupting the progress bar
    progress.println("Starting test operations...").unwrap();

    // Test suspend - should temporarily hide progress bar
    progress.suspend(&mut || {
        println!("This is printed during suspend");
    });

    progress.update(3, Some(5));
    progress.println("Operation checkpoint reached").unwrap();

    progress.complete(Some("All operations complete".to_string()));
}
