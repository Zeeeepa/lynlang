#!/bin/bash

# Comprehensive Zen Language Test Runner
set +e  # Don't exit on first failure

echo "=== Zen Language Test Suite - Complete Run ==="

# Ensure compiler exists
if [ ! -f "./target/release/zen" ]; then
    echo "Error: Compiler not found. Run 'cargo build --release' first."
    exit 1
fi

COMPILER="./target/release/zen"
TEST_DIR="tests"
PASSED=0
FAILED=0
SEGFAULTS=0
TOTAL=0

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Compiler: $COMPILER"
echo "Test directory: $TEST_DIR"
echo "Running ALL tests in $TEST_DIR"
echo

# Test all .zen files in tests directory
for test_file in $TEST_DIR/*.zen; do
    if [ -f "$test_file" ]; then
        basename=$(basename "$test_file")
        echo -n "[$((TOTAL + 1))] Testing $basename... "
        TOTAL=$((TOTAL + 1))
        
        # Compile and run test with timeout
        timeout 5s $COMPILER "$test_file" > /tmp/test_output 2>&1
        exit_code=$?
        
        if [ $exit_code -eq 0 ]; then
            echo -e "${GREEN}PASS${NC}"
            PASSED=$((PASSED + 1))
        elif [ $exit_code -eq 124 ]; then
            echo -e "${YELLOW}TIMEOUT${NC}"
            FAILED=$((FAILED + 1))
        elif [ $exit_code -eq 139 ]; then
            echo -e "${RED}SEGFAULT${NC}"
            SEGFAULTS=$((SEGFAULTS + 1))
            FAILED=$((FAILED + 1))
        else
            echo -e "${RED}FAIL${NC} (exit code: $exit_code)"
            FAILED=$((FAILED + 1))
        fi
    fi
done

echo
echo "=== Test Summary ==="
echo "Total tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC} ($((PASSED * 100 / TOTAL))%)"
echo -e "Failed: ${RED}$FAILED${NC} ($((FAILED * 100 / TOTAL))%)"
if [ $SEGFAULTS -gt 0 ]; then
    echo -e "Segfaults: ${RED}$SEGFAULTS${NC}"
fi

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi