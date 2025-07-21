# Kopi Env Command Performance Analysis

## Overview

The `kopi env` command is designed for high-performance execution in shell hooks and scripts. This document outlines the performance characteristics and optimization strategies.

## Performance Requirements

The env command must execute in under 100ms for typical use cases to avoid noticeable delays in shell prompts and directory changes.

## Performance Benchmarks

Run benchmarks with:
```bash
cargo bench --bench env_command
```

### Benchmark Scenarios

1. **Global Configuration** (`env_global_config`)
   - Reads global default version from config file
   - Target: < 10ms

2. **Project Version** (`env_project_version`)
   - Searches for `.kopi-version` or `.java-version` files
   - Target: < 20ms for reasonable directory depths

3. **Explicit Version** (`env_explicit_version`)
   - Version specified on command line
   - Target: < 10ms (fastest path)

4. **Deep Directory Hierarchy** (`env_deep_hierarchy`)
   - Version file at root, executed from deep subdirectory
   - Target: < 50ms for 10 levels deep

5. **Shell Formatters** (`env_shell_*`)
   - Different output formats for each shell
   - Target: < 1ms overhead per shell type

6. **Cold Start** (`env_cold_start`)
   - First execution without warm caches
   - Target: < 100ms

## Performance Optimizations

### 1. Minimal Dependencies
- No network operations
- No heavy computation
- Lightweight JSON parsing only for metadata

### 2. Efficient File System Operations
- Stop searching at first `.kopi-version` or `.java-version` found
- Avoid unnecessary stat calls
- Use platform-specific optimizations

### 3. Shell Detection Caching
- Shell detection can be expensive on some systems
- Users can bypass with `--shell` flag
- Consider environment variable for shell hint

### 4. Lazy Loading
- Only load JDK metadata when needed
- Skip validation for `--quiet` mode where possible
- Minimal memory allocations

## Optimization Strategies

### For Users

1. **Use `--shell` flag** to skip shell detection:
   ```bash
   eval "$(kopi env --shell bash)"
   ```

2. **Use `--quiet` flag** to suppress stderr output:
   ```bash
   eval "$(kopi env --quiet)"
   ```

3. **Place version files strategically**:
   - Put `.kopi-version` in project root, not deep subdirectories
   - Avoid deeply nested project structures

4. **Cache in shell configuration**:
   ```bash
   # Detect shell once at startup
   KOPI_SHELL=$(kopi env --shell bash | grep -o 'bash\|zsh\|fish')
   alias kopi-env='kopi env --shell $KOPI_SHELL'
   ```

### For Developers

1. **Profile regularly**:
   ```bash
   # Use cargo flamegraph
   cargo flamegraph --bench env_command

   # Use hyperfine for real-world timing
   hyperfine 'kopi env' 'kopi env --quiet' 'kopi env temurin@21'
   ```

2. **Monitor binary size**:
   ```bash
   # Check release binary size
   cargo build --release
   ls -lh target/release/kopi
   ```

3. **Consider separate binary**:
   - If main binary grows too large
   - `kopi-env` with minimal features
   - Shared code through library crate

## Measurement Tools

### Hyperfine (Real-world performance)
```bash
# Install hyperfine
cargo install hyperfine

# Benchmark common scenarios
hyperfine --warmup 3 \
  'kopi env' \
  'kopi env --quiet' \
  'kopi env --shell bash' \
  'kopi env temurin@21'
```

### Cargo Bench (Microbenchmarks)
```bash
# Run all benchmarks
cargo bench --bench env_command

# Run specific benchmark
cargo bench --bench env_command -- env_global_config

# Save baseline
cargo bench --bench env_command -- --save-baseline main

# Compare changes
cargo bench --bench env_command -- --baseline main
```

### Flamegraph (Profiling)
```bash
# Install flamegraph
cargo install flamegraph

# Profile env command
sudo cargo flamegraph --bench env_command
```

## Performance Targets

| Scenario | Target | Acceptable | Notes |
|----------|--------|------------|-------|
| Simple lookup (global) | < 10ms | < 20ms | Most common case |
| Project file (same dir) | < 15ms | < 30ms | Second most common |
| Project file (5 levels) | < 25ms | < 50ms | Reasonable depth |
| Deep hierarchy (10+) | < 50ms | < 100ms | Edge case |
| Cold start | < 50ms | < 100ms | First run |
| Error cases | < 5ms | < 10ms | Fast fail |

## Monitoring Performance

1. **CI Integration**:
   - Run benchmarks on each PR
   - Fail if regression > 10%
   - Track trends over time

2. **Release Checks**:
   - Profile release builds
   - Compare with previous version
   - Document any changes

3. **User Reports**:
   - Provide debug flag for timing
   - Collect performance data
   - Address bottlenecks

## Future Optimizations

1. **Binary Splitting**:
   - Separate `kopi-env` binary if needed
   - Remove unused features
   - Optimize for size and startup

2. **Compile-Time Optimizations**:
   - Profile-guided optimization (PGO)
   - Link-time optimization (LTO)
   - Custom allocator for small binary

3. **Runtime Caching**:
   - Shell-persistent cache daemon
   - Memory-mapped metadata
   - Inotify/FSEvents for cache invalidation

## Troubleshooting Performance

### Slow Shell Prompt
```bash
# Check timing
time kopi env --quiet

# Use explicit shell
eval "$(kopi env --shell $SHELL --quiet)"

# Debug version resolution
RUST_LOG=debug kopi env 2>&1 | grep "Version resolved"
```

### High CPU Usage
```bash
# Profile the command
sudo perf record -g kopi env
sudo perf report

# Check for file system issues
strace -c kopi env
```

### Memory Usage
```bash
# Check memory allocation
valgrind --tool=massif kopi env
ms_print massif.out.*
```