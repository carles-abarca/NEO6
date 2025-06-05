#!/bin/bash
# Launch neo6-proxy with TN3270 listener on port 2323 (debug log level)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Compilar neo6-protocols primero
cd "$PROJECT_ROOT/../neo6-protocols" && cargo build || exit 1
# Compilar neo6-proxy después
cd "$PROJECT_ROOT" && cargo build || exit 1

# Ejecutar el listener
cd "$PROJECT_ROOT"
cargo run --manifest-path "$PROJECT_ROOT/Cargo.toml" --bin neo6-proxy -- --protocol=tn3270 --port=2323 --log-level=debug
