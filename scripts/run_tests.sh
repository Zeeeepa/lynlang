#!/bin/bash

# Zen Test Runner Wrapper
# Delegates to the unified Python test runner

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPILER_BINARY="$PROJECT_ROOT/target/release/zen"

echo "=== Zen Language Test Runner ==="

# Check if we have the compiler
if [ ! -f "$COMPILER_BINARY" ]; then
    echo "Error: Compiler not found at $COMPILER_BINARY"
    echo "Building compiler..."
    cd "$PROJECT_ROOT"
    cargo build --release || exit 1
fi

# Run the unified Python test runner
cd "$PROJECT_ROOT"
exec python3 scripts/run_tests.py "$@"
