#!/bin/bash
# Enhanced Zen Syntax and Semantic Checker
# Provides comprehensive checking for Zen source files

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=false
CHECK_IMPORTS=true
CHECK_TYPES=true
CHECK_STYLE=true
CHECK_ALL=false
OUTPUT_FORMAT="text"
FIX_IMPORTS=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --no-imports)
            CHECK_IMPORTS=false
            shift
            ;;
        --no-types)
            CHECK_TYPES=false
            shift
            ;;
        --no-style)
            CHECK_STYLE=false
            shift
            ;;
        --all|-a)
            CHECK_ALL=true
            shift
            ;;
        --json)
            OUTPUT_FORMAT="json"
            shift
            ;;
        --fix-imports)
            FIX_IMPORTS=true
            shift
            ;;
        --help|-h)
            echo "Enhanced Zen Syntax and Semantic Checker"
            echo "========================================="
            echo ""
            echo "Usage: $0 [options] <file.zen|directory>"
            echo ""
            echo "Options:"
            echo "  -v, --verbose       Show detailed output"
            echo "  --no-imports        Skip import validation"
            echo "  --no-types          Skip type checking"
            echo "  --no-style          Skip style checking"
            echo "  -a, --all           Check all .zen files in directory"
            echo "  --json              Output results in JSON format"
            echo "  --fix-imports       Automatically fix import issues"
            echo "  -h, --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 hello.zen"
            echo "  $0 --all src/"
            echo "  $0 --verbose --json src/main.zen"
            echo "  $0 --fix-imports examples/"
            exit 0
            ;;
        *)
            FILE_OR_DIR="$1"
            shift
            ;;
    esac
done

# Check if file or directory provided
if [ -z "$FILE_OR_DIR" ]; then
    echo -e "${RED}Error: No file or directory specified${NC}"
    echo "Use --help for usage information"
    exit 1
fi

# Function to check a single file
check_file() {
    local file="$1"
    local errors=0
    local warnings=0
    
    if [ "$OUTPUT_FORMAT" == "json" ]; then
        echo "{"
        echo "  \"file\": \"$file\","
        echo "  \"checks\": ["
    else
        echo -e "${BLUE}Checking:${NC} $file"
        echo "─────────────────────────────────────"
    fi
    
    # 1. Syntax check
    if [ "$VERBOSE" == true ]; then
        echo -e "${CYAN}→ Syntax check...${NC}"
    fi
    
    syntax_output=$(cargo run --bin zen -- --check "$file" 2>&1 | grep -v "warning: unused" | grep -v "Compiling")
    syntax_exit_code=$?
    
    if [ $syntax_exit_code -ne 0 ]; then
        if [ "$OUTPUT_FORMAT" == "json" ]; then
            echo "    {\"type\": \"syntax\", \"severity\": \"error\", \"message\": \"Syntax errors found\"}"
        else
            echo -e "${RED}✗ Syntax errors found${NC}"
            if [ "$VERBOSE" == true ]; then
                echo "$syntax_output"
            fi
        fi
        ((errors++))
    else
        if [ "$OUTPUT_FORMAT" != "json" ] && [ "$VERBOSE" == true ]; then
            echo -e "${GREEN}✓ Syntax check passed${NC}"
        fi
    fi
    
    # 2. Import validation
    if [ "$CHECK_IMPORTS" == true ]; then
        if [ "$VERBOSE" == true ]; then
            echo -e "${CYAN}→ Import validation...${NC}"
        fi
        
        # Check for comptime imports (should not be used for imports)
        if grep -q "comptime.*@std" "$file"; then
            if [ "$OUTPUT_FORMAT" == "json" ]; then
                echo "    ,{\"type\": \"import\", \"severity\": \"error\", \"message\": \"Comptime should not be used for imports\"}"
            else
                echo -e "${RED}✗ Found comptime imports (should be module-level)${NC}"
            fi
            ((errors++))
            
            # Fix imports if requested
            if [ "$FIX_IMPORTS" == true ]; then
                echo -e "${YELLOW}→ Fixing imports...${NC}"
                sed -i.bak 's/comptime {.*@std\.\([^}]*\)}/\1 := @std.\1/g' "$file"
                echo -e "${GREEN}✓ Imports fixed${NC}"
            fi
        fi
        
        # Check for proper import syntax
        import_count=$(grep -c "^[a-zA-Z_][a-zA-Z0-9_]* := @std\." "$file")
        build_import_count=$(grep -c "^[a-zA-Z_][a-zA-Z0-9_]* := .*\.import(" "$file")
        
        if [ "$VERBOSE" == true ] && [ "$OUTPUT_FORMAT" != "json" ]; then
            echo "  Found $import_count standard imports"
            echo "  Found $build_import_count build imports"
        fi
    fi
    
    # 3. Type checking
    if [ "$CHECK_TYPES" == true ]; then
        if [ "$VERBOSE" == true ]; then
            echo -e "${CYAN}→ Type checking...${NC}"
        fi
        
        # Run type checker
        type_output=$(cargo run --bin zen -- --type-check "$file" 2>&1 | grep -v "warning: unused" | grep -v "Compiling")
        type_exit_code=$?
        
        if [ $type_exit_code -ne 0 ]; then
            if [ "$OUTPUT_FORMAT" == "json" ]; then
                echo "    ,{\"type\": \"type\", \"severity\": \"error\", \"message\": \"Type errors found\"}"
            else
                echo -e "${RED}✗ Type errors found${NC}"
                if [ "$VERBOSE" == true ]; then
                    echo "$type_output"
                fi
            fi
            ((errors++))
        else
            if [ "$OUTPUT_FORMAT" != "json" ] && [ "$VERBOSE" == true ]; then
                echo -e "${GREEN}✓ Type check passed${NC}"
            fi
        fi
    fi
    
    # 4. Style checking
    if [ "$CHECK_STYLE" == true ]; then
        if [ "$VERBOSE" == true ]; then
            echo -e "${CYAN}→ Style checking...${NC}"
        fi
        
        # Check naming conventions
        if grep -q "^[A-Z][a-zA-Z0-9_]* = " "$file"; then
            if [ "$OUTPUT_FORMAT" == "json" ]; then
                echo "    ,{\"type\": \"style\", \"severity\": \"warning\", \"message\": \"Function names should be snake_case\"}"
            else
                echo -e "${YELLOW}⚠ Function names should be snake_case${NC}"
            fi
            ((warnings++))
        fi
        
        # Check struct naming
        if grep -q "^[a-z][a-zA-Z0-9_]* := struct" "$file"; then
            if [ "$OUTPUT_FORMAT" == "json" ]; then
                echo "    ,{\"type\": \"style\", \"severity\": \"warning\", \"message\": \"Struct names should be PascalCase\"}"
            else
                echo -e "${YELLOW}⚠ Struct names should be PascalCase${NC}"
            fi
            ((warnings++))
        fi
        
        # Check line length
        long_lines=$(awk 'length > 100' "$file" | wc -l)
        if [ $long_lines -gt 0 ]; then
            if [ "$OUTPUT_FORMAT" == "json" ]; then
                echo "    ,{\"type\": \"style\", \"severity\": \"warning\", \"message\": \"$long_lines lines exceed 100 characters\"}"
            else
                echo -e "${YELLOW}⚠ $long_lines lines exceed 100 characters${NC}"
            fi
            ((warnings++))
        fi
    fi
    
    # 5. Additional checks
    if [ "$VERBOSE" == true ]; then
        echo -e "${CYAN}→ Additional checks...${NC}"
    fi
    
    # Check for TODO comments
    todo_count=$(grep -c "TODO\|FIXME\|XXX" "$file")
    if [ $todo_count -gt 0 ]; then
        if [ "$OUTPUT_FORMAT" == "json" ]; then
            echo "    ,{\"type\": \"todo\", \"severity\": \"info\", \"message\": \"Found $todo_count TODO/FIXME comments\"}"
        else
            echo -e "${CYAN}ℹ Found $todo_count TODO/FIXME comments${NC}"
        fi
    fi
    
    # Check for unused variables (simple check)
    unused_vars=$(grep -o "[a-zA-Z_][a-zA-Z0-9_]* :=" "$file" | cut -d' ' -f1 | sort | uniq -c | awk '$1==1 {print $2}')
    if [ ! -z "$unused_vars" ]; then
        unused_count=$(echo "$unused_vars" | wc -l)
        if [ "$OUTPUT_FORMAT" == "json" ]; then
            echo "    ,{\"type\": \"unused\", \"severity\": \"warning\", \"message\": \"Potentially $unused_count unused variables\"}"
        else
            echo -e "${YELLOW}⚠ Potentially $unused_count unused variables${NC}"
        fi
        ((warnings++))
    fi
    
    # Output summary
    if [ "$OUTPUT_FORMAT" == "json" ]; then
        echo "  ],"
        echo "  \"summary\": {"
        echo "    \"errors\": $errors,"
        echo "    \"warnings\": $warnings"
        echo "  }"
        echo "}"
    else
        echo "─────────────────────────────────────"
        if [ $errors -eq 0 ] && [ $warnings -eq 0 ]; then
            echo -e "${GREEN}✓ All checks passed!${NC}"
        elif [ $errors -eq 0 ]; then
            echo -e "${YELLOW}⚠ $warnings warnings found${NC}"
        else
            echo -e "${RED}✗ $errors errors, $warnings warnings${NC}"
        fi
        echo ""
    fi
    
    return $errors
}

# Function to check directory
check_directory() {
    local dir="$1"
    local total_errors=0
    local total_warnings=0
    local file_count=0
    
    echo -e "${BLUE}Checking directory:${NC} $dir"
    echo "═════════════════════════════════════"
    echo ""
    
    # Find all .zen files
    while IFS= read -r -d '' file; do
        check_file "$file"
        total_errors=$((total_errors + $?))
        ((file_count++))
    done < <(find "$dir" -name "*.zen" -type f -print0)
    
    # Summary
    echo "═════════════════════════════════════"
    echo -e "${BLUE}Summary:${NC}"
    echo "  Files checked: $file_count"
    echo "  Total errors: $total_errors"
    echo ""
    
    if [ $total_errors -eq 0 ]; then
        echo -e "${GREEN}✓ All files passed!${NC}"
        exit 0
    else
        echo -e "${RED}✗ Some files have errors${NC}"
        exit 1
    fi
}

# Main execution
if [ -f "$FILE_OR_DIR" ]; then
    # Single file
    check_file "$FILE_OR_DIR"
    exit $?
elif [ -d "$FILE_OR_DIR" ]; then
    # Directory
    if [ "$CHECK_ALL" == true ]; then
        check_directory "$FILE_OR_DIR"
    else
        echo -e "${RED}Error: $FILE_OR_DIR is a directory. Use --all to check all files.${NC}"
        exit 1
    fi
else
    echo -e "${RED}Error: $FILE_OR_DIR not found${NC}"
    exit 1
fi