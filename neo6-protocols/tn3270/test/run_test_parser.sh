#!/bin/bash

# Script to run the test_parser binary
# This script builds and runs the test_parser binary with proper arguments

set -e  # Exit on any error

# Enable debug logging for Rust applications
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TN3270_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$(dirname "$TN3270_DIR")")"
NEO6_PROXY_DIR="$PROJECT_ROOT/neo6-proxy"

echo "Building test_parser binary..."
cd "$TN3270_DIR"
cargo build --bin test_parser

echo "Rebuilding to ensure latest changes..."
cargo build --bin test_parser

echo ""
echo "Running test_parser with available templates (DEBUG MODE)..."
echo "RUST_LOG=$RUST_LOG"
echo "Working directory: $NEO6_PROXY_DIR"
echo "================================================"

# Change to neo6-proxy directory so relative paths work correctly (like listen_3270.sh)
cd "$NEO6_PROXY_DIR"

# Check if the screens directory exists
SCREENS_DIR="config/screens"
if [ ! -d "$SCREENS_DIR" ]; then
    echo "Error: Screens directory not found at $SCREENS_DIR"
    exit 1
fi

# Run test_parser for each markup template found
for template in "$SCREENS_DIR"/*_markup.txt; do
    if [ -f "$template" ]; then
        echo ""
        echo "Testing template: $(basename "$template")"
        echo "----------------------------------------"
        # Use the direct path to the binary since we're now in neo6-proxy directory
        "$TN3270_DIR/../target/debug/test_parser" "$template"
    fi
done

# If no markup templates found, try with available templates
if ! ls "$SCREENS_DIR"/*_markup.txt >/dev/null 2>&1; then
    echo ""
    echo "No markup templates found. Available files in screens directory:"
    ls -la "$SCREENS_DIR"
    echo ""
    echo "You can run the parser manually with:"
    echo "  \"$TN3270_DIR/target/debug/test_parser\" <path_to_template>"
    echo ""
    echo "Example usage:"
    for file in "$SCREENS_DIR"/*.txt; do
        if [ -f "$file" ]; then
            echo "  \"$TN3270_DIR/target/debug/test_parser\" \"$file\""
        fi
    done
fi
