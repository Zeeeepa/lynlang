#!/bin/bash

# Simple Zen test runner
echo "Building Zen compiler..."
cargo build --release --quiet

echo ""
echo "Testing Zen files..."
echo "===================="

# Test the basic import file that we know works
echo -n "Testing basic_import_test.zen... "
if ./target/release/zen tests/basic_import_test.zen > /dev/null 2>&1; then
    echo "✓ PASSED"
else
    echo "✗ FAILED"
fi

# Test arithmetic
echo -n "Testing test_arithmetic.zen... "
if ./target/release/zen tests/test_arithmetic.zen > /dev/null 2>&1; then
    echo "✓ PASSED"
else
    echo "✗ FAILED"
fi

# Test simple examples
for file in examples/01_basics_working.zen examples/02_functions_working.zen; do
    if [ -f "$file" ]; then
        name=$(basename "$file")
        echo -n "Testing $name... "
        if ./target/release/zen "$file" > /dev/null 2>&1; then
            echo "✓ PASSED"
        else
            echo "✗ FAILED"
        fi
    fi
done

echo ""
echo "Test run complete!"