#!/bin/bash

# Test runner for zen language test suite
# Runs all .zen test files in the tests/ directory and reports results

cd "$(dirname "$0")/../.."

# Build compiler first
echo "Building compiler..."
cargo build 2>&1 | grep -E "(Compiling|Finished|error)" || true

if [ ! -f "target/debug/zen" ]; then
    echo "Error: Compiler binary not found at target/debug/zen"
    exit 1
fi

# Run tests
echo "Running test suite..."
echo "===================="

PASS_COUNT=0
FAIL_COUNT=0
TOTAL_COUNT=0

cd tests

for test_file in *.zen; do
    if [ -f "$test_file" ]; then
        TOTAL_COUNT=$((TOTAL_COUNT + 1))
        
        # Skip disabled tests
        if [[ "$test_file" == *.disabled ]]; then
            continue
        fi
        
        # Run test with timeout
        OUTPUT=$(timeout 2 ../target/debug/zen "$test_file" 2>&1)
        EXIT_CODE=$?
        
        if [ $EXIT_CODE -eq 0 ]; then
            echo "✓ $test_file"
            PASS_COUNT=$((PASS_COUNT + 1))
        else
            echo "✗ $test_file (exit code: $EXIT_CODE)"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    fi
done

echo "===================="
echo "Test Results:"
echo "  Total: $TOTAL_COUNT"
echo "  Passed: $PASS_COUNT"
echo "  Failed: $FAIL_COUNT"
if [ $TOTAL_COUNT -gt 0 ]; then
    PASS_RATE=$((PASS_COUNT * 100 / TOTAL_COUNT))
    echo "  Pass Rate: ${PASS_RATE}%"
fi

exit $FAIL_COUNT
