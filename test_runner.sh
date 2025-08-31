#!/bin/bash

# Zen Language Test Runner
# Runs all tests and reports results

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Build the compiler first
echo "Building Zen compiler..."
cargo build --release 2>&1 | tail -5
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to build compiler${NC}"
    exit 1
fi

COMPILER="./target/release/zen"

# Create test output directory
mkdir -p test_output

echo ""
echo "======================================"
echo "       Zen Language Test Suite       "
echo "======================================"
echo ""

# Function to run a single test file
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .zen)
    
    echo -n "Testing $test_name... "
    TOTAL=$((TOTAL + 1))
    
    # Compile the test
    if $COMPILER "$test_file" -o "test_output/$test_name" > "test_output/${test_name}.log" 2>&1; then
        # Check if the test contains a main function
        if grep -q "main\s*=" "$test_file"; then
            # Try to run the compiled test (if it has a main)
            if [ -f "test_output/$test_name" ]; then
                if ./test_output/$test_name > "test_output/${test_name}_run.log" 2>&1; then
                    echo -e "${GREEN}✓ PASSED${NC}"
                    PASSED=$((PASSED + 1))
                else
                    echo -e "${RED}✗ FAILED (runtime)${NC}"
                    echo "  Runtime error in $test_file:"
                    cat "test_output/${test_name}_run.log" | head -5
                    FAILED=$((FAILED + 1))
                fi
            else
                # Compilation succeeded but no executable (might be library)
                echo -e "${GREEN}✓ COMPILED${NC}"
                PASSED=$((PASSED + 1))
            fi
        else
            # No main function, just check compilation
            echo -e "${GREEN}✓ COMPILED${NC}"
            PASSED=$((PASSED + 1))
        fi
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "  Compilation error in $test_file:"
        grep -E "error|Error" "test_output/${test_name}.log" | head -3
        FAILED=$((FAILED + 1))
    fi
}

# Run parser tests
echo "Running Parser Tests..."
echo "----------------------"
for test in tests/parser_*.zen; do
    if [ -f "$test" ]; then
        run_test "$test"
    fi
done

echo ""
echo "Running Example Tests..."
echo "----------------------"
# Test example files
for example in examples/*.zen; do
    if [ -f "$example" ]; then
        # Skip archive directory
        case "$example" in
            *archive*) continue ;;
            *) run_test "$example" ;;
        esac
    fi
done

echo ""
echo "Running Standard Library Tests..."
echo "---------------------------------"
# Test stdlib files (they should at least parse correctly)
for stdlib_file in stdlib/*.zen; do
    if [ -f "$stdlib_file" ]; then
        # Skip test framework files for now
        case "$stdlib_file" in
            *test_*.zen) continue ;;
            *) run_test "$stdlib_file" ;;
        esac
    fi
done

echo ""
echo "Running Integration Tests..."
echo "----------------------------"
# Test specific features
if [ -f "tests/test_import_system.zen" ]; then
    run_test "tests/test_import_system.zen"
fi

# Clean up test outputs (optional)
# rm -rf test_output

echo ""
echo "======================================"
echo "           Test Summary               "
echo "======================================"
echo -e "Total:   $TOTAL"
echo -e "Passed:  ${GREEN}$PASSED${NC}"
echo -e "Failed:  ${RED}$FAILED${NC}"
if [ $SKIPPED -gt 0 ]; then
    echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"
fi

echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. Check test_output/ for details.${NC}"
    exit 1
fi