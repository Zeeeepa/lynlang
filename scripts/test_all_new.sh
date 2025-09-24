#!/bin/bash

# Comprehensive Test Runner for Zen Language
# Runs ALL tests in the tests/ directory

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Zen Language Full Test Suite ==="
echo

# Check compiler exists
COMPILER="./target/release/zen"
if [ ! -f "$COMPILER" ]; then
    echo "Error: Compiler not found at $COMPILER"
    echo "Building compiler..."
    cargo build --release || exit 1
fi

TEST_DIR="tests"
PASSED=0
FAILED=0
TOTAL=0

echo "Compiler: $COMPILER"
echo "Test directory: $TEST_DIR"
echo "Running all .zen files in tests/"
echo

# Find all .zen files in the test directory
TEST_FILES=$(find "$TEST_DIR" -name "*.zen" -type f | sort)

for test_file in $TEST_FILES; do
    TOTAL=$((TOTAL + 1))
    TEST_NAME=$(basename "$test_file")
    
    echo -n "[$TOTAL] Testing $TEST_NAME... "
    
    # Run the test and capture exit code
    if timeout 2s "$COMPILER" "$test_file" > /tmp/test_output.txt 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        PASSED=$((PASSED + 1))
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 124 ]; then
            echo -e "${YELLOW}TIMEOUT${NC}"
        elif [ $EXIT_CODE -eq 139 ]; then
            echo -e "${RED}SEGFAULT${NC}"
        else
            echo -e "${RED}FAIL${NC} (exit code: $EXIT_CODE)"
        fi
        FAILED=$((FAILED + 1))
        
        # Optionally show error for debugging
        if [ "${SHOW_ERRORS:-0}" = "1" ]; then
            echo "  Error output:"
            head -5 /tmp/test_output.txt | sed 's/^/    /'
        fi
    fi
done

echo
echo "=== Test Summary ==="
echo "Total tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC} ($((PASSED * 100 / TOTAL))%)"
echo -e "Failed: ${RED}$FAILED${NC} ($((FAILED * 100 / TOTAL))%)"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi