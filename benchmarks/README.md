# Benchmark Results

This directory contains benchmark results for tracking performance over time.

## Directory Structure

```
benchmarks/
├── baselines/       # Baseline results for comparison (tracked in Git)
│   ├── main.json    # Latest main branch baseline
│   └── v*.json      # Release version baselines
├── results/         # Individual benchmark runs (NOT tracked in Git)
│   └── YYYY-MM-DD/  # Results organized by date
└── README.md        # This file
```

## What's Tracked in Git

- **baselines/** - Important baseline results for comparison
  - Updated when merging to main
  - Saved for each release version
  
- **results/** - Daily results are NOT tracked in Git
  - Available as GitHub Actions artifacts
  - Local development results stay local

## Running Benchmarks

### Basic Usage

```bash
# Run all benchmarks
cargo bench

# Run and save results
./scripts/save-benchmark.sh

# Check for performance regressions
./scripts/check-performance.sh
```

### Comparing with Baselines

```bash
# Compare with main branch baseline
cargo bench -- --baseline main

# Compare with specific version
cargo bench -- --baseline v0.1.0
```

## Benchmark Groups

- **version_parsing**: Version string parsing performance
- **cache_operations**: Cache read/write and serialization
- **search_performance**: JDK search and filtering algorithms

## Performance Targets

| Operation | Target | Description |
|-----------|--------|-------------|
| Simple version parse | < 100 ns | Parsing "21" |
| Complex version parse | < 500 ns | Parsing "21.0.1+12-LTS" |
| Cache search (1000 items) | < 10 µs | Finding version in cache |
| JSON serialize (1000 items) | < 1 ms | Saving cache to disk |

## Interpreting Results

Criterion automatically detects statistically significant changes:
- **Improved**: Performance gain detected
- **Regressed**: Performance loss detected  
- **No change**: Within noise threshold

Results include confidence intervals and outlier detection.