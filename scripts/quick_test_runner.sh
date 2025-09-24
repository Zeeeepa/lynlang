#!/bin/bash

# Quick test runner for Zen test suite
# Counts pass/fail rate for all test files

PASS=0
FAIL=0
TOTAL=0

echo "Running Zen test suite..."
echo "========================"

for test_file in tests/*.zen; do
    if [ -f "$test_file" ]; then
        TOTAL=$((TOTAL + 1))
        
        # Run test and capture exit code
        ./target/release/zen "$test_file" >/dev/null 2>&1
        EXIT_CODE=$?
        
        if [ $EXIT_CODE -eq 0 ]; then
            PASS=$((PASS + 1))
            echo "✓ $(basename "$test_file")"
        else
            FAIL=$((FAIL + 1))
            echo "✗ $(basename "$test_file")"
        fi
    fi
done

echo "========================"
echo "Results: $PASS/$TOTAL passed ($(( PASS * 100 / TOTAL ))%)"
echo "Failed: $FAIL tests"