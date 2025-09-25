#!/bin/bash

# Test runner for Zen language

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# Project root is one level up from scripts/
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
ZEN_COMPILER="$PROJECT_ROOT/target/release/zen"
TESTS_DIR="$PROJECT_ROOT/tests"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
total_tests=0
passed_tests=0
failed_tests=0
skipped_tests=0

# Arrays to store failed test names
declare -a failed_test_names

# Build the compiler first
echo "Building Zen compiler..."
cd "$PROJECT_ROOT"
cargo build --release 2>&1 > /dev/null
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to build compiler${NC}"
    exit 1
fi

echo "Running Zen test suite..."
echo "========================="

# Run all .zen test files
for test_file in "$TESTS_DIR"/*.zen; do
    # Skip if no .zen files found
    [ ! -f "$test_file" ] && continue
    
    test_name=$(basename "$test_file")
    
    # Skip disabled tests
    if [[ "$test_name" == *".disabled"* ]]; then
        ((skipped_tests++))
        echo -e "${YELLOW}SKIP${NC} $test_name (disabled)"
        continue
    fi
    
    ((total_tests++))
    
    # Run the test
    if timeout 2 "$ZEN_COMPILER" "$test_file" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC} $test_name"
        ((passed_tests++))
    else
        echo -e "${RED}FAIL${NC} $test_name"
        ((failed_tests++))
        failed_test_names+=("$test_name")
    fi
done

# Summary
echo "========================="
echo "Test Results Summary:"
echo -e "Total tests:   $total_tests"
echo -e "Passed:        ${GREEN}$passed_tests${NC}"
echo -e "Failed:        ${RED}$failed_tests${NC}"
echo -e "Skipped:       ${YELLOW}$skipped_tests${NC}"

if [ $failed_tests -eq 0 ] && [ $total_tests -gt 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    PASS_RATE="100.0"
else
    PASS_RATE=$(echo "scale=1; $passed_tests * 100 / $total_tests" | bc 2>/dev/null || echo "0.0")
fi

echo -e "Pass rate:     ${PASS_RATE}%"

# List failed tests if any
if [ $failed_tests -gt 0 ]; then
    echo -e "\nFailed tests:"
    for failed_test in "${failed_test_names[@]}"; do
        echo "  - $failed_test"
    done
fi

# Exit with appropriate code
if [ $failed_tests -eq 0 ]; then
    exit 0
else
    exit 1
fi
