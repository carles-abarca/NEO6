#!/bin/bash
# Launch neo6-proxy with TN3270 listener on port 2323 in dynamic mode (debug)

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROTOCOLS_ROOT="$(cd "$PROJECT_ROOT/../neo6-protocols" && pwd)"

# Determine build mode (default: debug)
BUILD_MODE="${1:-debug}"
if [[ "$BUILD_MODE" != "debug" && "$BUILD_MODE" != "release" ]]; then
    echo "Usage: $0 [debug|release]"
    echo "Default: debug"
    exit 1
fi

# Set build flags
if [[ "$BUILD_MODE" == "release" ]]; then
    BUILD_FLAG="--release"
    BUILD_DIR="release"
else
    BUILD_FLAG=""
    BUILD_DIR="debug"
fi

echo "=== Building in $BUILD_MODE mode ==="

# Create delivery directories
DELIVERY_DIR="$PROJECT_ROOT/delivery/$BUILD_DIR"
mkdir -p "$DELIVERY_DIR/lib"
mkdir -p "$DELIVERY_DIR/bin"

echo "=== Compiling neo6-protocols (dynamic libraries) ==="
cd "$PROTOCOLS_ROOT"
cargo build $BUILD_FLAG || {
    echo "Error: Failed to compile neo6-protocols"
    exit 1
}

echo "=== Compiling neo6-proxy ==="
cd "$PROJECT_ROOT"
# Note: We don't use features for dynamic loading - protocols are loaded at runtime
cargo build $BUILD_FLAG || {
    echo "Error: Failed to compile neo6-proxy"
    exit 1
}

echo "=== Copying dynamic libraries to delivery ==="
# Copy all protocol dynamic libraries
for lib in libjca.dylib liblu62.dylib libmq.dylib librest.dylib libtcp.dylib libtn3270.dylib; do
    if [[ -f "$PROTOCOLS_ROOT/target/$BUILD_DIR/$lib" ]]; then
        cp "$PROTOCOLS_ROOT/target/$BUILD_DIR/$lib" "$DELIVERY_DIR/lib/"
        echo "Copied $lib"
    else
        echo "Warning: $lib not found in $PROTOCOLS_ROOT/target/$BUILD_DIR/"
    fi
done

echo "=== Copying neo6-proxy binary to delivery ==="
cp "$PROJECT_ROOT/target/$BUILD_DIR/neo6-proxy" "$DELIVERY_DIR/bin/"

echo "=== Copying configuration files ==="
cp -r "$PROJECT_ROOT/config" "$DELIVERY_DIR/"

echo "=== Setting up library path ==="
# On macOS, we need to set DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH="$DELIVERY_DIR/lib:$DYLD_LIBRARY_PATH"

echo "=== Stopping any existing neo6-proxy instances ==="
pkill -f neo6-proxy || true

echo "=== Moving to delivery directory ==="
cd "$DELIVERY_DIR"

echo "=== Starting neo6-proxy in dynamic mode ==="
echo "Build mode: $BUILD_MODE"
echo "Working directory: $(pwd)"
echo "Library path: $DYLD_LIBRARY_PATH"
echo "Protocol: tn3270"
echo "Port: 2323"
echo "Mode: dynamic"
echo ""

# Set debug logging
export RUST_LOG=debug

# Show available files
echo "=== Delivery contents ==="
echo "Libraries in lib/:"
ls -la lib/ || echo "No lib directory"
echo ""
echo "Binaries in bin/:"
ls -la bin/ || echo "No bin directory"
echo ""
echo "Config files:"
ls -la config/ || echo "No config directory"
echo ""

# Run the proxy in dynamic mode
echo "=== Executing neo6-proxy ==="
exec ./bin/neo6-proxy --protocol=tn3270 --port=2323 --log-level=debug
