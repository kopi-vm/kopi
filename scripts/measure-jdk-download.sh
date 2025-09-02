#!/bin/bash
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

set -euo pipefail

# Simple JDK Download Time Measurement Script
# This is a simplified version that uses the same approach as the actual testing

# Show help if requested
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    cat << EOF
Usage: $0 [DISTRIBUTIONS] [VERSION]

Measure JDK download and installation times for various distributions.

Arguments:
    DISTRIBUTIONS    Space-separated list of distributions to test
                    (default: "temurin")
                    Available: temurin, corretto, zulu, graalvm, etc.
    VERSION         JDK major version to test (default: 21)

Environment variables:
    METADATA_DIR    Directory containing JDK metadata JSON files
    OUTPUT_DIR      Directory to save results

Examples:
    # Test Temurin JDK 21 (default)
    $0
    
    # Test multiple distributions
    $0 "temurin corretto zulu"
    
    # Test specific version
    $0 temurin 17
    
    # Test all common distributions
    $0 "temurin corretto zulu graalvm" 21

Results are saved as timestamped JSON files in:
    benchmarks/jdk-downloads/results/

EOF
    exit 0
fi

METADATA_DIR="${METADATA_DIR:-/workspaces/kopi-workspace/metadata/docs/linux-x64-glibc}"
OUTPUT_DIR="${OUTPUT_DIR:-/workspaces/kopi-workspace/first/benchmarks/jdk-downloads/results}"
TEMP_DIR="/tmp/jdk-download-test-$$"

# Default test configurations
DISTRIBUTIONS="${1:-temurin}"
JDK_VERSION="${2:-21}"

# Cleanup on exit
trap "rm -rf $TEMP_DIR" EXIT

# Create directories
mkdir -p "$OUTPUT_DIR"
mkdir -p "$TEMP_DIR"

echo "=== JDK Download Time Measurement ==="
echo "Testing: $DISTRIBUTIONS"
echo "Version: $JDK_VERSION"
echo ""

# Results file
RESULT_FILE="$OUTPUT_DIR/$(date +%Y-%m-%d_%H-%M-%S).json"

# Start JSON output
echo "{" > "$RESULT_FILE"
echo '  "timestamp": "'$(date -Iseconds)'",' >> "$RESULT_FILE"
echo '  "measurements": [' >> "$RESULT_FILE"

FIRST=true

for dist in $DISTRIBUTIONS; do
    echo "Testing $dist JDK $JDK_VERSION..."
    
    # Get JDK info from metadata
    METADATA_FILE="$METADATA_DIR/${dist}.json"
    if [[ ! -f "$METADATA_FILE" ]]; then
        echo "  Metadata file not found: $METADATA_FILE"
        continue
    fi
    
    # Extract URL and size
    JDK_INFO=$(cat "$METADATA_FILE" | jq -r --arg ver "$JDK_VERSION" '
        .[] | 
        select(.package_type == "jdk" and .version.components[0] == ($ver | tonumber)) |
        {url: .download_url, size: .size} | @json
    ' | head -1)
    
    if [[ -z "$JDK_INFO" ]]; then
        echo "  No JDK found for $dist version $JDK_VERSION"
        continue
    fi
    
    URL=$(echo "$JDK_INFO" | jq -r '.url')
    SIZE=$(echo "$JDK_INFO" | jq -r '.size')
    SIZE_MB=$(python3 -c "print(f'{$SIZE / 1048576:.2f}')")
    
    echo "  URL: $URL"
    echo "  Size: ${SIZE_MB}MB"
    
    # Download and measure
    cd "$TEMP_DIR"
    ARCHIVE_NAME="${dist}-${JDK_VERSION}.tar.gz"
    
    echo "  Downloading..."
    START_TIME=$(date +%s%N)
    if time wget -q "$URL" -O "$ARCHIVE_NAME"; then
        END_TIME=$(date +%s%N)
        DOWNLOAD_MS=$(( (END_TIME - START_TIME) / 1000000 ))
        DOWNLOAD_S=$(python3 -c "print(f'{$DOWNLOAD_MS / 1000:.2f}')")
        SPEED_MBPS=$(python3 -c "print(f'{($SIZE / 1048576) / ($DOWNLOAD_MS / 1000):.2f}' if $DOWNLOAD_MS > 0 else '0')")
        
        echo "  Download time: ${DOWNLOAD_S}s (${SPEED_MBPS} MB/s)"
        
        # Extract and measure
        echo "  Extracting..."
        START_TIME=$(date +%s%N)
        if [[ "$ARCHIVE_NAME" == *.zip ]]; then
            time unzip -q "$ARCHIVE_NAME"
        else
            time tar -xzf "$ARCHIVE_NAME"
        fi
        END_TIME=$(date +%s%N)
        EXTRACT_MS=$(( (END_TIME - START_TIME) / 1000000 ))
        EXTRACT_S=$(python3 -c "print(f'{$EXTRACT_MS / 1000:.2f}')")
        
        echo "  Extract time: ${EXTRACT_S}s"
        
        TOTAL_S=$(python3 -c "print(f'{$DOWNLOAD_S + $EXTRACT_S:.2f}')")
        echo "  Total time: ${TOTAL_S}s"
        
        # Add to JSON
        if [[ "$FIRST" != "true" ]]; then
            echo "," >> "$RESULT_FILE"
        fi
        FIRST=false
        
        cat >> "$RESULT_FILE" << EOF
    {
      "distribution": "$dist",
      "version": "$JDK_VERSION",
      "url": "$URL",
      "size": $SIZE,
      "size_mb": $SIZE_MB,
      "download": {
        "duration_ms": $DOWNLOAD_MS,
        "duration_s": $DOWNLOAD_S,
        "speed_mbps": $SPEED_MBPS
      },
      "extraction": {
        "duration_ms": $EXTRACT_MS,
        "duration_s": $EXTRACT_S
      },
      "total_s": $TOTAL_S
    }
EOF
        
        # Clean up
        rm -rf "$TEMP_DIR"/*
    else
        echo "  Download failed!"
    fi
    
    echo ""
done

# Close JSON
echo "" >> "$RESULT_FILE"
echo "  ]" >> "$RESULT_FILE"
echo "}" >> "$RESULT_FILE"

# Display results
echo "=== Results saved to: $RESULT_FILE ==="
echo ""

# Show timeout recommendations
echo "=== Timeout Recommendations ==="
echo "Based on GraalVM (320MB) as worst case:"
python3 -c "
speeds = [(100, 12.5), (50, 6.25), (20, 2.5), (10, 1.25), (5, 0.625), (2, 0.25)]
for mbps, mb_per_s in speeds:
    dl_time = int(320 / mb_per_s)
    total = dl_time + 5
    print(f'  {mbps:3d} Mbps ({mb_per_s:5.2f} MB/s): {total:4d}s (~{total//60}m)')
"

echo ""
echo "Recommended default timeout: 600s (10 minutes)"
echo "This covers connections down to 5Mbps"