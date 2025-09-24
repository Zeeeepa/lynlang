#!/bin/bash

# Zen Test Runner - Root Directory
# Delegates to the proper test runner

echo "=== Zen Language Test Runner ==="

# Check if we have the compiler
if [ ! -f "target/release/zen" ]; then
    echo "Error: Compiler not found at target/release/zen"
    echo "Building compiler..."
    cargo build --release || exit 1
fi

# Use the Python test runner (moved to scripts)
echo "Running test suite..."
python3 scripts/run_tests.py
