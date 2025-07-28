#!/bin/bash
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
    -a, --archive NAME      Create archive with specified name after generation
    -h, --help              Show this help message

EXAMPLES:
    # Generate metadata for all distributions and platforms
    $0

    # Generate metadata for specific distributions
    $0 --distributions temurin,corretto,zulu

    # Generate metadata for specific platforms
    $0 --platforms linux-x64-glibc,macos-aarch64

    # Generate and create archive
    $0 --archive metadata-\$(date +%Y-%m).tar.gz

    # CI/CD usage with environment variables
    DISTRIBUTIONS=temurin,corretto PLATFORMS=linux-x64-glibc $0
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
        -a|--archive)
            ARCHIVE_NAME="$2"
            shift 2
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

if [[ -n "$DISTRIBUTIONS" ]]; then
    CMD_ARGS+=("--distributions" "$DISTRIBUTIONS")
fi

if [[ -n "$PLATFORMS" ]]; then
    CMD_ARGS+=("--platforms" "$PLATFORMS")
fi

if [[ "$JAVAFX" == "true" ]]; then
    CMD_ARGS+=("--javafx")
fi

# Run metadata generation
print_info "Starting metadata generation..."
print_info "Output directory: $OUTPUT_DIR"

if [[ -n "$DISTRIBUTIONS" ]]; then
    print_info "Distributions: $DISTRIBUTIONS"
fi

if [[ -n "$PLATFORMS" ]]; then
    print_info "Platforms: $PLATFORMS"
fi

# Execute the generator
if ! "$METADATA_GEN" "${CMD_ARGS[@]}"; then
    print_error "Metadata generation failed"
    exit 1
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

# Show some statistics
if [[ -f "$OUTPUT_DIR/index.json" ]]; then
    FILE_COUNT=$(find "$OUTPUT_DIR" -name "*.json" | wc -l)
    INDEX_SIZE=$(du -h "$OUTPUT_DIR/index.json" | cut -f1)
    TOTAL_SIZE=$(du -sh "$OUTPUT_DIR" | cut -f1)
    
    print_info "Statistics:"
    print_info "  Total JSON files: $FILE_COUNT"
    print_info "  Index file size: $INDEX_SIZE"
    print_info "  Total size: $TOTAL_SIZE"
fi

# GitHub Actions output
if [[ -n "$GITHUB_OUTPUT" ]]; then
    echo "output_dir=$OUTPUT_DIR" >> "$GITHUB_OUTPUT"
    if [[ -n "$ARCHIVE_NAME" ]]; then
        echo "archive_name=$ARCHIVE_NAME" >> "$GITHUB_OUTPUT"
    fi
fi

exit 0