#!/bin/bash
# Zen Language Linter and Syntax Checker
# Provides detailed syntax checking and code quality analysis

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Options
VERBOSE=false
CHECK_STYLE=false
CHECK_IMPORTS=true
CHECK_TYPES=true
FIX_IMPORTS=false

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to check file syntax
check_syntax() {
    local file=$1
    local filename=$(basename "$file")
    
    # Run the compiler in check mode
    local output
    output=$(./target/debug/zen "$file" --emit-ir 2>&1 || true)
    
    # Check for errors
    if echo "$output" | grep -q "error\|Error\|failed\|Failed"; then
        print_color "$RED" "✗ Syntax errors in $filename:"
        echo "$output" | grep -E "error|Error|failed|Failed" | head -10
        return 1
    else
        print_color "$GREEN" "✓ Syntax OK: $filename"
        return 0
    fi
}

# Function to check imports
check_imports() {
    local file=$1
    local filename=$(basename "$file")
    local has_errors=0
    
    # Check for imports in comptime blocks
    if grep -q "comptime.*{[^}]*@std" "$file"; then
        print_color "$RED" "✗ Imports found in comptime block in $filename"
        print_color "$RED" "  Imports must be at module level, not in comptime blocks"
        local lines=$(grep -n "comptime.*{[^}]*@std" "$file" | cut -d: -f1 | head -3)
        echo "  Error at lines: $lines"
        has_errors=1
    fi
    
    # Check for imports inside functions
    # Look for @std imports that are indented (indicating they're inside a block)
    local indented_imports=$(grep -n "^[[:space:]]\+.*@std\." "$file" | head -5)
    if [ ! -z "$indented_imports" ]; then
        print_color "$RED" "✗ Imports found inside functions/blocks in $filename"
        print_color "$RED" "  Imports must be at module level (not indented)"
        echo "$indented_imports" | while read line; do
            echo "  $line"
        done
        has_errors=1
    fi
    
    # Check for imports after function definitions
    local first_func_line=$(grep -n "^[a-z_][a-zA-Z0-9_]* = (" "$file" | head -1 | cut -d: -f1)
    if [ ! -z "$first_func_line" ]; then
        local late_imports=$(grep -n "^[a-zA-Z_].*:= @std\." "$file" | while read line; do
            line_num=$(echo "$line" | cut -d: -f1)
            if [ "$line_num" -gt "$first_func_line" ]; then
                echo "$line"
            fi
        done)
        
        if [ ! -z "$late_imports" ]; then
            print_color "$YELLOW" "⚠ Imports found after function definitions in $filename"
            print_color "$YELLOW" "  Consider moving imports to the top of the file"
            echo "$late_imports" | head -3
        fi
    fi
    
    # Check for build.import usage
    if grep -q "build\.import(" "$file"; then
        # Verify build is imported first
        if ! grep -q "^build := @std\.build" "$file"; then
            print_color "$YELLOW" "⚠ Using build.import without importing build module"
            print_color "$YELLOW" "  Add: build := @std.build"
        fi
    fi
    
    if [ $has_errors -eq 0 ]; then
        print_color "$GREEN" "✓ Import structure OK: $filename"
    fi
    
    return $has_errors
}

# Function to check code style
check_style() {
    local file=$1
    local filename=$(basename "$file")
    local warnings=0
    
    # Check line length
    local long_lines=$(awk 'length > 100 {print NR}' "$file")
    if [ ! -z "$long_lines" ]; then
        print_color "$YELLOW" "⚠ Lines exceeding 100 characters in $filename:"
        echo "  Lines: $(echo $long_lines | head -c 50)..."
        ((warnings++))
    fi
    
    # Check for tabs vs spaces
    if grep -q $'\t' "$file"; then
        local tab_lines=$(grep -n $'\t' "$file" | head -3 | cut -d: -f1 | tr '\n' ' ')
        print_color "$YELLOW" "⚠ Tabs found in $filename (consider using spaces)"
        echo "  First occurrences at lines: $tab_lines"
        ((warnings++))
    fi
    
    # Check for trailing whitespace
    if grep -q '[[:space:]]$' "$file"; then
        local trail_lines=$(grep -n '[[:space:]]$' "$file" | head -3 | cut -d: -f1 | tr '\n' ' ')
        print_color "$YELLOW" "⚠ Trailing whitespace in $filename"
        echo "  Lines: $trail_lines"
        ((warnings++))
    fi
    
    # Check for missing newline at end of file
    if [ -n "$(tail -c 1 "$file")" ]; then
        print_color "$YELLOW" "⚠ No newline at end of file in $filename"
        ((warnings++))
    fi
    
    if [ $warnings -eq 0 ]; then
        print_color "$GREEN" "✓ Style OK: $filename"
        return 0
    else
        return 1
    fi
}

# Function to check types
check_types() {
    local file=$1
    local filename=$(basename "$file")
    
    # Check for type annotations on public functions
    if grep -q "^[a-z_][a-zA-Z0-9_]* = (" "$file"; then
        local untyped=$(grep -n "^[a-z_][a-zA-Z0-9_]* = (" "$file" | grep -v ")" | head -3)
        if [ ! -z "$untyped" ]; then
            print_color "$YELLOW" "⚠ Functions without return type annotations in $filename:"
            echo "$untyped" | head -3
        fi
    fi
    
    return 0
}

# Function to lint a single file
lint_file() {
    local file=$1
    local filename=$(basename "$file")
    local errors=0
    local warnings=0
    
    print_color "$BLUE" "\nChecking: $file"
    print_color "$BLUE" "----------------------------------------"
    
    # Syntax check
    if ! check_syntax "$file"; then
        ((errors++))
    fi
    
    # Import check
    if [ "$CHECK_IMPORTS" = true ]; then
        if ! check_imports "$file"; then
            ((warnings++))
        fi
    fi
    
    # Style check
    if [ "$CHECK_STYLE" = true ]; then
        if ! check_style "$file"; then
            ((warnings++))
        fi
    fi
    
    # Type check
    if [ "$CHECK_TYPES" = true ]; then
        if ! check_types "$file"; then
            ((warnings++))
        fi
    fi
    
    # Summary for this file
    if [ $errors -eq 0 ] && [ $warnings -eq 0 ]; then
        print_color "$GREEN" "✓ $filename: All checks passed"
    elif [ $errors -gt 0 ]; then
        print_color "$RED" "✗ $filename: $errors error(s), $warnings warning(s)"
    else
        print_color "$YELLOW" "⚠ $filename: $warnings warning(s)"
    fi
    
    return $errors
}

# Function to show usage
show_usage() {
    echo "Zen Language Linter"
    echo "==================="
    echo ""
    echo "Usage: $0 [OPTIONS] <file.zen|directory>"
    echo ""
    echo "Options:"
    echo "  -v, --verbose       Show detailed output"
    echo "  -s, --style         Check code style"
    echo "  -i, --no-imports    Skip import checking"
    echo "  -t, --no-types      Skip type checking"
    echo "  -f, --fix           Attempt to fix issues (experimental)"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 hello.zen                    # Lint a single file"
    echo "  $0 -s examples/                 # Lint directory with style checks"
    echo "  $0 --fix stdlib/lexer.zen       # Lint and fix issues"
    echo ""
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -s|--style)
            CHECK_STYLE=true
            shift
            ;;
        -i|--no-imports)
            CHECK_IMPORTS=false
            shift
            ;;
        -t|--no-types)
            CHECK_TYPES=false
            shift
            ;;
        -f|--fix)
            FIX_IMPORTS=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            TARGET="$1"
            shift
            ;;
    esac
done

# Check if target is provided
if [ -z "$TARGET" ]; then
    show_usage
    exit 1
fi

# Build the compiler if needed
if [ ! -f "./target/debug/zen" ]; then
    print_color "$BLUE" "Building Zen compiler..."
    cargo build --quiet
fi

# Main execution
print_color "$CYAN" "Zen Linter v0.1.0"
print_color "$CYAN" "================="

TOTAL_ERRORS=0
TOTAL_WARNINGS=0
TOTAL_FILES=0

# Check if target is a file or directory
if [ -f "$TARGET" ]; then
    # Single file
    lint_file "$TARGET"
    TOTAL_ERRORS=$?
    TOTAL_FILES=1
elif [ -d "$TARGET" ]; then
    # Directory
    print_color "$BLUE" "Linting directory: $TARGET"
    
    # Find all .zen files
    while IFS= read -r -d '' file; do
        # Skip archive directory
        if [[ "$file" == *"/archive/"* ]]; then
            continue
        fi
        
        lint_file "$file"
        local result=$?
        ((TOTAL_ERRORS += result))
        ((TOTAL_FILES++))
    done < <(find "$TARGET" -name "*.zen" -type f -print0)
else
    print_color "$RED" "Error: '$TARGET' is not a valid file or directory"
    exit 1
fi

# Final summary
echo ""
print_color "$CYAN" "======================================="
print_color "$CYAN" "Linting Summary"
print_color "$CYAN" "======================================="
echo "Files checked: $TOTAL_FILES"

if [ $TOTAL_ERRORS -eq 0 ]; then
    print_color "$GREEN" "✓ No errors found!"
else
    print_color "$RED" "✗ Found $TOTAL_ERRORS file(s) with errors"
fi

exit $TOTAL_ERRORS