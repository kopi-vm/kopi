#!/usr/bin/env bash
#
# Save benchmark results for tracking performance over time
# This script runs benchmarks and archives the results

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BENCHMARKS_DIR="$PROJECT_ROOT/benchmarks"
DATE=$(date +%Y-%m-%d)
TIME=$(date +%H-%M-%S)
BRANCH=$(git rev-parse --abbrev-ref HEAD)
COMMIT=$(git rev-parse --short HEAD)

echo "Running benchmarks for branch: $BRANCH, commit: $COMMIT"

# Create results directory
RESULTS_DIR="$BENCHMARKS_DIR/results/$DATE"
mkdir -p "$RESULTS_DIR"

# Run benchmarks
echo "Running cargo bench..."
if cargo bench; then
    echo "Benchmarks completed successfully"
else
    echo "Benchmark execution failed"
    exit 1
fi

# Save criterion results
if [ -d "target/criterion" ]; then
    echo "Saving benchmark results to $RESULTS_DIR"
    
    # Create metadata file
    cat > "$RESULTS_DIR/metadata.json" <<EOF
{
    "date": "$DATE",
    "time": "$TIME",
    "branch": "$BRANCH",
    "commit": "$COMMIT",
    "rust_version": "$(rustc --version | awk '{print $2}')"
}
EOF
    
    # Copy benchmark data (excluding large HTML files)
    find target/criterion -name "*.json" -o -name "*.csv" | while read -r file; do
        # Create relative directory structure
        rel_path="${file#target/criterion/}"
        dest_dir="$RESULTS_DIR/$(dirname "$rel_path")"
        mkdir -p "$dest_dir"
        cp "$file" "$dest_dir/"
    done
    
    # Extract summary for easy viewing
    echo "Extracting benchmark summary..."
    "$SCRIPT_DIR/extract-benchmark-summary.sh" > "$RESULTS_DIR/summary.txt"
    
    # Update baseline if on main branch
    if [ "$BRANCH" = "main" ]; then
        echo "Updating main branch baseline..."
        # Create consolidated baseline file
        "$SCRIPT_DIR/create-baseline.sh" "$RESULTS_DIR" > "$BENCHMARKS_DIR/baselines/main.json"
        echo "Main baseline updated"
    fi
    
    # For version tags, save as version baseline
    if [[ "$BRANCH" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Saving version baseline for $BRANCH..."
        "$SCRIPT_DIR/create-baseline.sh" "$RESULTS_DIR" > "$BENCHMARKS_DIR/baselines/$BRANCH.json"
        echo "Version baseline saved"
    fi
    
    echo "Results saved to: $RESULTS_DIR"
    echo "Summary:"
    head -20 "$RESULTS_DIR/summary.txt"
else
    echo "No criterion results found in target/criterion"
    exit 1
fi