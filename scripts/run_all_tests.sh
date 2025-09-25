#!/bin/bash

# Test runner script for Zen language test suite

cd tests
PASS=0
FAIL=0
TIMEOUT=0

for file in $(find . -name "*.zen" -type f | sort); do
    echo -n "$file: "
    output=$(timeout 2 ../target/release/zen "$file" 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 124 ]; then
        echo "TIMEOUT"
        ((TIMEOUT++))
    elif [ $exit_code -eq 0 ]; then
        echo "PASS"
        ((PASS++))
    else
        echo "FAIL (exit code: $exit_code)"
        ((FAIL++))
    fi
done

echo ""
echo "===== TEST RESULTS ====="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Timeout: $TIMEOUT"
TOTAL=$((PASS + FAIL + TIMEOUT))
echo "Total: $TOTAL"
PASS_RATE=$(echo "scale=2; $PASS * 100 / $TOTAL" | bc)
echo "Pass rate: $PASS_RATE%"