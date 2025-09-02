# Performance Optimizations

## 1. Direct Process Execution

The most critical optimization is avoiding process chains. Each shim should directly execute the target JDK tool:

```rust
// BAD: Creates process chain (shim → kopi → java)
let status = Command::new("kopi")
    .arg("exec")
    .arg(&tool_name)
    .status()?;

// GOOD: Direct execution with process replacement
#[cfg(unix)]
Command::new(&tool_path)
    .args(env::args().skip(1))
    .exec(); // Process replacement - shim disappears

#[cfg(windows)]
let status = Command::new(&tool_path)
    .args(env::args().skip(1))
    .status()?;
```

## 2. Environment Variable Optimization

Since each shim execution is a separate process, we use environment variables for cross-invocation optimization:

```rust
pub fn resolve_version(current_dir: &Path) -> Result<Version> {
    // 1. Check pre-resolved version (fastest)
    if let Ok(version) = env::var("KOPI_JAVA_VERSION") {
        return Version::parse(&version);
    }
    
    // 2. Do filesystem traversal (slower)
    find_version_in_directory_tree(current_dir)
}
```

Build scripts can optimize multiple invocations:

```bash
# Pre-resolve version for build tools
export KOPI_JAVA_VERSION=$(kopi current --version-only)
javac *.java  # Skips filesystem traversal
java Main     # Skips filesystem traversal
```

## 3. Minimal File System Operations

Since shims are short-lived processes, we optimize for minimal I/O:

```rust
fn resolve_tool_path(tool_name: &str, version: &Version) -> Result<PathBuf> {
    // Direct path construction without unnecessary checks
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine home directory")?;
    
    let tool_path = Path::new(&home)
        .join(".kopi/jdks")
        .join(version.to_string())
        .join("bin")
        .join(tool_name);
    
    #[cfg(windows)]
    let tool_path = tool_path.with_extension("exe");
    
    // Single existence check
    if tool_path.exists() {
        Ok(tool_path)
    } else {
        Err(format!("Tool {} not found for version {}", tool_name, version).into())
    }
}

// Optimize directory traversal
fn find_version_file(start_dir: &Path) -> Result<Version> {
    let mut dir = start_dir;
    
    // Pre-allocate path buffer to avoid repeated allocations
    let mut path_buf = PathBuf::with_capacity(256);
    
    loop {
        // Check both files in one directory access
        path_buf.clear();
        path_buf.push(dir);
        path_buf.push(".kopi-version");
        
        if path_buf.exists() {
            return read_version_file(&path_buf);
        }
        
        path_buf.pop();
        path_buf.push(".java-version");
        
        if path_buf.exists() {
            return read_version_file(&path_buf);
        }
        
        // Move to parent
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    
    Err("No version file found".into())
}
```

## 4. Minimal Dependencies

Keep the shim binary small and fast:
- Avoid heavy dependencies
- Statically link when possible
- Use `no_std` where feasible for core functionality
- Target binary size: < 1MB
- Cold start time: < 10ms

## Performance Targets

| Operation | Target Time | Actual (Typical) |
|-----------|-------------|------------------|
| Tool name detection | < 1ms | 0.1-0.5ms |
| Version resolution (cached) | < 1ms | 0.2-0.5ms |
| Version resolution (file) | < 5ms | 1-3ms |
| Path resolution | < 1ms | 0.1-0.5ms |
| Process exec (Unix) | < 5ms | 1-2ms |
| Process spawn (Windows) | < 20ms | 10-15ms |
| **Total overhead** | **1-20ms** | **1-10ms (Unix), 10-20ms (Windows)** |

## Benchmarking

```rust
// benches/shim_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_version_resolution(c: &mut Criterion) {
    c.bench_function("resolve version", |b| {
        b.iter(|| {
            resolve_version(black_box(Path::new("/test/project")))
        });
    });
}

fn benchmark_shim_overhead(c: &mut Criterion) {
    c.bench_function("shim overhead", |b| {
        b.iter(|| {
            // Measure time to resolve and prepare execution
            let tool_path = resolve_tool_path("java", &Version::new(17, 0, 2));
            black_box(tool_path);
        });
    });
}
```

## Next: [Error Handling](./08-error-handling.md)