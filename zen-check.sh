#!/bin/bash

# Zen Syntax Checker
# Quick syntax validation tool for Zen files

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if compiler exists
if [ ! -f "./target/release/zen" ]; then
    echo -e "${YELLOW}Compiler not found. Building...${NC}"
    cargo build --release
fi

COMPILER="./target/release/zen"

# Function to check a single file
check_file() {
    local file=$1
    local show_details=${2:-false}
    
    if [ ! -f "$file" ]; then
        echo -e "${RED}Error: File '$file' not found${NC}"
        return 1
    fi
    
    # Try to compile with syntax checking only
    if $COMPILER "$file" --check-only > /tmp/zen_check.log 2>&1; then
        echo -e "${GREEN}✓${NC} $file"
        if [ "$show_details" = "true" ]; then
            echo "  No syntax errors found"
        fi
        return 0
    else
        echo -e "${RED}✗${NC} $file"
        if [ "$show_details" = "true" ]; then
            echo -e "${RED}  Errors found:${NC}"
            grep -E "error|Error" /tmp/zen_check.log | head -10 | sed 's/^/    /'
        fi
        return 1
    fi
}

# Function to check all files in a directory
check_directory() {
    local dir=$1
    local pattern=${2:-"*.zen"}
    
    if [ ! -d "$dir" ]; then
        echo -e "${RED}Error: Directory '$dir' not found${NC}"
        return 1
    fi
    
    echo -e "${BLUE}Checking files in $dir...${NC}"
    
    local total=0
    local passed=0
    local failed=0
    
    for file in "$dir"/$pattern; do
        if [ -f "$file" ]; then
            total=$((total + 1))
            if check_file "$file" false; then
                passed=$((passed + 1))
            else
                failed=$((failed + 1))
            fi
        fi
    done
    
    echo ""
    echo "Results for $dir:"
    echo "  Total: $total"
    echo -e "  Passed: ${GREEN}$passed${NC}"
    if [ $failed -gt 0 ]; then
        echo -e "  Failed: ${RED}$failed${NC}"
    fi
}

# Main script logic
if [ $# -eq 0 ]; then
    echo "Zen Syntax Checker"
    echo ""
    echo "Usage:"
    echo "  $0 <file.zen>           - Check a single file"
    echo "  $0 -d <directory>       - Check all .zen files in directory"
    echo "  $0 -v <file.zen>        - Check with verbose output"
    echo "  $0 --all                - Check all project files"
    echo ""
    exit 0
fi

case "$1" in
    -d|--directory)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: Directory path required${NC}"
            exit 1
        fi
        check_directory "$2"
        ;;
    -v|--verbose)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: File path required${NC}"
            exit 1
        fi
        check_file "$2" true
        ;;
    --all)
        echo -e "${BLUE}=== Checking All Zen Files ===${NC}"
        echo ""
        
        check_directory "examples"
        echo ""
        check_directory "stdlib"
        echo ""
        check_directory "tests"
        
        echo ""
        echo -e "${BLUE}=== Check Complete ===${NC}"
        ;;
    *)
        # Check single file
        check_file "$1" true
        ;;
esac