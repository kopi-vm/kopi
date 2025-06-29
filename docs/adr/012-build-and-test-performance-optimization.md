# ADR-012: Build and Test Performance Optimization

## Status
Proposed

## Context
The Kopi project's build and test execution times have become significantly long, impacting development efficiency. Analysis reveals several bottlenecks:

- Total of 218 tests (126 unit tests, 92 integration tests)
- Unnecessary tokio runtime dependency used only for `#[tokio::main]` without async operations
- Aggressive release profile optimization settings (`lto = "fat"`, `codegen-units = 1`)
- Large test data generation in performance tests (8,100+ test objects per test)
- Duplicate dependencies (e.g., multiple versions of `linux-raw-sys`, `rustix`)
- No optimized profiles for development or test builds

## Decision

### 1. Remove Unnecessary Async Runtime

**Current State**: The project includes tokio with `rt-multi-thread` feature but only uses it for the main function annotation.

**Proposed Change**: Remove tokio dependency entirely and use a standard synchronous main function.

```rust
// Before
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // No async operations
}

// After
fn main() -> anyhow::Result<()> {
    // Same code without async overhead
}
```

**Impact**: Significant reduction in compile time and binary size.

### 2. Optimize Cargo Profiles

**Current State**: Only aggressive release profile defined, no dev/test optimization.

**Proposed Changes**:

```toml
# Keep release profile for distribution
[profile.release]
lto = "fat"
codegen-units = 1

# Add fast release profile for development
[profile.release-fast]
inherits = "release"
lto = false
codegen-units = 16

# Optimize test builds
[profile.test]
opt-level = 1  # Basic optimization
debug = 1      # Limited debug info

# Optimize dev builds with dependencies
[profile.dev.package."*"]
opt-level = 2
```

**Impact**: 
- Development builds: 2-3x faster compilation
- Test execution: 30-50% faster with optimization
- Debug builds: Faster iteration cycles

### 3. Implement Test Optimization Strategies

#### 3.1 Control Test Parallelism
```toml
# In Cargo.toml or as environment variable
[env]
RUST_TEST_THREADS = "4"  # Limit concurrent test threads
```

#### 3.2 Separate Heavy Tests
Create a feature flag for performance tests:
```toml
[features]
perf-tests = []
```

Mark heavy tests:
```rust
#[cfg_attr(not(feature = "perf-tests"), ignore)]
#[test]
fn test_cache_performance_large_dataset() {
    // Heavy test
}
```

#### 3.3 Optimize Test Data Generation
- Use lazy static test data where possible
- Reduce performance test dataset sizes (e.g., from 8,100 to 1,000 items)
- Share test fixtures between tests

### 4. Dependency Optimization

#### 4.1 Consolidate Duplicate Dependencies
Run regular dependency audits:
```bash
cargo tree --duplicates
cargo update
```

#### 4.2 Feature Flag Optimization
Review and minimize feature flags for dependencies:
```toml
# Example: Use only required features
attohttpc = { version = "0.29", default-features = false, features = ["tls-native"] }
```

### 5. Build Caching Strategies

#### 5.1 Use sccache for Distributed Caching
```bash
# Install sccache
cargo install sccache

# Configure as compiler wrapper
export RUSTC_WRAPPER=sccache
```

#### 5.2 CI/CD Optimization
- Cache `~/.cargo/registry`
- Cache `~/.cargo/git`
- Cache `target/` directory between builds
- Use cargo-chef for Docker layer caching

### 6. Incremental Compilation Optimization

Ensure incremental compilation is enabled:
```toml
[build]
incremental = true
```

For CI environments, consider:
```bash
export CARGO_INCREMENTAL=1
```

### 7. Benchmark Suite Implementation

Add a benchmark suite to track performance over time:
```toml
[[bench]]
name = "kopi_bench"
harness = false

[dev-dependencies]
criterion = "0.5"
```

## Consequences

### Positive
- **Build Time Reduction**: Expected 40-60% reduction in development build times
- **Test Execution**: Expected 30-50% faster test runs
- **Developer Experience**: Faster feedback loops and improved productivity
- **CI/CD Efficiency**: Reduced resource usage and faster pipelines
- **Binary Size**: Smaller binaries from removing unnecessary dependencies

### Negative
- **Configuration Complexity**: More Cargo profiles to maintain
- **Test Management**: Additional feature flags for test categorization
- **Initial Setup**: One-time effort to implement all optimizations
- **Monitoring Required**: Need to track build times to ensure optimizations remain effective

### Neutral
- **Code Changes**: Minimal code changes required (mainly removing tokio)
- **Compatibility**: No impact on end-user functionality
- **Maintenance**: Regular dependency audits become necessary

## Implementation Plan

1. **Phase 1**: Remove tokio dependency (immediate, high impact)
2. **Phase 2**: Add optimized Cargo profiles (immediate, medium impact)
3. **Phase 3**: Implement test optimization strategies (1 week)
4. **Phase 4**: Set up build caching and CI optimization (1 week)
5. **Phase 5**: Add benchmark suite for ongoing monitoring (2 weeks)

## Metrics

Track these metrics before and after implementation:
- `cargo build` time (debug)
- `cargo build --release` time
- `cargo test` execution time
- `cargo test --release` execution time
- Binary size
- CI/CD pipeline duration