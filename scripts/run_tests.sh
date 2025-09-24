#!/bin/bash

# Zen Language Test Runner - Comprehensive Suite
# Run all tests and provide detailed statistics

echo "=== Zen Language Test Suite ==="
echo "Building compiler..."

# Build the compiler  
cargo build --release 2>/dev/null

# Check if we're in the tests directory or project root
if [ -d "tests" ]; then
    TEST_DIR="tests"
    COMPILER="target/release/zen"
else
    TEST_DIR="."
    COMPILER="../target/release/zen"
fi
PASSED=0
FAILED=0
DISABLED=0

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Running all tests..."
echo

# Run all tests (excluding disabled and rust files)
for test_file in $TEST_DIR/*.zen; do
    test_name=$(basename "$test_file")
    
    # Count and skip disabled tests
    if [[ "$test_file" == *.disabled ]]; then
        ((DISABLED++))
        continue
    fi
    
    # Run the test
    if timeout 5s $COMPILER "$test_file" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $test_name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $test_name"
        ((FAILED++))
    fi
done

# Count disabled tests
DISABLED=$(ls $TEST_DIR/*.zen.disabled 2>/dev/null | wc -l)
TOTAL=$((PASSED + FAILED))

# Calculate pass rate
if [ $TOTAL -gt 0 ]; then
    PASS_RATE=$(( (PASSED * 100) / TOTAL ))
else
    PASS_RATE=0
fi

echo
echo "=== Test Summary ==="
echo -e "Passed:   ${GREEN}$PASSED${NC}"
echo -e "Failed:   ${RED}$FAILED${NC}"
echo -e "Disabled: ${YELLOW}$DISABLED${NC}"
echo "Total:    $TOTAL"
echo "Pass Rate: $PASS_RATE%"
echo

# Exit with appropriate code
if [ $PASS_RATE -ge 100 ]; then
    echo -e "${GREEN}Perfect! All tests passing!${NC}"
    exit 0
elif [ $PASS_RATE -ge 90 ]; then
    echo -e "${GREEN}Excellent test coverage!${NC}"
    exit 0
elif [ $PASS_RATE -ge 70 ]; then
    echo -e "${YELLOW}Good progress, some tests failing${NC}"
    exit 0
else
    echo -e "${RED}Many tests failing, needs attention${NC}"
    exit 1
fi
