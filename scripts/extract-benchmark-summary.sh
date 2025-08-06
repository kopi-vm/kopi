#!/usr/bin/env bash
# Copyright 2025 dentsusoken
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
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
    echo "## Performance Comparison"
    echo ""
    echo "To compare with baselines, use: ./scripts/check-performance.sh [baseline-name]"
    echo "HTML report available at: $CRITERION_DIR/report/index.html"
fi
