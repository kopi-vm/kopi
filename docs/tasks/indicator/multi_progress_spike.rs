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

#![allow(dead_code)]

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== MultiProgress Spike Test ===\n");
    
    test_basic_multi_progress();
    println!("\n");
    
    test_nested_progress();
    println!("\n");
    
    test_concurrent_downloads();
    println!("\n");
    
    test_dynamic_progress_bars();
    println!("\n");
    
    test_parent_child_relationship();
    println!("\n");
    
    println!("=== All tests completed ===");
}

fn test_basic_multi_progress() {
    println!("Test 1: Basic MultiProgress with multiple bars");
    
    let multi = MultiProgress::new();
    
    // Create multiple progress bars
    let pb1 = multi.add(ProgressBar::new(100));
    pb1.set_style(
        ProgressStyle::default_bar()
            .template("{prefix} [{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb1.set_prefix("Task 1");
    
    let pb2 = multi.add(ProgressBar::new(200));
    pb2.set_style(
        ProgressStyle::default_bar()
            .template("{prefix} [{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb2.set_prefix("Task 2");
    
    // Simulate progress in a thread
    let pb1_clone = pb1.clone();
    let pb2_clone = pb2.clone();
    let handle = thread::spawn(move || {
        for i in 0..=100 {
            pb1_clone.set_position(i);
            pb2_clone.set_position(i * 2);
            thread::sleep(Duration::from_millis(20));
        }
        pb1_clone.finish_with_message("Complete");
        pb2_clone.finish_with_message("Complete");
    });
    
    handle.join().unwrap();
}

fn test_nested_progress() {
    println!("Test 2: Nested progress bars");
    println!(); // Add blank line for separation
    
    let multi = MultiProgress::new();
    
    // Parent progress bar with spinner at the beginning
    let parent = multi.add(ProgressBar::new(3));
    parent.set_style(
        ProgressStyle::default_bar()
            .template("{spinner} {prefix} [{bar:30}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("██░")
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    parent.set_prefix("Parent Task");
    parent.enable_steady_tick(Duration::from_millis(80));
    
    for i in 1..=3 {
        // Update parent message
        parent.set_message(format!("Processing step {} of 3", i));
        
        // Child progress bar for each step - insert after parent
        let child = multi.insert_after(&parent, ProgressBar::new(50));
        child.set_style(
            ProgressStyle::default_bar()
                .template("  └─ {prefix} [{bar:25}] {pos}/{len}")
                .unwrap()
                .progress_chars("██░"),
        );
        child.set_prefix(format!("Subtask {}", i));
        
        // Update child progress
        for j in 0..=50 {
            child.set_position(j);
            thread::sleep(Duration::from_millis(30));
        }
        
        // Complete child
        child.finish_with_message("Done");
        thread::sleep(Duration::from_millis(300));
        child.finish_and_clear();
        
        // Update parent
        parent.set_position(i);
        parent.set_message(format!("Step {} completed", i));
        thread::sleep(Duration::from_millis(300));
    }
    
    parent.finish_with_message("All steps complete");
    thread::sleep(Duration::from_millis(500)); // Keep final state visible
}

fn test_concurrent_downloads() {
    println!("Test 3: Simulating concurrent downloads");
    
    let multi = Arc::new(MultiProgress::new());
    let mut handles = vec![];
    
    let files = vec![
        ("temurin-21.tar.gz", 1024 * 1024 * 100), // 100MB
        ("corretto-17.tar.gz", 1024 * 1024 * 80), // 80MB
        ("zulu-11.tar.gz", 1024 * 1024 * 120),    // 120MB
    ];
    
    for (filename, size) in files {
        let multi_clone = multi.clone();
        
        let handle = thread::spawn(move || {
            let pb = multi_clone.add(ProgressBar::new(size));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{prefix} [{bar:30}] {bytes}/{total_bytes} {bytes_per_sec}")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.set_prefix(filename);
            
            // Simulate download with variable speeds
            let chunk_size = 1024 * 512; // 512KB chunks
            let mut downloaded = 0u64;
            
            while downloaded < size {
                let chunk = chunk_size.min(size - downloaded);
                downloaded += chunk;
                pb.set_position(downloaded);
                
                // Simulate variable network speed
                thread::sleep(Duration::from_millis(10 + (rand() % 20)));
            }
            
            pb.finish_with_message("Downloaded");
        });
        
        handles.push(handle);
    }
    
    // Wait for all downloads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

fn test_dynamic_progress_bars() {
    println!("Test 4: Dynamically adding/removing progress bars");
    
    let multi = MultiProgress::new();
    let mut bars = vec![];
    
    // Spinner for discovering tasks
    let discover = multi.add(ProgressBar::new_spinner());
    discover.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {prefix} {msg}")
            .unwrap()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    discover.set_prefix("Discovering JDKs");
    discover.enable_steady_tick(Duration::from_millis(100));
    
    thread::sleep(Duration::from_secs(2));
    
    // Simulate discovering tasks
    let jdks = vec!["temurin@21", "corretto@17", "zulu@11", "liberica@8"];
    
    for jdk in jdks {
        discover.set_message(format!("Found {}", jdk));
        thread::sleep(Duration::from_millis(500));
        
        // Add a new progress bar for each discovered JDK
        let pb = multi.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{bar:25}] {pos}%")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_prefix(format!("Installing {}", jdk));
        bars.push(pb);
    }
    
    discover.finish_and_clear();
    
    // Process all bars
    for pb in bars {
        for i in 0..=100 {
            pb.set_position(i);
            thread::sleep(Duration::from_millis(10));
        }
        pb.finish_with_message("✓");
    }
}

fn test_parent_child_relationship() {
    println!("Test 5: Parent-child relationship");
    println!(); // Add blank line for separation
    
    let multi = MultiProgress::new();
    
    // Main progress bar with spinner at the beginning
    let main_pb = multi.add(ProgressBar::new(100));
    main_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner} {prefix} [{bar:40}] {pos}/{len}")
            .unwrap()
            .progress_chars("██░")
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    main_pb.set_prefix("Main Operation");
    main_pb.enable_steady_tick(Duration::from_millis(80));
    
    for i in 0..=100 {
        main_pb.set_position(i);
        
        // Add sub-operations at specific points
        if i == 25 || i == 50 || i == 75 {
            // Add a sub-operation below main bar
            let sub_pb = multi.insert_after(&main_pb, ProgressBar::new_spinner());
            sub_pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  └─ {spinner} {msg}")
                    .unwrap()
                    .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
            );
            sub_pb.set_message(format!("Processing checkpoint at {}%", i));
            sub_pb.enable_steady_tick(Duration::from_millis(100));
            
            // Keep spinner visible
            thread::sleep(Duration::from_secs(2));
            
            sub_pb.finish_with_message(format!("Checkpoint {}% completed", i));
            thread::sleep(Duration::from_millis(500));
            sub_pb.finish_and_clear();
        }
        
        thread::sleep(Duration::from_millis(30));
    }
    
    main_pb.finish_with_message("Complete");
    thread::sleep(Duration::from_millis(500));
}

// Simple random number generator for simulation
fn rand() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}