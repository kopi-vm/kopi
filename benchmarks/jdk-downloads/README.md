# JDK Download Performance Measurements

This directory contains performance measurements for JDK downloads and installations, used to determine appropriate lock timeout values for Kopi.

## Purpose

The measurements help answer critical questions:
- How long does it take to download various JDK distributions?
- How do download times vary with network speed?
- What is an appropriate default timeout for lock acquisition?

## Measurement Script

The measurement script is located at: `../../scripts/measure-jdk-download.sh`

### Usage

```bash
# Basic usage - test all common distributions
./scripts/measure-jdk-download.sh

# Test specific distributions
./scripts/measure-jdk-download.sh --distributions "temurin corretto"

# Test different JDK version
./scripts/measure-jdk-download.sh --version 17

# Multiple runs for averaging
./scripts/measure-jdk-download.sh --runs 3

# Custom metadata directory
./scripts/measure-jdk-download.sh --metadata-dir /path/to/metadata
```

### Environment Variables

- `DISTRIBUTIONS`: Space-separated list of distributions to test
- `JDK_VERSION`: Major version number to test (default: 21)  
- `RUNS_PER_JDK`: Number of measurement runs per JDK (default: 1)
- `METADATA_DIR`: Path to metadata JSON files

## Results

Results are stored in `results/` as timestamped JSON files with the following structure:

```json
{
  "timestamp": "2025-09-02T10:30:00Z",
  "host": "hostname",
  "measurements": [
    {
      "distribution": "temurin",
      "version": "21",
      "size_mb": 197,
      "download": {
        "duration_ms": 23500,
        "duration_s": 23.5,
        "speed_mbps": 8.4
      },
      "extraction": {
        "duration_ms": 2500,
        "duration_s": 2.5
      },
      "total_s": 26.0
    }
  ],
  "summary": {
    "avg_download_speed_mbps": 11.7,
    "max_total_time_s": 26.4,
    "min_total_time_s": 20.0
  }
}
```

## Key Findings (2025-09-02)

Based on empirical measurements:

| Distribution | Size | Total Time | Download Speed |
|-------------|------|------------|----------------|
| Temurin 21 | 197MB | 26s | 8.4 MB/s |
| GraalVM 21 | 320MB | 26.4s | 14.4 MB/s |
| Corretto 21 | 199MB | ~17s | 12.1 MB/s |
| Zulu 21 | 207MB | 20s | 12.1 MB/s |

### Network Speed Analysis

Estimated times for GraalVM (320MB - largest common JDK):

| Network Type | Speed | Total Time |
|-------------|-------|------------|
| Fast (100Mbps) | 12.5 MB/s | 31s |
| Typical (50Mbps) | 6.25 MB/s | 56s |
| Mobile 4G (20Mbps) | 2.5 MB/s | 133s |
| Slow (10Mbps) | 1.25 MB/s | 261s |
| Very Slow (5Mbps) | 0.625 MB/s | **517s** |
| Worst (2Mbps) | 0.25 MB/s | **1285s** |

## Timeout Recommendation

Based on the measurements:
- **300s (5 min)**: Insufficient for 5Mbps connections
- **600s (10 min)**: Recommended default - covers down to 5Mbps
- **1200s+ or infinite**: Required for extremely slow connections

The 600-second default ensures most users can complete installations while still detecting hung processes.

## Related Documentation

- [ADR-0001: Concurrent Process Locking Strategy](../../docs/adr/ADR-0001-concurrent-process-locking-strategy.md)
- Performance measurements section in the ADR contains the latest analysis