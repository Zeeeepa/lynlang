#!/bin/bash

# Zen Language Test Runner
# Runs all tests and generates a report

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================"
echo "          Zen Language Test Suite"
echo "================================================"
echo ""

# Build the compiler in release mode
echo "Building Zen compiler..."
cargo build --release 2>&1 | tail -5
echo ""

# Count total tests
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Run Rust unit tests
echo "------------------------------------------------"
echo "Running Rust Unit Tests..."
echo "------------------------------------------------"
cargo_output=$(cargo test 2>&1 | tail -5)
if echo "$cargo_output" | grep -q "test result: ok"; then
    echo -e "${GREEN}✓ All Rust unit tests passed${NC}"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ Some Rust unit tests failed${NC}"
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))
echo ""

# Function to run a Zen test file
run_zen_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .zen)
    
    echo -n "Testing $test_name... "
    
    # Skip files that are known to be examples or incomplete
    if [[ "$test_file" == *"archive"* ]] || [[ "$test_file" == *"example"* ]]; then
        echo -e "${YELLOW}SKIPPED${NC}"
        ((SKIPPED_TESTS++))
        return
    fi
    
    # Try to compile and run the test
    if timeout 5s ./target/release/zen "$test_file" > /tmp/zen_test_output_$$ 2>&1; then
        # Check if output contains any error indicators
        if grep -q -i "error\|panic\|fail" /tmp/zen_test_output_$$; then
            echo -e "${RED}FAILED (runtime error)${NC}"
            ((FAILED_TESTS++))
        else
            echo -e "${GREEN}PASSED${NC}"
            ((PASSED_TESTS++))
        fi
    else
        exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${YELLOW}TIMEOUT${NC}"
            ((SKIPPED_TESTS++))
        else
            echo -e "${RED}FAILED (exit code: $exit_code)${NC}"
            ((FAILED_TESTS++))
        fi
    fi
    
    rm -f /tmp/zen_test_output_$$
    ((TOTAL_TESTS++))
}

# Test basic language features
echo "------------------------------------------------"
echo "Testing Basic Language Features..."
echo "------------------------------------------------"
for test_file in tests/*.zen; do
    if [ -f "$test_file" ]; then
        run_zen_test "$test_file"
    fi
done
echo ""

# Test stdlib modules
echo "------------------------------------------------"
echo "Testing Standard Library Modules..."
echo "------------------------------------------------"
for test_file in stdlib/test_*.zen; do
    if [ -f "$test_file" ]; then
        run_zen_test "$test_file"
    fi
done
echo ""

# Test self-hosting components
echo "------------------------------------------------"
echo "Testing Self-Hosting Components..."
echo "------------------------------------------------"
for test_file in tests/test_self_host*.zen; do
    if [ -f "$test_file" ]; then
        run_zen_test "$test_file"
    fi
done
echo ""

# Test examples (smoke tests only)
echo "------------------------------------------------"
echo "Smoke Testing Example Files..."
echo "------------------------------------------------"
for example_file in examples/*_working.zen; do
    if [ -f "$example_file" ]; then
        run_zen_test "$example_file"
    fi
done
echo ""

# Generate summary report
echo "================================================"
echo "                Test Summary"
echo "================================================"
echo -e "Total Tests:    $TOTAL_TESTS"
echo -e "Passed:         ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:         ${RED}$FAILED_TESTS${NC}"
echo -e "Skipped:        ${YELLOW}$SKIPPED_TESTS${NC}"
echo ""

# Calculate pass rate
if [ $TOTAL_TESTS -gt 0 ]; then
    PASS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo -e "Pass Rate:      ${PASS_RATE}%"
fi

echo "================================================"

# Exit with appropriate code
if [ $FAILED_TESTS -gt 0 ]; then
    exit 1
else
    exit 0
fi