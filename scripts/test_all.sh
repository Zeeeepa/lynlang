#!/bin/bash

# Zen Language Test Runner
# Runs all tests in the tests/ directory

echo "================================"
echo "Zen Language Test Suite"
echo "Based on LANGUAGE_SPEC.zen"
echo "================================"
echo

# Build the compiler first
echo "Building Zen compiler..."
cargo build --release 2>/dev/null
if [ $? -ne 0 ]; then
    echo "❌ Failed to build compiler"
    exit 1
fi
echo "✅ Compiler built successfully"
echo

# Clean and create output directory for test binaries
rm -rf test_output
mkdir -p test_output

# Counter for test results
TOTAL=0
PASSED=0
FAILED=0

# Function to run a single test
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .zen)
    
    echo -n "Testing $test_name... "
    TOTAL=$((TOTAL + 1))
    
    # Compile the test (capture both stdout and stderr)
    output=$(cargo run --bin zen -- "$test_file" -o "test_output/$test_name" 2>&1)
    
    # Check if compilation succeeded (look for success message)
    if echo "$output" | grep -q "Successfully compiled"; then
        echo "✅ PASSED"
        PASSED=$((PASSED + 1))
    else
        echo "❌ FAILED"
        FAILED=$((FAILED + 1))
        # Show error for debugging (filter out warnings)
        echo "  Error output:"
        echo "$output" | grep -E "(Error|error:|Failed)" | head -5 | sed 's/^/    /'
    fi
}

# Run all tests in tests/ directory
echo "Running tests..."
echo "----------------"

# Core feature tests
run_test "tests/zen_test_hello_world.zen"
run_test "tests/zen_test_variables_complete.zen"
run_test "tests/zen_test_structs_complete.zen"
run_test "tests/zen_test_enums_and_option.zen"
run_test "tests/zen_test_pattern_matching_complete.zen"
run_test "tests/zen_test_loops_and_ranges.zen"
run_test "tests/zen_test_functions_and_ufc.zen"
run_test "tests/zen_test_traits.zen"
run_test "tests/zen_test_error_handling.zen"
run_test "tests/zen_test_pointers.zen"
run_test "tests/zen_test_imports.zen"
run_test "tests/zen_test_collections.zen"

# Also run existing tests
for test_file in tests/zen_test_spec_*.zen; do
    if [ -f "$test_file" ]; then
        run_test "$test_file"
    fi
done

# Summary
echo
echo "================================"
echo "Test Results Summary"
echo "================================"
echo "Total:  $TOTAL"
echo "Passed: $PASSED ✅"
echo "Failed: $FAILED ❌"
echo

if [ $FAILED -eq 0 ]; then
    echo "🎉 All tests passed!"
    exit 0
else
    echo "⚠️  Some tests failed. Please review the implementation."
    exit 1
fi