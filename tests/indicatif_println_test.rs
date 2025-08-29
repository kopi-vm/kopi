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

//! Test to verify IndicatifProgress println uses owned_bar's println

use kopi::indicator::{ProgressConfig, ProgressFactory, ProgressStyle};

#[test]
fn test_println_with_active_progress() {
    // Create an IndicatifProgress
    let mut progress = ProgressFactory::create(false);

    // Start progress bar
    let config =
        ProgressConfig::new("Processing", "data".to_string(), ProgressStyle::Count).with_total(10);
    progress.start(config);

    // Test println while progress bar is active
    // This should use the owned_bar's println
    progress.println("Message 1: Starting processing").unwrap();

    progress.update(5, Some(10));
    progress.println("Message 2: Halfway done").unwrap();

    progress.update(10, Some(10));
    progress.println("Message 3: Almost complete").unwrap();

    progress.complete(Some("Processing complete".to_string()));

    // After completion, owned_bar is removed
    // println should fall back to regular println
    progress.println("Message 4: After completion").unwrap();
}

#[test]
fn test_println_without_active_progress() {
    // Create an IndicatifProgress but don't start it
    let progress = ProgressFactory::create(false);

    // Test println without an active progress bar
    // This should fall back to regular println
    progress.println("No progress bar active").unwrap();
}

#[test]
fn test_println_with_child_progress() {
    // Create parent progress
    let mut parent = ProgressFactory::create(false);

    let config =
        ProgressConfig::new("Parent", "task".to_string(), ProgressStyle::Count).with_total(10);
    parent.start(config);

    // Create child progress
    let mut child = parent.create_child();
    let child_config =
        ProgressConfig::new("Child", "subtask".to_string(), ProgressStyle::Count).with_total(5);
    child.start(child_config);

    // Both parent and child should be able to use println
    parent.println("Parent message").unwrap();
    child.println("Child message").unwrap();

    // Update and complete
    child.update(5, Some(5));
    child.complete(None);

    parent.update(10, Some(10));
    parent.complete(None);
}
