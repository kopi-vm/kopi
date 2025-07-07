#!/usr/bin/env bash
#
# Check for performance regressions against baseline

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

# Run benchmarks with comparison
echo "Running benchmarks with baseline comparison..."
cargo bench -- --baseline "$BASELINE"

# Check for regressions in the output
REGRESSION_FOUND=false
IMPROVEMENT_FOUND=false

# Parse criterion output for changes
if [ -d "target/criterion" ]; then
    # Look for performance changes in the criterion output
    # This is a simplified check - in practice, you might want to parse the JSON output
    echo ""
    echo "Performance Summary:"
    echo "==================="
    
    # Extract changes from criterion comparison
    # Note: This would be more robust with proper JSON parsing
    find target/criterion -name "change.json" -type f 2>/dev/null | while read -r change_file; do
        if command -v jq >/dev/null 2>&1; then
            change=$(jq -r '.mean.point_estimate' "$change_file" 2>/dev/null || echo "0")
            if [ "$change" != "0" ] && [ "$change" != "null" ]; then
                change_percent=$(echo "$change" | awk '{printf "%.2f", $1 * 100}')
                bench_name=$(basename "$(dirname "$(dirname "$change_file")")")
                
                if (( $(echo "$change_percent > 5" | bc -l) )); then
                    echo "⚠️  REGRESSION: $bench_name degraded by ${change_percent}%"
                    REGRESSION_FOUND=true
                elif (( $(echo "$change_percent < -5" | bc -l) )); then
                    echo "✅ IMPROVEMENT: $bench_name improved by ${change_percent#-}%"
                    IMPROVEMENT_FOUND=true
                else
                    echo "➖ No significant change in $bench_name (${change_percent}%)"
                fi
            fi
        fi
    done
fi

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
