# Implementation Details

## Core Shim Binary

The shim binary is a lightweight Rust program that:

1. Determines which tool was invoked (via argv[0])
2. Finds the appropriate JDK version for the current context
3. Locates the actual tool binary
4. Executes it with process replacement (Unix) or direct execution (Windows)

## Common Implementation Pattern

```rust
// src/bin/kopi-shim.rs
use std::env;
use std::process::Command;
use kopi_core::{version_resolver, jdk_locator, jdk_installer};

fn main() {
    // Get tool name from argv[0]
    let tool_name = env::current_exe()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            eprintln!("kopi: failed to determine tool name");
            std::process::exit(1);
        });

    // Resolve required version
    let version = match resolve_version(&env::current_dir().unwrap()) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("kopi: No Java version specified. Create a .kopi-version file");
            std::process::exit(1);
        }
    };

    // Check if JDK is installed, install if missing
    let jdk_path = ensure_jdk_installed(&version).unwrap_or_else(|e| {
        eprintln!("kopi: {}", e);
        std::process::exit(1);
    });

    // Resolve tool path
    let tool_path = jdk_path.join("bin").join(&tool_name);

    #[cfg(windows)]
    let tool_path = tool_path.with_extension("exe");

    if !tool_path.exists() {
        eprintln!("kopi: Tool '{}' not found in Java {}", tool_name, version);
        std::process::exit(1);
    }

    // Platform-specific execution
    #[cfg(unix)]
    execute_unix(&tool_path);

    #[cfg(windows)]
    execute_windows(&tool_path);
}

fn ensure_jdk_installed(version: &Version) -> Result<PathBuf> {
    let jdk_path = get_jdk_path(version);

    if !jdk_path.exists() {
        // Check if auto-install is enabled
        let config = KopiConfig::load()?;
        if !config.shims.auto_install {
            bail!("Java {} is not installed. Run 'kopi install {}'", version, version);
        }

        // Check if we should prompt
        if config.shims.auto_install_prompt && !should_install_jdk(version)? {
            bail!("Java {} is not installed. Installation cancelled by user.", version);
        }

        // Install JDK
        install_jdk_from_shim(version)?;
    }

    Ok(jdk_path)
}
```

## Platform-Specific Execution Functions

The actual execution differs between platforms:

- **Unix**: Uses `exec()` to replace the process (see [Unix Implementation](./04-unix-implementation.md))
- **Windows**: Spawns a new process and waits (see [Windows Implementation](./05-windows-implementation.md))

## Build Process Overview

1. Compile the `kopi-shim` binary
2. Create platform-specific shims:
   - **Unix**: Symlinks to the single binary
   - **Windows**: Copies of the binary with different names

## Next: [Unix Implementation](./04-unix-implementation.md)
