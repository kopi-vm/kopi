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
#
# Generate metadata files from foojay API for Kopi
# This script is used in CI/CD to generate metadata archives
#

set -euo pipefail

# Default values
OUTPUT_DIR="${OUTPUT_DIR:-./metadata}"
DISTRIBUTIONS="${DISTRIBUTIONS:-}"
PLATFORMS="${PLATFORMS:-}"
JAVAFX="${JAVAFX:-false}"
PARALLEL="${PARALLEL:-4}"
ARCHIVE_NAME="${ARCHIVE_NAME:-}"
DRY_RUN="${DRY_RUN:-false}"
NO_MINIFY="${NO_MINIFY:-false}"
FORCE="${FORCE:-false}"
CONFIG_FILE="${CONFIG_FILE:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Generate metadata files from foojay API

OPTIONS:
    -o, --output DIR         Output directory (default: ./metadata)
    -d, --distributions LIST Comma-separated list of distributions to include
    -p, --platforms LIST     Comma-separated list of platforms (format: os-arch-libc)
    -j, --javafx            Include JavaFX bundled versions
    -t, --parallel NUM      Number of parallel API requests (default: 4)
    -c, --config FILE       Configuration file (TOML format)
    -a, --archive NAME      Create archive with specified name after generation
    --dry-run               Show what would be generated without actually writing files
    --no-minify             Don't minify JSON output (default is to minify)
    --force                 Force fresh generation, ignoring any existing state files
    -h, --help              Show this help message

EXAMPLES:
    # Generate metadata for all distributions and platforms
    $0

    # Generate metadata for specific distributions
    $0 --distributions temurin,corretto,zulu

    # Generate metadata for specific platforms
    $0 --platforms linux-x64-glibc,macos-aarch64

    # Generate metadata using configuration file
    $0 --config metadata-gen.toml

    # Generate and create archive
    $0 --archive metadata-\$(date +%Y-%m).tar.gz

    # Dry-run to see what would be generated
    $0 --dry-run --distributions temurin

    # Force fresh generation with unminified output
    $0 --force --no-minify

    # CI/CD usage with environment variables
    DISTRIBUTIONS=temurin,corretto PLATFORMS=linux-x64-glibc $0
    
    # Generate example configuration file
    kopi-metadata-gen generate-config --output metadata-gen.toml
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -d|--distributions)
            DISTRIBUTIONS="$2"
            shift 2
            ;;
        -p|--platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        -j|--javafx)
            JAVAFX="true"
            shift
            ;;
        -t|--parallel)
            PARALLEL="$2"
            shift 2
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        -a|--archive)
            ARCHIVE_NAME="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
        --no-minify)
            NO_MINIFY="true"
            shift
            ;;
        --force)
            FORCE="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Check if kopi-metadata-gen is available
if ! command -v kopi-metadata-gen &> /dev/null; then
    print_info "kopi-metadata-gen not found in PATH, checking cargo build"
    
    # Try to find it in target directory
    METADATA_GEN=""
    for path in "./target/release/kopi-metadata-gen" "./target/debug/kopi-metadata-gen"; do
        if [[ -x "$path" ]]; then
            METADATA_GEN="$path"
            break
        fi
    done
    
    if [[ -z "$METADATA_GEN" ]]; then
        print_error "kopi-metadata-gen not found. Please build it first:"
        print_error "  cargo build --release --bin kopi-metadata-gen"
        exit 1
    fi
else
    METADATA_GEN="kopi-metadata-gen"
fi

print_info "Using metadata generator: $METADATA_GEN"

# Build command arguments
CMD_ARGS=("generate" "--output" "$OUTPUT_DIR" "--parallel" "$PARALLEL")

if [[ -n "$CONFIG_FILE" ]]; then
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Configuration file not found: $CONFIG_FILE"
        exit 1
    fi
    CMD_ARGS+=("--config" "$CONFIG_FILE")
fi

if [[ -n "$DISTRIBUTIONS" ]]; then
    CMD_ARGS+=("--distributions" "$DISTRIBUTIONS")
fi

if [[ -n "$PLATFORMS" ]]; then
    CMD_ARGS+=("--platforms" "$PLATFORMS")
fi

if [[ "$JAVAFX" == "true" ]]; then
    CMD_ARGS+=("--javafx")
fi

if [[ "$DRY_RUN" == "true" ]]; then
    CMD_ARGS+=("--dry-run")
fi

if [[ "$NO_MINIFY" == "true" ]]; then
    CMD_ARGS+=("--no-minify")
fi

if [[ "$FORCE" == "true" ]]; then
    CMD_ARGS+=("--force")
fi

# Run metadata generation
print_info "Starting metadata generation..."
print_info "Output directory: $OUTPUT_DIR"

if [[ -n "$CONFIG_FILE" ]]; then
    print_info "Configuration file: $CONFIG_FILE"
fi

if [[ -n "$DISTRIBUTIONS" ]]; then
    print_info "Distributions: $DISTRIBUTIONS"
fi

if [[ -n "$PLATFORMS" ]]; then
    print_info "Platforms: $PLATFORMS"
fi

if [[ "$DRY_RUN" == "true" ]]; then
    print_info "Mode: DRY RUN (no files will be written)"
fi

if [[ "$FORCE" == "true" ]]; then
    print_info "Mode: FORCE (ignoring existing state files)"
fi

if [[ "$NO_MINIFY" == "true" ]]; then
    print_info "JSON output: NOT MINIFIED"
fi

# Execute the generator
if ! "$METADATA_GEN" "${CMD_ARGS[@]}"; then
    print_error "Metadata generation failed"
    exit 1
fi

# Skip validation and archive creation in dry-run mode
if [[ "$DRY_RUN" == "true" ]]; then
    print_info "Dry-run completed successfully!"
    exit 0
fi

# Validate the generated metadata
print_info "Validating generated metadata..."
if ! "$METADATA_GEN" validate --input "$OUTPUT_DIR"; then
    print_error "Metadata validation failed"
    exit 1
fi

# Create archive if requested
if [[ -n "$ARCHIVE_NAME" ]]; then
    print_info "Creating archive: $ARCHIVE_NAME"
    
    # Ensure archive name has proper extension
    if [[ ! "$ARCHIVE_NAME" =~ \.(tar\.gz|tgz)$ ]]; then
        ARCHIVE_NAME="${ARCHIVE_NAME}.tar.gz"
    fi
    
    # Create archive
    if tar czf "$ARCHIVE_NAME" -C "$OUTPUT_DIR" .; then
        print_info "Archive created successfully: $ARCHIVE_NAME"
        
        # Show archive details
        ARCHIVE_SIZE=$(du -h "$ARCHIVE_NAME" | cut -f1)
        print_info "Archive size: $ARCHIVE_SIZE"
        
        # List contents
        print_info "Archive contents:"
        tar tzf "$ARCHIVE_NAME" | head -20
        
        # Count files
        FILE_COUNT=$(tar tzf "$ARCHIVE_NAME" | wc -l)
        print_info "Total files in archive: $FILE_COUNT"
    else
        print_error "Failed to create archive"
        exit 1
    fi
fi

# Summary
print_info "Metadata generation completed successfully!"

# Show some statistics (skip in dry-run mode)
if [[ "$DRY_RUN" != "true" ]] && [[ -f "$OUTPUT_DIR/index.json" ]]; then
    FILE_COUNT=$(find "$OUTPUT_DIR" -name "*.json" | wc -l)
    INDEX_SIZE=$(du -h "$OUTPUT_DIR/index.json" | cut -f1)
    TOTAL_SIZE=$(du -sh "$OUTPUT_DIR" | cut -f1)
    
    print_info "Statistics:"
    print_info "  Total JSON files: $FILE_COUNT"
    print_info "  Index file size: $INDEX_SIZE"
    print_info "  Total size: $TOTAL_SIZE"
fi

# GitHub Actions output
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "output_dir=$OUTPUT_DIR" >> "$GITHUB_OUTPUT"
    if [[ -n "$ARCHIVE_NAME" ]]; then
        echo "archive_name=$ARCHIVE_NAME" >> "$GITHUB_OUTPUT"
    fi
fi

exit 0