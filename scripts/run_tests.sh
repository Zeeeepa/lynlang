#!/bin/bash

# Zen Test Runner Wrapper
# Delegates to the unified Python test runner

echo "=== Zen Language Test Runner ==="

# Check if we have the compiler
if [ ! -f "target/release/zen" ]; then
    echo "Error: Compiler not found at target/release/zen"
    echo "Building compiler..."
    cargo build --release || exit 1
fi

# Run the unified Python test runner
exec python3 scripts/run_tests.py "$@"
