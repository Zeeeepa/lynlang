#!/bin/bash
# Zen Language Enhanced Linter and Syntax Checker
# Provides comprehensive syntax checking, import validation, and code quality analysis

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Options
VERBOSE=false
CHECK_STYLE=true
CHECK_IMPORTS=true
CHECK_TYPES=true
CHECK_SECURITY=false
FIX_IMPORTS=false
OUTPUT_FORMAT="text"  # text, json, github
ERROR_CONTEXT_LINES=2

# Counters
TOTAL_ERRORS=0
TOTAL_WARNINGS=0
TOTAL_INFO=0
TOTAL_FILES=0

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    if [ "$OUTPUT_FORMAT" = "text" ]; then
        echo -e "${color}${message}${NC}"
    else
        echo "$message"
    fi
}

# Function to report issue in GitHub format
report_github_issue() {
    local level=$1  # error, warning, notice
    local file=$2
    local line=$3
    local col=$4
    local message=$5
    
    if [ "$OUTPUT_FORMAT" = "github" ]; then
        echo "::${level} file=${file},line=${line},col=${col}::${message}"
    fi
}

# Function to check file syntax with detailed error reporting
check_syntax() {
    local file=$1
    local filename=$(basename "$file")
    
    # Run the compiler in check mode
    local output
    if [ -f "./build/zenc" ]; then
        output=$(./build/zenc "$file" --check-only 2>&1 || true)
    elif [ -f "./target/release/zen" ]; then
        output=$(./target/release/zen "$file" --emit-ir 2>&1 | head -100 || true)
    else
        print_color "$YELLOW" "⚠ Compiler not found, building..."
        cargo build --release --quiet
        output=$(./target/release/zen "$file" --emit-ir 2>&1 | head -100 || true)
    fi
    
    # Parse errors with line numbers
    if echo "$output" | grep -q "error\|Error\|failed\|Failed"; then
        print_color "$RED" "✗ Syntax errors in $filename:"
        
        # Extract error lines with context
        echo "$output" | grep -E "error|Error" | while read -r error_line; do
            # Try to extract line number from error message
            if [[ "$error_line" =~ :([0-9]+):([0-9]+) ]]; then
                local line_num="${BASH_REMATCH[1]}"
                local col_num="${BASH_REMATCH[2]}"
                report_github_issue "error" "$file" "$line_num" "$col_num" "$error_line"
                
                if [ "$VERBOSE" = true ]; then
                    # Show code context
                    print_color "$CYAN" "  Line $line_num:"
                    sed -n "$((line_num-ERROR_CONTEXT_LINES)),$((line_num+ERROR_CONTEXT_LINES))p" "$file" | \
                        awk -v ln="$line_num" -v ctx="$ERROR_CONTEXT_LINES" \
                        'NR==ctx+1 {print "  > " $0} NR!=ctx+1 {print "    " $0}'
                fi
            fi
            echo "  $error_line"
        done
        ((TOTAL_ERRORS++))
        return 1
    else
        if [ "$VERBOSE" = true ]; then
            print_color "$GREEN" "✓ Syntax OK: $filename"
        fi
        return 0
    fi
}

# Enhanced import checking
check_imports() {
    local file=$1
    local filename=$(basename "$file")
    local issues=0
    
    # Check for comptime-wrapped imports (now invalid)
    local comptime_imports=$(grep -n "comptime.*{[^}]*@std" "$file" 2>/dev/null || true)
    if [ ! -z "$comptime_imports" ]; then
        print_color "$RED" "✗ Invalid comptime imports in $filename:"
        echo "$comptime_imports" | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            local content=$(echo "$line" | cut -d: -f2-)
            print_color "$RED" "  Line $line_num: $content"
            report_github_issue "error" "$file" "$line_num" "1" "Imports must not be wrapped in comptime blocks"
            
            if [ "$FIX_IMPORTS" = true ]; then
                print_color "$CYAN" "  Suggested fix: Move import to module level"
            fi
        done
        ((issues++))
        ((TOTAL_ERRORS++))
    fi
    
    # Check import ordering
    local import_lines=$(grep -n "^[[:space:]]*[a-z_][a-zA-Z0-9_]* := @std\." "$file" 2>/dev/null | cut -d: -f1 || true)
    if [ ! -z "$import_lines" ]; then
        local last_import=0
        local first_non_import=0
        
        # Find first non-import, non-comment line
        while IFS= read -r line_num; do
            local line_content=$(sed -n "${line_num}p" "$file")
            if [[ ! "$line_content" =~ ^[[:space:]]*// ]] && \
               [[ ! "$line_content" =~ ^[[:space:]]*$ ]] && \
               [[ ! "$line_content" =~ "@std\." ]]; then
                first_non_import=$line_num
                break
            fi
        done < <(seq 1 $(wc -l < "$file"))
        
        # Check if imports come after code
        for import_line in $import_lines; do
            if [ $first_non_import -gt 0 ] && [ $import_line -gt $first_non_import ]; then
                print_color "$YELLOW" "⚠ Import at line $import_line comes after code (first code at line $first_non_import)"
                report_github_issue "warning" "$file" "$import_line" "1" "Import should be at module level before code"
                ((issues++))
                ((TOTAL_WARNINGS++))
            fi
        done
    fi
    
    # Check for duplicate imports
    local imports=$(grep "^[[:space:]]*[a-z_][a-zA-Z0-9_]* := @std\." "$file" 2>/dev/null | sed 's/^[[:space:]]*//' | cut -d' ' -f1 || true)
    if [ ! -z "$imports" ]; then
        local duplicates=$(echo "$imports" | sort | uniq -d)
        if [ ! -z "$duplicates" ]; then
            print_color "$YELLOW" "⚠ Duplicate imports found in $filename:"
            echo "$duplicates" | while read -r dup; do
                echo "  $dup imported multiple times"
                ((TOTAL_WARNINGS++))
            done
            ((issues++))
        fi
    fi
    
    # Check for unused imports (heuristic)
    if [ "$VERBOSE" = true ] && [ ! -z "$imports" ]; then
        echo "$imports" | while read -r import_name; do
            # Check if import is used elsewhere in file
            local usage_count=$(grep -c "\b${import_name}\." "$file" 2>/dev/null || echo "0")
            if [ "$usage_count" -eq 1 ]; then
                print_color "$YELLOW" "⚠ Potentially unused import: $import_name"
                ((TOTAL_INFO++))
            fi
        done
    fi
    
    if [ $issues -eq 0 ] && [ "$VERBOSE" = true ]; then
        print_color "$GREEN" "✓ Imports OK: $filename"
    fi
    
    return $issues
}

# Enhanced style checking
check_style() {
    local file=$1
    local filename=$(basename "$file")
    local warnings=0
    
    # Line length check
    local long_lines=$(awk 'length > 100 {print NR ": " substr($0, 1, 100) "..."}' "$file" | head -5)
    if [ ! -z "$long_lines" ]; then
        print_color "$YELLOW" "⚠ Lines exceeding 100 characters in $filename:"
        echo "$long_lines" | while read -r line; do
            echo "  $line"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Consistent indentation check
    local has_tabs=$(grep -c $'\t' "$file" 2>/dev/null || echo "0")
    local has_spaces=$(grep -c '^    ' "$file" 2>/dev/null || echo "0")
    
    if [ $has_tabs -gt 0 ] && [ $has_spaces -gt 0 ]; then
        print_color "$YELLOW" "⚠ Mixed tabs and spaces for indentation in $filename"
        ((warnings++))
        ((TOTAL_WARNINGS++))
    elif [ $has_tabs -gt 0 ] && [ "$VERBOSE" = true ]; then
        print_color "$YELLOW" "⚠ Using tabs for indentation (consider spaces) in $filename"
        ((TOTAL_INFO++))
    fi
    
    # Trailing whitespace
    local trailing=$(grep -n '[[:space:]]$' "$file" 2>/dev/null | head -3 || true)
    if [ ! -z "$trailing" ]; then
        print_color "$YELLOW" "⚠ Trailing whitespace in $filename:"
        echo "$trailing" | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num"
            report_github_issue "warning" "$file" "$line_num" "999" "Trailing whitespace"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Missing final newline
    if [ -n "$(tail -c 1 "$file" 2>/dev/null)" ]; then
        print_color "$YELLOW" "⚠ No newline at end of file in $filename"
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Check naming conventions
    # Functions should be snake_case
    local bad_func_names=$(grep -n "^[A-Z][a-zA-Z0-9_]* = (" "$file" 2>/dev/null || true)
    if [ ! -z "$bad_func_names" ]; then
        print_color "$YELLOW" "⚠ Function names should be snake_case in $filename:"
        echo "$bad_func_names" | head -3 | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Types should be PascalCase
    local bad_type_names=$(grep -n "^[a-z][a-zA-Z0-9_]* = {$" "$file" 2>/dev/null || true)
    if [ ! -z "$bad_type_names" ]; then
        print_color "$YELLOW" "⚠ Type names should be PascalCase in $filename:"
        echo "$bad_type_names" | head -3 | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    if [ $warnings -eq 0 ] && [ "$VERBOSE" = true ]; then
        print_color "$GREEN" "✓ Style OK: $filename"
    fi
    
    return $warnings
}

# Enhanced type checking
check_types() {
    local file=$1
    local filename=$(basename "$file")
    local warnings=0
    
    # Check for missing return types
    local untyped_funcs=$(grep -n "^[a-z_][a-zA-Z0-9_]* = ([^)]*) {" "$file" 2>/dev/null || true)
    if [ ! -z "$untyped_funcs" ]; then
        print_color "$YELLOW" "⚠ Functions without explicit return types in $filename:"
        echo "$untyped_funcs" | head -3 | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            local content=$(echo "$line" | cut -d: -f2- | head -c 60)
            echo "  Line $line_num: $content..."
            report_github_issue "warning" "$file" "$line_num" "1" "Function missing explicit return type"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Check for untyped variables (heuristic)
    local untyped_vars=$(grep -n "^[[:space:]]*[a-z_][a-zA-Z0-9_]* := " "$file" 2>/dev/null | \
                         grep -v "@std\." | grep -v "\." | head -5 || true)
    if [ ! -z "$untyped_vars" ] && [ "$VERBOSE" = true ]; then
        print_color "$CYAN" "ℹ Variables relying on type inference in $filename:"
        echo "$untyped_vars" | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num (consider explicit type annotation)"
        done
        ((TOTAL_INFO++))
    fi
    
    if [ $warnings -eq 0 ] && [ "$VERBOSE" = true ]; then
        print_color "$GREEN" "✓ Types OK: $filename"
    fi
    
    return $warnings
}

# Security checks (optional)
check_security() {
    local file=$1
    local filename=$(basename "$file")
    local warnings=0
    
    # Check for unsafe operations
    local unsafe_ops=$(grep -n "@unsafe\|unsafe\|transmute\|raw_ptr" "$file" 2>/dev/null || true)
    if [ ! -z "$unsafe_ops" ]; then
        print_color "$YELLOW" "⚠ Potentially unsafe operations in $filename:"
        echo "$unsafe_ops" | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num"
            report_github_issue "warning" "$file" "$line_num" "1" "Potentially unsafe operation"
        done
        ((warnings++))
        ((TOTAL_WARNINGS++))
    fi
    
    # Check for hardcoded secrets (basic pattern matching)
    local potential_secrets=$(grep -n -E "(password|secret|token|key)[[:space:]]*[:=][[:space:]]*\"[^\"]+\"" "$file" 2>/dev/null || true)
    if [ ! -z "$potential_secrets" ]; then
        print_color "$RED" "✗ Potential hardcoded secrets in $filename:"
        echo "$potential_secrets" | while read -r line; do
            local line_num=$(echo "$line" | cut -d: -f1)
            echo "  Line $line_num"
            report_github_issue "error" "$file" "$line_num" "1" "Potential hardcoded secret"
        done
        ((warnings++))
        ((TOTAL_ERRORS++))
    fi
    
    return $warnings
}

# Function to lint a single file
lint_file() {
    local file=$1
    local filename=$(basename "$file")
    local file_errors=0
    local file_warnings=0
    
    if [ "$VERBOSE" = true ] || [ "$OUTPUT_FORMAT" = "text" ]; then
        print_color "$BLUE" "\nChecking: $file"
        print_color "$BLUE" "----------------------------------------"
    fi
    
    # Syntax check
    if ! check_syntax "$file"; then
        ((file_errors++))
    fi
    
    # Import check
    if [ "$CHECK_IMPORTS" = true ]; then
        check_imports "$file" || true
    fi
    
    # Style check
    if [ "$CHECK_STYLE" = true ]; then
        check_style "$file" || true
    fi
    
    # Type check
    if [ "$CHECK_TYPES" = true ]; then
        check_types "$file" || true
    fi
    
    # Security check
    if [ "$CHECK_SECURITY" = true ]; then
        check_security "$file" || true
    fi
    
    # File summary
    if [ "$VERBOSE" = true ]; then
        if [ $TOTAL_ERRORS -eq 0 ] && [ $TOTAL_WARNINGS -eq 0 ]; then
            print_color "$GREEN" "✓ $filename: All checks passed"
        else
            local msg="$filename: "
            [ $TOTAL_ERRORS -gt 0 ] && msg+="$TOTAL_ERRORS error(s) "
            [ $TOTAL_WARNINGS -gt 0 ] && msg+="$TOTAL_WARNINGS warning(s) "
            [ $TOTAL_INFO -gt 0 ] && msg+="$TOTAL_INFO info"
            print_color "$CYAN" "$msg"
        fi
    fi
    
    return $file_errors
}

# Function to show usage
show_usage() {
    echo "Zen Language Enhanced Linter"
    echo "============================"
    echo ""
    echo "Usage: $0 [OPTIONS] <file.zen|directory>"
    echo ""
    echo "Options:"
    echo "  -v, --verbose         Show detailed output"
    echo "  -q, --quiet           Minimal output"
    echo "  -s, --no-style        Skip style checking"
    echo "  -i, --no-imports      Skip import checking"
    echo "  -t, --no-types        Skip type checking"
    echo "  -S, --security        Enable security checks"
    echo "  -f, --fix             Attempt to fix issues (experimental)"
    echo "  -c, --context N       Show N lines of context for errors (default: 2)"
    echo "  -o, --output FORMAT   Output format: text|json|github (default: text)"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 hello.zen                       # Lint a single file"
    echo "  $0 -v examples/                    # Lint directory with verbose output"
    echo "  $0 -S --output github src/          # GitHub Actions compatible output"
    echo "  $0 --fix stdlib/io.zen             # Lint and attempt fixes"
    echo ""
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -q|--quiet)
            VERBOSE=false
            shift
            ;;
        -s|--no-style)
            CHECK_STYLE=false
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
        -S|--security)
            CHECK_SECURITY=true
            shift
            ;;
        -f|--fix)
            FIX_IMPORTS=true
            shift
            ;;
        -c|--context)
            ERROR_CONTEXT_LINES="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
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

# Main execution
if [ "$OUTPUT_FORMAT" = "text" ]; then
    print_color "$CYAN" "Zen Enhanced Linter v0.2.0"
    print_color "$CYAN" "=========================="
fi

# Check if target is a file or directory
if [ -f "$TARGET" ]; then
    # Single file
    lint_file "$TARGET"
    TOTAL_FILES=1
elif [ -d "$TARGET" ]; then
    # Directory
    if [ "$OUTPUT_FORMAT" = "text" ]; then
        print_color "$BLUE" "Linting directory: $TARGET"
    fi
    
    # Find all .zen files
    while IFS= read -r -d '' file; do
        # Skip archive and build directories
        if [[ "$file" == *"/archive/"* ]] || [[ "$file" == *"/build/"* ]]; then
            continue
        fi
        
        lint_file "$file" || true
        ((TOTAL_FILES++))
    done < <(find "$TARGET" -name "*.zen" -type f -print0)
else
    print_color "$RED" "Error: '$TARGET' is not a valid file or directory"
    exit 1
fi

# Final summary
if [ "$OUTPUT_FORMAT" = "text" ]; then
    echo ""
    print_color "$CYAN" "======================================="
    print_color "$CYAN" "Linting Summary"
    print_color "$CYAN" "======================================="
    echo "Files checked: $TOTAL_FILES"
    [ $TOTAL_ERRORS -gt 0 ] && print_color "$RED" "Errors: $TOTAL_ERRORS"
    [ $TOTAL_WARNINGS -gt 0 ] && print_color "$YELLOW" "Warnings: $TOTAL_WARNINGS"
    [ "$VERBOSE" = true ] && [ $TOTAL_INFO -gt 0 ] && print_color "$CYAN" "Info: $TOTAL_INFO"
    
    if [ $TOTAL_ERRORS -eq 0 ]; then
        print_color "$GREEN" "✓ No errors found!"
        exit 0
    else
        print_color "$RED" "✗ Linting failed with $TOTAL_ERRORS error(s)"
        exit 1
    fi
elif [ "$OUTPUT_FORMAT" = "json" ]; then
    # Output JSON summary
    echo "{\"files\":$TOTAL_FILES,\"errors\":$TOTAL_ERRORS,\"warnings\":$TOTAL_WARNINGS,\"info\":$TOTAL_INFO}"
    [ $TOTAL_ERRORS -eq 0 ] && exit 0 || exit 1
else
    # GitHub format - summary is in annotations
    [ $TOTAL_ERRORS -eq 0 ] && exit 0 || exit 1
fi