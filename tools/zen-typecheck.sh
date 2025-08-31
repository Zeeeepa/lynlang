#!/bin/bash

# Zen Type Checker
# Performs type checking on Zen source files

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if file argument is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <file.zen> [file2.zen ...]"
    echo "Type checks Zen source files"
    exit 1
fi

# Function to type check a single file
typecheck_file() {
    local file="$1"
    
    if [ ! -f "$file" ]; then
        echo -e "${RED}✗${NC} $file"
        echo "  File not found"
        return 1
    fi
    
    # Run the Zen compiler with type checking flag
    output=$(cargo run --release --bin zen -- --typecheck-only "$file" 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $file"
        echo "  Type checking passed"
        return 0
    else
        echo -e "${RED}✗${NC} $file"
        # Extract type errors
        echo "$output" | grep -E "(Type error|Type mismatch|Cannot infer type|Undefined type)" | while IFS= read -r line; do
            echo "  $line"
        done
        return 1
    fi
}

# Track overall success
total_files=0
passed_files=0
failed_files=0

echo "=== Zen Type Checker ==="
echo

# Process each file
for file in "$@"; do
    ((total_files++))
    if typecheck_file "$file"; then
        ((passed_files++))
    else
        ((failed_files++))
    fi
    echo
done

# Print summary
echo "=== Summary ==="
echo "Total: $total_files"
echo -e "Passed: ${GREEN}$passed_files${NC}"
echo -e "Failed: ${RED}$failed_files${NC}"

# Exit with error if any file failed
if [ $failed_files -gt 0 ]; then
    exit 1
fi