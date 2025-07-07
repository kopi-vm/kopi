#!/usr/bin/env bash
#
# Extract a human-readable summary from criterion benchmark results

set -euo pipefail

CRITERION_DIR="target/criterion"

if [ ! -d "$CRITERION_DIR" ]; then
    echo "No criterion results found"
    exit 1
fi

echo "Benchmark Summary"
echo "================"
echo ""
echo "Generated: $(date)"
echo ""

# Process each benchmark group
for group_dir in "$CRITERION_DIR"/*; do
    if [ -d "$group_dir" ] && [ -f "$group_dir/base/estimates.json" ]; then
        group_name=$(basename "$group_dir")
        
        # Skip the report directory
        if [ "$group_name" = "report" ]; then
            continue
        fi
        
        echo "## $group_name"
        echo ""
        
        # Extract timing from estimates.json
        if command -v jq >/dev/null 2>&1; then
            # Use jq if available
            mean=$(jq -r '.mean.point_estimate' "$group_dir/base/estimates.json" 2>/dev/null || echo "N/A")
            
            if [ "$mean" != "N/A" ] && [ "$mean" != "null" ]; then
                # Convert nanoseconds to appropriate unit
                mean_ns=$(echo "$mean" | awk '{printf "%.0f", $1}')
                
                if [ "$mean_ns" -lt 1000 ]; then
                    echo "  Time: ${mean_ns} ns"
                elif [ "$mean_ns" -lt 1000000 ]; then
                    mean_us=$(echo "$mean_ns" | awk '{printf "%.2f", $1/1000}')
                    echo "  Time: ${mean_us} µs"
                elif [ "$mean_ns" -lt 1000000000 ]; then
                    mean_ms=$(echo "$mean_ns" | awk '{printf "%.2f", $1/1000000}')
                    echo "  Time: ${mean_ms} ms"
                else
                    mean_s=$(echo "$mean_ns" | awk '{printf "%.2f", $1/1000000000}')
                    echo "  Time: ${mean_s} s"
                fi
            else
                echo "  Time: Unable to parse"
            fi
        else
            # Fallback: try to parse with basic tools
            echo "  Time: See detailed results (jq not available)"
        fi
        
        echo ""
    fi
done

# Performance comparison if baseline exists
if [ -f "$CRITERION_DIR/report/index.html" ]; then
    echo "## Performance Changes"
    echo ""
    echo "Run 'cargo bench -- --baseline <name>' to compare with saved baselines"
    echo "HTML report available at: $CRITERION_DIR/report/index.html"
fi
