#!/usr/bin/env bash
#
# Check for performance regressions against baseline
#
set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BENCHMARKS_DIR="$PROJECT_ROOT/benchmarks"
BASELINE="${1:-main}"

echo "Checking performance against baseline: $BASELINE"

# Check if baseline exists
BASELINE_FILE="$BENCHMARKS_DIR/baselines/$BASELINE.json"
if [ ! -f "$BASELINE_FILE" ]; then
    echo "Baseline not found: $BASELINE_FILE"
    echo "Available baselines:"
    ls -1 "$BENCHMARKS_DIR/baselines/" 2>/dev/null || echo "  No baselines found"
    exit 1
fi

# Create temporary directory for current results
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Run benchmarks
echo "Running benchmarks..."
cargo bench

# Save current results using the same logic as create-baseline.sh
echo "Collecting benchmark results..."
CURRENT_RESULTS="$TEMP_DIR/current.json"

# Create metadata for current run
cat > "$TEMP_DIR/metadata.json" << EOF
{
    "date": "$(date +%Y-%m-%d)",
    "time": "$(date +%H-%M-%S)",
    "branch": "$(git branch --show-current 2>/dev/null || echo 'unknown')",
    "commit": "$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')",
    "rust_version": "$(rustc --version | awk '{print $2}')"
}
EOF

# Collect all estimates.json files and create consolidated results
echo "{" > "$CURRENT_RESULTS"
echo '    "metadata":' >> "$CURRENT_RESULTS"
cat "$TEMP_DIR/metadata.json" >> "$CURRENT_RESULTS"
echo ',' >> "$CURRENT_RESULTS"
echo '    "benchmarks": {' >> "$CURRENT_RESULTS"

FIRST=true
find target/criterion -name "estimates.json" -type f | grep -v "/change/" | sort | while read -r estimates_file; do
    # Extract benchmark name from path (e.g., cache_operations/convert_package_to_metadata/base/estimates.json)
    bench_path=$(dirname "$estimates_file")
    
    # Remove target/criterion/ prefix and /base or /new suffix
    bench_name=$(echo "$bench_path" | sed -e 's|target/criterion/||' -e 's|/base$||' -e 's|/new$||' -e 's|/|.|g')
    
    if [ "$FIRST" = false ]; then
        echo ',' >> "$CURRENT_RESULTS"
    fi
    FIRST=false
    
    echo -n "        \"$bench_name\": " >> "$CURRENT_RESULTS"
    cat "$estimates_file" >> "$CURRENT_RESULTS"
done

echo "" >> "$CURRENT_RESULTS"
echo '    }' >> "$CURRENT_RESULTS"
echo '}' >> "$CURRENT_RESULTS"

# Compare with baseline
echo ""
echo "Performance Summary:"
echo "==================="

REGRESSION_FOUND=false
IMPROVEMENT_FOUND=false
THRESHOLD=5  # 5% threshold for significant changes

# Check if jq is available for JSON parsing
if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required for performance comparison. Please install jq."
    exit 1
fi

# Compare each benchmark - filter out .change, .initial, .new entries
jq -r '.benchmarks | keys[]' "$BASELINE_FILE" | grep -v -E '\.(change|initial|new)$' | while read -r bench_name; do
    # Get baseline mean time (in nanoseconds)
    baseline_mean=$(jq -r ".benchmarks[\"$bench_name\"].mean.point_estimate" "$BASELINE_FILE" 2>/dev/null)
    
    # Get current mean time
    current_mean=$(jq -r ".benchmarks[\"$bench_name\"].mean.point_estimate" "$CURRENT_RESULTS" 2>/dev/null)
    
    # Check if both values exist and are valid
    if [ "$baseline_mean" != "null" ] && [ "$current_mean" != "null" ] && [ -n "$baseline_mean" ] && [ -n "$current_mean" ]; then
        # Calculate percentage change
        if command -v bc >/dev/null 2>&1; then
            change_percent=$(echo "scale=2; (($current_mean - $baseline_mean) / $baseline_mean) * 100" | bc -l 2>/dev/null || echo "0")
        else
            # Fallback to awk if bc is not available
            change_percent=$(awk -v curr="$current_mean" -v base="$baseline_mean" 'BEGIN { printf "%.2f", ((curr - base) / base) * 100 }')
        fi
        
        # Format benchmark name for display
        display_name=$(echo "$bench_name" | sed 's/\./ > /g')
        
        # Check if change is significant
        abs_change=$(echo "$change_percent" | sed 's/^-//')
        
        # Use awk for comparison since bc might not be available
        is_significant=$(awk -v val="$abs_change" -v thresh="$THRESHOLD" 'BEGIN { print (val > thresh ? "1" : "0") }')
        
        if [ "$is_significant" = "1" ]; then
            # Check if it's positive (regression) or negative (improvement)
            is_positive=$(awk -v val="$change_percent" 'BEGIN { print (val > 0 ? "1" : "0") }')
            
            if [ "$is_positive" = "1" ]; then
                echo "⚠️  REGRESSION: $display_name degraded by ${change_percent}%"
                echo "REGRESSION_FOUND" > "$TEMP_DIR/regression_found"
            else
                echo "✅ IMPROVEMENT: $display_name improved by ${abs_change}%"
                echo "IMPROVEMENT_FOUND" > "$TEMP_DIR/improvement_found"
            fi
        else
            echo "➖ No significant change in $display_name (${change_percent}%)"
        fi
    else
        # Check if benchmark exists only in current results (new benchmark)
        if [ "$baseline_mean" = "null" ] && [ "$current_mean" != "null" ]; then
            echo "🆕 NEW: $display_name (no baseline available)"
        fi
    fi
done

# Check if regression/improvement flags were set
if [ -f "$TEMP_DIR/regression_found" ]; then
    REGRESSION_FOUND=true
fi
if [ -f "$TEMP_DIR/improvement_found" ]; then
    IMPROVEMENT_FOUND=true
fi

# Check for benchmarks that exist in current but not in baseline
echo ""
jq -r '.benchmarks | keys[]' "$CURRENT_RESULTS" | grep -v -E '\.(change|initial|new)$' | while read -r bench_name; do
    baseline_exists=$(jq -r ".benchmarks[\"$bench_name\"]" "$BASELINE_FILE" 2>/dev/null)
    if [ "$baseline_exists" = "null" ]; then
        display_name=$(echo "$bench_name" | sed 's/\./ > /g')
        echo "🆕 NEW: $display_name (not in baseline)"
    fi
done

echo ""
echo "Full report available at: target/criterion/report/index.html"

# Exit with appropriate code
if [ "$REGRESSION_FOUND" = true ]; then
    echo ""
    echo "❌ Performance regressions detected!"
    exit 1
else
    echo ""
    echo "✅ No significant performance regressions detected"
    exit 0
fi