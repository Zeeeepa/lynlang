#!/bin/bash

# Zen Language Test Suite
# Run all zen_test_*.zen files and report results

echo "================================"
echo "Zen Language Test Suite"
echo "================================"
echo ""

# Build the compiler first
echo "Building compiler..."
cargo build --release 2>&1 | tail -2
if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Compiler built successfully"
echo ""

# Find all test files
test_files=$(ls zen_test_*.zen 2>/dev/null)

if [ -z "$test_files" ]; then
    echo "No test files found (zen_test_*.zen)"
    exit 1
fi

passed=0
failed=0
total=0

echo "Running tests..."
echo "--------------------------------"

for test_file in $test_files; do
    total=$((total + 1))
    test_name=$(basename "$test_file" .zen)
    output_file="${test_name}_out"
    
    # Compile the test
    ./target/release/zen "$test_file" -o "$output_file" 2>/dev/null
    
    if [ $? -eq 0 ]; then
        # Run the compiled test
        ./"$output_file" > /tmp/test_output.txt 2>&1
        if [ $? -eq 0 ]; then
            echo "✅ $test_name"
            passed=$((passed + 1))
        else
            echo "❌ $test_name (runtime error)"
            failed=$((failed + 1))
        fi
        # Clean up
        rm -f "$output_file" "${output_file}.ll"
    else
        echo "❌ $test_name (compilation error)"
        failed=$((failed + 1))
    fi
done

echo ""
echo "================================"
echo "Test Results"
echo "================================"
echo "Total:  $total"
echo "Passed: $passed"
echo "Failed: $failed"
echo ""

if [ $failed -eq 0 ]; then
    echo "🎉 All tests passed!"
    exit 0
else
    echo "⚠️  Some tests failed"
    exit 1
fi