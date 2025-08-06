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
# Create a consolidated baseline file from benchmark results

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <results-directory>"
    exit 1
fi

RESULTS_DIR="$1"

if [ ! -d "$RESULTS_DIR" ]; then
    echo "Results directory does not exist: $RESULTS_DIR"
    exit 1
fi

# Create consolidated baseline JSON
cat <<EOF
{
    "metadata": $(cat "$RESULTS_DIR/metadata.json" 2>/dev/null || echo '{}'),
    "benchmarks": {
EOF

first=true
# Find all estimates.json files and consolidate
find "$RESULTS_DIR" -name "estimates.json" -type f | grep -v "/change/" | sort | while read -r estimates_file; do
    # Extract benchmark name from path
    bench_path="${estimates_file#$RESULTS_DIR/}"
    bench_name=$(dirname "$bench_path" | sed -e 's|/base$||' -e 's|/new$||' | tr '/' '.')
    
    if [ "$first" = true ]; then
        first=false
    else
        echo ","
    fi
    
    echo -n "        \"$bench_name\": $(cat "$estimates_file")"
done

cat <<EOF

    }
}
EOF
