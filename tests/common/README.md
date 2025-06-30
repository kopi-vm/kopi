# TestHomeGuard - Test Helper for Kopi Integration Tests

## Overview
`TestHomeGuard` is a test utility that creates isolated KOPI_HOME directories for integration tests. It creates directories under `target/home/<random-8-chars>/` and automatically cleans them up when the test completes.

## Usage

### Basic Usage
```rust
mod common;
use common::TestHomeGuard;

#[test]
fn test_with_isolated_kopi_home() {
    // Create test home directory
    let test_home = TestHomeGuard::new();
    let test_home = test_home.setup_kopi_structure();
    
    // Get path to KOPI_HOME
    let kopi_home = test_home.kopi_home();
    
    // Use with commands
    let mut cmd = Command::cargo_bin("kopi").unwrap();
    cmd.env("KOPI_HOME", &kopi_home)
        .arg("list");
    
    cmd.assert().success();
    
    // Directory is automatically cleaned up when test_home is dropped
}
```

### Directory Structure
When you call `setup_kopi_structure()`, it creates:
```
target/home/<random-8-chars>/
└── .kopi/
    ├── jdks/
    ├── cache/
    └── bin/
```

### Key Features
1. **Isolation**: Each test gets its own unique directory
2. **Cleanup**: Directories are automatically removed when the guard is dropped
3. **Predictable Location**: All test directories are under `target/home/`
4. **Random Names**: Uses 8-character alphanumeric names to avoid conflicts

### Migration from TempDir
```rust
// OLD: Using TempDir
let temp_dir = TempDir::new().unwrap();
let kopi_home = temp_dir.path().join(".kopi");
fs::create_dir_all(&kopi_home).unwrap();

// NEW: Using TestHomeGuard
let test_home = TestHomeGuard::new();
let test_home = test_home.setup_kopi_structure();
let kopi_home = test_home.kopi_home();
```

### Important Notes
- The guard must be kept alive for the duration of the test
- Use two-step initialization to avoid lifetime issues:
  ```rust
  let test_home = TestHomeGuard::new();
  let test_home = test_home.setup_kopi_structure();
  ```
- The directory path is relative to the workspace root

## Benefits
- Consistent test environment setup
- No system temp directory pollution
- Easy to debug (check `target/home/` if cleanup fails)
- Thread-safe (each test gets unique directory)