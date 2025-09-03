# Performance Guide

This guide covers performance optimization and measurement in the Kopi project.

## Build Optimization

### Cargo Build Profiles

Kopi provides optimized build profiles for different use cases:

```bash
# Standard debug build (fastest compilation)
cargo build

# Fast release build (for development)
cargo build --profile release-fast

# Production release build (maximum optimization)
cargo build --release
```

### Incremental Compilation

Incremental compilation is enabled by default in `.cargo/config.toml`, which provides 30-50% faster rebuilds when making small changes.

## Benchmark Suite

Kopi includes a comprehensive benchmark suite using [Criterion.rs](https://github.com/bheisler/criterion.rs) to track performance over time.

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench version_parsing
cargo bench cache_operations
cargo bench search_performance

# Note: This project uses a custom baseline system instead of Criterion's native baseline feature.
# Use the provided scripts for baseline management (see below).
```

### Benchmark Coverage

The benchmark suite measures:

1. **Version Parsing** (`version_parsing`)
   - Simple versions: "21"
   - Complex versions: "21.0.1+12-LTS"
   - Distribution parsing: "temurin@21.0.1"
   - Validation performance

2. **Cache Operations** (`cache_operations`)
   - Version lookup in different cache sizes
   - Package finding with exact criteria
   - JSON serialization/deserialization
   - Metadata conversion

3. **Search Performance** (`search_performance`)
   - Major version search
   - Exact version search
   - Distribution filtering
   - Platform-specific filtering
   - LTS version filtering
   - Auto-selection algorithm

### Interpreting Results

Criterion generates HTML reports in `target/criterion/`:

```bash
# Open the report in your browser (macOS)
open target/criterion/report/index.html

# Linux
xdg-open target/criterion/report/index.html

# Or use Python's HTTP server
cd target/criterion
python3 -m http.server 8000
# Then open http://localhost:8000/report/index.html
```

Example output:

```
version_parsing/simple_version
                        time:   [52.3 ns 52.8 ns 53.4 ns]
                        change: [-2.1% +0.5% +3.2%] (p = 0.71)
                        No change in performance detected.
```

### Performance Targets

Based on current benchmarks, these are the performance targets:

| Operation                   | Target   | Description              |
| --------------------------- | -------- | ------------------------ |
| Simple version parse        | < 100 ns | Parsing "21"             |
| Complex version parse       | < 500 ns | Parsing "21.0.1+12-LTS"  |
| Cache search (1000 items)   | < 10 µs  | Finding version in cache |
| JSON serialize (1000 items) | < 1 ms   | Saving cache to disk     |
| Platform filter             | < 5 µs   | Filtering by OS/arch     |

### Continuous Performance Monitoring

To prevent performance regressions:

1. **Save current benchmarks:**

   ```bash
   ./scripts/save-benchmark.sh
   ```

2. **Check for regressions against baseline:**

   ```bash
   ./scripts/check-performance.sh
   # Or compare with a specific baseline
   ./scripts/check-performance.sh v0.1.0
   ```

3. **View detailed results** in the Criterion HTML report:
   ```bash
   open target/criterion/report/index.html
   ```

### Performance Best Practices

1. **Use benchmarks for optimization decisions**
   - Measure before optimizing
   - Focus on hot paths identified by benchmarks
   - Verify improvements with benchmarks

2. **Regular benchmark runs**
   - Run benchmarks before merging PRs
   - Track performance trends over releases
   - Document significant changes

3. **Profile-guided optimization**
   ```bash
   # Generate flamegraph for a benchmark
   cargo bench --bench kopi_bench -- --profile-time=5
   ```

### Adding New Benchmarks

To add a new benchmark:

1. Choose the appropriate file:
   - `benches/version_parsing.rs` - Version parsing logic
   - `benches/cache_operations.rs` - Cache-related operations
   - `benches/search_performance.rs` - Search algorithms
   - Or create a new file and register it in `benches/kopi_bench.rs`

2. Add your benchmark:

   ```rust
   group.bench_function("operation_name", |b| {
       b.iter(|| {
           // Code to benchmark
           operation(black_box(input))
       })
   });
   ```

3. Use `black_box` to prevent compiler optimizations
4. If creating a new file, add it to `benches/kopi_bench.rs`:
   ```rust
   criterion_group!(
       benches,
       version_parsing::benches,
       cache_operations::benches,
       search_performance::benches,
       your_new_module::benches  // Add your module here
   );
   ```
5. Run and verify the benchmark works correctly

## Test Performance

### Configuration

Tests are configured for optimal performance:

```toml
# .cargo/config.toml
[env]
RUST_TEST_THREADS = "4"  # Limit concurrent tests

# Cargo.toml
[profile.test]
opt-level = 1  # Basic optimization
debug = 1      # Limited debug info
```

### Running Tests Efficiently

```bash
# Run only unit tests (fastest)
cargo test --lib

# Run specific test
cargo test test_name

# Run tests in release mode (faster execution)
cargo test --release

# Run performance tests (usually ignored)
cargo test --features perf-tests
```

## Development Workflow Optimization

### Recommended Build Commands

```bash
# Fast check for errors
cargo check

# Fast debug build
cargo build

# Fast release build
cargo build --profile release-fast

# Production build (slower but optimized)
cargo build --release
```

### IDE Integration

For faster feedback in your IDE:

1. Enable `rust-analyzer` with these settings:

   ```json
   {
     "rust-analyzer.cargo.features": "all",
     "rust-analyzer.checkOnSave.command": "check"
   }
   ```

2. Use `cargo watch` for continuous feedback:
   ```bash
   cargo install cargo-watch
   cargo watch -x check -x test
   ```

## Saving Benchmark Results

To track performance over time, save benchmark results:

```bash
# Run and save benchmarks (also updates main baseline if on main branch)
./scripts/save-benchmark.sh

# Check for performance regressions against main baseline
./scripts/check-performance.sh

# Compare with specific baseline
./scripts/check-performance.sh v0.1.0

# Extract human-readable summary from current results
./scripts/extract-benchmark-summary.sh

# Manually create a baseline from saved results
./scripts/create-baseline.sh benchmarks/results/2025-01-08 > benchmarks/baselines/custom.json
```

### Script Details

- **save-benchmark.sh**: Runs benchmarks, saves results with metadata, and updates baselines
- **check-performance.sh**: Compares current benchmarks against a baseline (default: main)
  - Exits with code 0 if no regressions > 5%
  - Exits with code 1 if regressions are detected
  - Shows improvements, regressions, and unchanged benchmarks
- **extract-benchmark-summary.sh**: Generates a human-readable summary from Criterion results
- **create-baseline.sh**: Creates a consolidated baseline JSON from benchmark results

Benchmark results are stored in:

- `benchmarks/baselines/` - Baseline results (tracked in Git)
  - `main.json` - Baseline for the main branch
  - `vX.Y.Z.json` - Baselines for version tags
- `benchmarks/results/YYYY-MM-DD/` - Daily runs (local only, not in Git)

The custom baseline system:

- Automatically updates `main.json` when running benchmarks on the main branch
- Creates version-specific baselines for tagged releases
- Detects regressions > 5% when comparing with baselines
- Provides detailed performance summaries with improvement/regression indicators
