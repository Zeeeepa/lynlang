#!/bin/bash
# Zen Syntax Checker Wrapper
# Checks Zen source files for syntax errors

if [ $# -eq 0 ]; then
    echo "Zen Syntax Checker"
    echo "=================="
    echo ""
    echo "Usage: $0 <file.zen>"
    echo ""
    echo "Examples:"
    echo "  $0 hello.zen"
    echo "  $0 examples/functions.zen"
    exit 1
fi

FILE="$1"

if [ ! -f "$FILE" ]; then
    echo "Error: File '$FILE' not found"
    exit 1
fi

echo "Checking $FILE..."
echo ""

# Use the Rust parser to check syntax
cargo run --bin zen -- --check "$FILE" 2>&1 | grep -v "warning:" | grep -v "Compiling"

# Alternative: Parse the file and check for errors
if cargo run --bin zen -- "$FILE" --emit-ir 2>&1 | grep -q "error\|Error"; then
    echo "✗ Syntax errors found"
    exit 1
else
    echo "✓ No syntax errors found"
    exit 0
fi