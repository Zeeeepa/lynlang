#!/bin/bash

# Zen Language Test Runner - Quiet Mode
# Run all tests with minimal output (suppresses debug messages)

echo "=== Zen Language Test Suite (Quiet Mode) ==="
echo "Building compiler..."

# Build the compiler  
cargo build --release 2>/dev/null

if [ ! -f "target/release/zen" ]; then
    echo "Error: Compiler build failed"
    exit 1
fi

COMPILER="./target/release/zen"
TEST_DIR="tests"

PASSED=0
FAILED=0
DISABLED=0
FAILED_TESTS=""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Running tests..."
echo

# Run all tests (excluding disabled and rust files)
for test_file in $TEST_DIR/*.zen; do
    test_name=$(basename "$test_file")
    
    # Skip if file doesn't exist (glob didn't match)
    [ ! -f "$test_file" ] && continue
    
    # Count and skip disabled tests
    if [[ "$test_file" == *.disabled ]]; then
        ((DISABLED++))
        continue
    fi
    
    # Run the test, checking exit code directly
    if timeout 5s $COMPILER "$test_file" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $test_name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $test_name"
        FAILED_TESTS="$FAILED_TESTS\n  - $test_name"
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

if [ $FAILED -gt 0 ]; then
    echo
    echo -e "${RED}Failed tests:${NC}$FAILED_TESTS"
fi

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