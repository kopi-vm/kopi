# Shim Binary Size Review (2025-07-06)

This document analyzes the kopi-shim binary size issue where the current implementation exceeds the 1MB target by over 6x, reaching 6.1MB. This review examines the root causes and proposes solutions for size reduction.

## Summary

The kopi-shim binary is significantly larger than its 1MB design target due to the inclusion of the full installation machinery through AutoInstaller. The shim statically links all kopi dependencies including HTTP clients, JSON parsers, archive extractors, and progress UI components, resulting in a 6.1MB binary.

## Design Context

According to ADR documentation, the shim was designed as a statically linked binary (not a minimal subprocess caller) to:

- Achieve 1-20ms overhead target
- Avoid process chain overhead
- Provide direct execution via exec() system call
- Maintain self-contained reliability

## Current Binary Size Analysis

### Measured Size

```bash
$ cargo build --release --bin kopi-shim && ls -lh target/release/kopi-shim
-rwxr-xr-x 2 vscode vscode 6.1M Jul  6 07:25 target/release/kopi-shim
```

### Heavy Dependencies Included

**HTTP/Networking:**

- `attohttpc` - Full HTTP client with JSON, compression, TLS support
- `headers`, `httpdate` - HTTP header/date parsing
- `retry` - Retry logic for HTTP requests

**Archive Handling:**

- `tar` - TAR archive extraction
- `zip` - ZIP archive extraction
- `flate2` - GZIP compression/decompression
- `tempfile` - Temporary file handling

**JSON/Serialization:**

- `serde`, `serde_json` - Full JSON parsing for API responses

**UI/Progress:**

- `indicatif` - Progress bars and spinners
- `colored` - Terminal color output
- `comfy-table` - Table formatting

**Other:**

- `walkdir` - Recursive directory traversal
- `sha2` - SHA256 checksum verification
- `config` - Configuration file parsing
- `toml` - TOML format support

## Root Cause Analysis

### 1. AutoInstaller Integration

The shim includes `AutoInstaller` which directly uses `InstallCommand` (src/shim/mod.rs:64):

```rust
let auto_installer = AutoInstaller::new(&config);
match auto_installer.auto_install(&version_request) {
    Ok(path) => path,
    Err(_e) => { /* error handling */ }
}
```

### 2. No Feature Flag Separation

All dependencies in Cargo.toml are included in both main and shim binaries without conditional compilation.

### 3. Tight Coupling

The auto-installation functionality is tightly coupled with the shim's core version resolution logic.

## Proposed Solutions

### Solution 1: Delegate Auto-Installation to Main Binary (Recommended)

Remove AutoInstaller from shim and call the main kopi binary when auto-installation is needed:

```rust
// Instead of embedded auto-installation
match std::process::Command::new("kopi")
    .arg("install")
    .arg(format!("{}@{}", distribution, version))
    .arg("--auto")
    .status() {
    Ok(status) if status.success() => {
        // Retry finding the JDK
        find_jdk_installation(&repository, &version_request)?
    }
    Err(_) => return Err(KopiError::JdkNotInstalled(...))
}
```

**Pros:**

- Reduces shim size by ~4MB
- Maintains ADR performance goals for normal execution
- Only adds process overhead during auto-installation

**Cons:**

- Violates "no process chain" principle during auto-installation
- Requires locating kopi binary at runtime

### Solution 2: Cargo Workspace Separation

Restructure project into workspaces:

```
kopi/
├── kopi-core/     # Minimal shared types
├── kopi-cli/      # Full CLI with all features
└── kopi-shim/     # Lightweight shim with minimal deps
```

**Dependencies for kopi-shim:**

```toml
[dependencies]
thiserror = "1.0"
dirs = "6.0"
log = "0.4"
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

### Solution 3: Feature Flags

Add feature flags to conditionally compile functionality:

```toml
[features]
default = ["full"]
full = ["install", "api", "progress"]
shim = []  # Minimal features only

install = ["attohttpc", "tar", "zip", "indicatif", "tempfile", "sha2"]
```

Build shim with: `cargo build --release --bin kopi-shim --no-default-features --features shim`

### Solution 4: Compilation Optimizations

Enhanced release profile for shim:

```toml
[profile.release-shim]
inherits = "release"
lto = "fat"
codegen-units = 1
strip = true
opt-level = "z"  # Size optimization
panic = "abort"
```

## Size Reduction Estimates

| Approach                    | Estimated Reduction | Final Size |
| --------------------------- | ------------------- | ---------- |
| Remove AutoInstaller        | 3-4 MB              | ~2-3 MB    |
| + Workspace separation      | 1-2 MB              | ~1-1.5 MB  |
| + Compilation optimizations | 0.5-1 MB            | ~0.5-1 MB  |

## Recommendation

Implement Solution 1 (delegate auto-installation) as the immediate fix, followed by Solution 2 (workspace separation) for long-term maintainability. This approach:

1. Quickly reduces binary size below the 1MB target
2. Maintains performance for the common case (JDK already installed)
3. Accepts process overhead only during auto-installation (rare case)
4. Provides clear separation of concerns

The violation of the "no process chain" principle is acceptable as a pragmatic trade-off, limited to the auto-installation scenario where the performance overhead is overshadowed by the network download time.

## Next Steps

1. Remove AutoInstaller from shim module
2. Implement subprocess call to main kopi binary for auto-installation
3. Measure resulting binary size
4. If still above 1MB, proceed with workspace separation
5. Document the auto-installation exception in ADR
