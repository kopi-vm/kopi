use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kopi::indicator::factory::ProgressFactory;
use kopi::indicator::status::StatusReporter;
use kopi::indicator::{ProgressConfig, ProgressStyle};
use serial_test::serial;

mod common;
use common::test_home::TestHomeGuard;

#[test]
fn test_progress_factory_terminal_detection() {
    // Test that factory correctly detects terminal/non-terminal environments
    // With no_progress flag - should return silent progress
    let mut progress = ProgressFactory::create(true);

    // Silent progress should handle all operations without panicking
    let config = ProgressConfig {
        operation: "Test".to_string(),
        context: "Silent mode".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };
    progress.start(config);
    progress.update(50, None);
    progress.complete(None);

    // Without no_progress flag - behavior depends on terminal detection
    let mut progress = ProgressFactory::create(false);
    let config = ProgressConfig {
        operation: "Test".to_string(),
        context: "Normal mode".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };
    progress.start(config);
    progress.update(50, None);
    progress.complete(None);
}

#[test]
fn test_progress_indicator_with_install_simulation() {
    let test_home = TestHomeGuard::new();

    // Simulate install operation with progress
    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Installing JDK".to_string(),
        context: "temurin@21".to_string(),
        total: Some(150_000_000), // 150MB
        style: ProgressStyle::Bytes,
    };

    progress.start(config);

    // Simulate download progress
    for i in 0..10 {
        progress.update(i as u64 * 15_000_000, None);
        thread::sleep(Duration::from_millis(10));
    }

    progress.complete(Some("Installation complete".to_string()));

    // Verify no panic and progress completes
    assert!(test_home.path().exists());
}

#[test]
fn test_progress_indicator_with_cache_operations() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Refreshing cache".to_string(),
        context: "Fetching metadata".to_string(),
        total: None, // Spinner mode
        style: ProgressStyle::Count,
    };

    progress.start(config);

    // Simulate cache refresh with message updates
    for i in 0..5 {
        progress.set_message(format!("Processing item {}/5", i + 1));
        thread::sleep(Duration::from_millis(10));
    }

    progress.set_message("Processing distributions...".to_string());
    thread::sleep(Duration::from_millis(10));

    progress.complete(None);
}

#[test]
fn test_progress_indicator_batch_operations() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Uninstalling JDKs".to_string(),
        context: "Batch operation".to_string(),
        total: Some(5), // 5 JDKs to uninstall
        style: ProgressStyle::Count,
    };

    progress.start(config);

    // Simulate batch uninstall
    for i in 0..5 {
        progress.set_message(format!("Removing JDK {}/5", i + 1));
        progress.update(i as u64 + 1, None);
        thread::sleep(Duration::from_millis(10));
    }

    progress.complete(Some("All JDKs uninstalled".to_string()));
}

#[test]
fn test_no_progress_mode_across_commands() {
    let _test_home = TestHomeGuard::new();

    // Test with no_progress = true (silent mode)
    let mut progress = ProgressFactory::create(true);

    // All operations should be silent
    let config = ProgressConfig {
        operation: "Test Operation".to_string(),
        context: "Should be silent".to_string(),
        total: Some(100),
        style: ProgressStyle::Bytes,
    };

    progress.start(config);
    progress.update(50, None);
    progress.set_message("This should not be visible".to_string());
    progress.complete(None);

    // Silent progress handles all operations without output
}

#[test]
#[serial]
fn test_progress_in_ci_environment() {
    // Save original CI env var
    let original_ci = env::var("CI").ok();

    // Test in CI environment
    unsafe {
        env::set_var("CI", "true");
    }

    let mut progress = ProgressFactory::create(false);

    // In CI, should use simple progress (not indicatif with fancy bars)
    let config = ProgressConfig {
        operation: "CI Operation".to_string(),
        context: "Running in CI".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };

    progress.start(config);
    progress.update(100, None);
    progress.complete(None);

    // Restore original CI env var
    unsafe {
        match original_ci {
            Some(val) => env::set_var("CI", val),
            None => env::remove_var("CI"),
        }
    }
}

#[test]
#[serial]
fn test_progress_with_dumb_terminal() {
    // Save original TERM env var
    let original_term = env::var("TERM").ok();

    // Test with TERM=dumb
    unsafe {
        env::set_var("TERM", "dumb");
    }

    let mut progress = ProgressFactory::create(false);

    // Should use simple progress for dumb terminals
    let config = ProgressConfig {
        operation: "Dumb Terminal Operation".to_string(),
        context: "TERM=dumb".to_string(),
        total: Some(50),
        style: ProgressStyle::Bytes,
    };

    progress.start(config);
    progress.update(25, None);
    progress.complete(None);

    // Restore original TERM env var
    unsafe {
        match original_term {
            Some(val) => env::set_var("TERM", val),
            None => env::remove_var("TERM"),
        }
    }
}

#[test]
fn test_progress_indicator_error_handling() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Operation with error".to_string(),
        context: "Testing error".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };

    progress.start(config);
    progress.update(50, None);

    // Simulate error
    progress.error("Simulated error occurred".to_string());

    // Progress should handle error gracefully
    progress.complete(None);
}

#[test]
fn test_progress_indicator_concurrent_operations() {
    let _test_home = TestHomeGuard::new();

    let finished = Arc::new(AtomicBool::new(false));

    let mut handles = vec![];

    // Spawn multiple threads with progress indicators
    for i in 0..3 {
        let finished = Arc::clone(&finished);

        let handle = thread::spawn(move || {
            let mut progress = ProgressFactory::create(false);

            let config = ProgressConfig {
                operation: format!("Thread {i} operation"),
                context: format!("Concurrent test {i}"),
                total: Some(50),
                style: ProgressStyle::Count,
            };

            progress.start(config);

            for j in 0..50 {
                if finished.load(Ordering::Relaxed) {
                    break;
                }
                progress.update(j, None);
                thread::sleep(Duration::from_millis(5));
            }

            progress.complete(None);
        });

        handles.push(handle);
    }

    // Let threads run for a bit
    thread::sleep(Duration::from_millis(100));
    finished.store(true, Ordering::Relaxed);

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete");
    }
}

#[test]
fn test_status_reporter_silent_mode() {
    let _test_home = TestHomeGuard::new();

    // Test with silent mode
    let reporter = StatusReporter::new(true);

    // These should not output anything
    reporter.operation("Silent operation", "test context");
    reporter.step("Silent step");
    reporter.success("Silent success");

    // Error should still be shown even in silent mode
    reporter.error("This error should be visible");
}

#[test]
fn test_status_reporter_normal_mode() {
    let _test_home = TestHomeGuard::new();

    // Test without silent mode
    let reporter = StatusReporter::new(false);

    reporter.operation("Starting operation", "test context");
    reporter.step("Step 1: Preparing");
    reporter.step("Step 2: Processing");
    reporter.success("Operation completed successfully");
    reporter.error("Example error message");
}

#[test]
fn test_progress_styles() {
    let _test_home = TestHomeGuard::new();

    // Test Bytes style
    {
        let mut progress = ProgressFactory::create(false);
        let config = ProgressConfig {
            operation: "Download".to_string(),
            context: "file.tar.gz".to_string(),
            total: Some(1_000_000),
            style: ProgressStyle::Bytes,
        };

        progress.start(config);
        progress.update(500_000, None);
        progress.complete(None);
    }

    // Test Count style
    {
        let mut progress = ProgressFactory::create(false);
        let config = ProgressConfig {
            operation: "Processing".to_string(),
            context: "items".to_string(),
            total: Some(100),
            style: ProgressStyle::Count,
        };

        progress.start(config);
        progress.update(50, None);
        progress.complete(None);
    }
}

#[test]
fn test_progress_with_message_updates() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Multi-step operation".to_string(),
        context: "Testing messages".to_string(),
        total: None,
        style: ProgressStyle::Count,
    };

    progress.start(config);

    let messages = vec![
        "Initializing...",
        "Connecting to server...",
        "Downloading metadata...",
        "Processing data...",
        "Finalizing...",
    ];

    for msg in messages {
        progress.set_message(msg.to_string());
        thread::sleep(Duration::from_millis(10));
    }

    progress.complete(None);
}

#[test]
fn test_progress_indicator_memory_usage() {
    let _test_home = TestHomeGuard::new();

    // Create and destroy many progress indicators to check for leaks
    for _ in 0..100 {
        let mut progress = ProgressFactory::create(false);

        let config = ProgressConfig {
            operation: "Memory test".to_string(),
            context: "Checking for leaks".to_string(),
            total: Some(10),
            style: ProgressStyle::Count,
        };

        progress.start(config);
        progress.update(5, None);
        progress.complete(None);
    }

    // If we get here without issues, memory management is working
}

#[test]
fn test_progress_indicator_performance() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(true); // Use silent mode for consistent timing

    let config = ProgressConfig {
        operation: "Performance test".to_string(),
        context: "Measuring overhead".to_string(),
        total: Some(1_000_000),
        style: ProgressStyle::Count,
    };

    let start = Instant::now();

    progress.start(config);

    // Perform many updates
    for i in 0..1_000_000 {
        if i % 10_000 == 0 {
            progress.update(i, None);
        }
    }

    progress.complete(None);

    let elapsed = start.elapsed();

    // Verify minimal overhead (should complete in less than 1 second)
    assert!(
        elapsed < Duration::from_secs(1),
        "Progress indicator overhead too high: {elapsed:?}"
    );
}

#[test]
fn test_progress_with_long_operations() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    // Simulate a long-running operation
    let config = ProgressConfig {
        operation: "Long operation".to_string(),
        context: "Processing large dataset".to_string(),
        total: Some(1_000_000_000), // 1GB
        style: ProgressStyle::Bytes,
    };

    progress.start(config);

    // Simulate progress in chunks
    let chunk_size = 100_000_000; // 100MB chunks
    for i in 0..10 {
        progress.update(i * chunk_size, None);
        thread::sleep(Duration::from_millis(5));
    }

    progress.complete(None);
}

#[test]
fn test_progress_indicator_state_transitions() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    // Test state transitions: not started -> started -> completed
    let config = ProgressConfig {
        operation: "State test".to_string(),
        context: "Testing transitions".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };

    // Start
    progress.start(config.clone());

    // Update multiple times
    progress.update(25, None);
    progress.update(50, None);
    progress.update(75, None);

    // Complete
    progress.complete(None);

    // Starting again should work
    progress.start(config);
    progress.update(100, None);
    progress.complete(None);
}

#[test]
fn test_progress_indicator_zero_total() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    // Test with zero total (should handle gracefully)
    let config = ProgressConfig {
        operation: "Zero total test".to_string(),
        context: "Edge case".to_string(),
        total: Some(0),
        style: ProgressStyle::Count,
    };

    progress.start(config);
    progress.update(0, None);
    progress.complete(None);
}

#[test]
fn test_progress_indicator_overflow_protection() {
    let _test_home = TestHomeGuard::new();

    let mut progress = ProgressFactory::create(false);

    let config = ProgressConfig {
        operation: "Overflow test".to_string(),
        context: "Testing bounds".to_string(),
        total: Some(100),
        style: ProgressStyle::Count,
    };

    progress.start(config);

    // Try to update beyond total (should handle gracefully)
    progress.update(150, None);
    progress.update(200, None);

    progress.complete(None);
}
