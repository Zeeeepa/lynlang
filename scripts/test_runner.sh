#!/bin/bash

# Zen Language Test Runner
# Runs all enabled tests and provides summary

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build compiler in release mode
echo "Building Zen compiler..."
cargo build --release 2>/dev/null || {
    echo "Error: Failed to build compiler"
    exit 1
}

COMPILER="./target/release/zen"

# Initialize counters
TOTAL=0
PASSED=0
FAILED=0
SEGFAULTS=0

# Create temp directory for test outputs
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "Running Zen tests..."
echo "================================================"

# Find and run all .zen test files (excluding .disabled)
for test_file in tests/*.zen tests/tests/*.zen; do
    # Skip if file doesn't exist or is disabled
    [[ ! -f "$test_file" ]] && continue
    [[ "$test_file" == *.disabled ]] && continue
    
    TOTAL=$((TOTAL + 1))
    base_name=$(basename "$test_file")
    
    # Run test with timeout
    if timeout 5s $COMPILER "$test_file" > "$TEMP_DIR/$base_name.out" 2>&1; then
        echo -e "${GREEN}✅${NC} $base_name"
        PASSED=$((PASSED + 1))
    else
        exit_code=$?
        if [[ $exit_code -eq 139 ]]; then
            echo -e "${RED}❌${NC} $base_name (SEGFAULT)"
            SEGFAULTS=$((SEGFAULTS + 1))
            FAILED=$((FAILED + 1))
        else
            echo -e "${RED}❌${NC} $base_name"
            FAILED=$((FAILED + 1))
        fi
    fi
done

echo "================================================"
echo "Test Results: $PASSED/$TOTAL passed ($(echo "scale=1; $PASSED * 100 / $TOTAL" | bc)%)"
echo "Failures: $FAILED (including $SEGFAULTS segfaults)"

# Count disabled tests
DISABLED=$(find tests -name "*.zen.disabled" 2>/dev/null | wc -l)
if [[ $DISABLED -gt 0 ]]; then
    echo "Disabled tests: $DISABLED"
fi

exit $([[ $FAILED -eq 0 ]] && echo 0 || echo 1)
