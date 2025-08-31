#!/bin/bash

# Zen Language Test Runner
# Comprehensive test suite for the Zen programming language

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Test directories
TEST_DIRS=(
    "examples"
    "tests"
    "stdlib/tests"
)

# Log file for test results
LOG_FILE="test_results_$(date +%Y%m%d_%H%M%S).log"

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to print test header
print_header() {
    echo ""
    print_color "$BLUE" "=========================================="
    print_color "$BLUE" "$1"
    print_color "$BLUE" "=========================================="
}

# Function to compile and run a Zen file
run_zen_test() {
    local test_file=$1
    local test_name=$(basename "$test_file")
    
    ((TOTAL_TESTS++))
    
    # Skip archive files
    if [[ "$test_file" == *"/archive/"* ]]; then
        print_color "$YELLOW" "⊘ SKIPPED: $test_name (archived)"
        ((SKIPPED_TESTS++))
        echo "SKIPPED: $test_file (archived)" >> "$LOG_FILE"
        return 0
    fi
    
    # Try to compile the file
    if ./target/debug/zen "$test_file" > /tmp/zen_output 2>&1; then
        # Check if it's supposed to run
        if grep -q "main\s*=" "$test_file"; then
            # Try to run the compiled output
            if [ -f "output" ]; then
                if ./output > /tmp/zen_run_output 2>&1; then
                    print_color "$GREEN" "✓ PASSED: $test_name"
                    ((PASSED_TESTS++))
                    echo "PASSED: $test_file" >> "$LOG_FILE"
                else
                    print_color "$RED" "✗ FAILED: $test_name (runtime error)"
                    ((FAILED_TESTS++))
                    echo "FAILED: $test_file (runtime error)" >> "$LOG_FILE"
                    cat /tmp/zen_run_output >> "$LOG_FILE"
                fi
                rm -f output
            else
                print_color "$GREEN" "✓ COMPILED: $test_name"
                ((PASSED_TESTS++))
                echo "COMPILED: $test_file" >> "$LOG_FILE"
            fi
        else
            print_color "$GREEN" "✓ PARSED: $test_name (no main)"
            ((PASSED_TESTS++))
            echo "PARSED: $test_file (no main)" >> "$LOG_FILE"
        fi
    else
        print_color "$RED" "✗ FAILED: $test_name (compilation error)"
        ((FAILED_TESTS++))
        echo "FAILED: $test_file (compilation error)" >> "$LOG_FILE"
        cat /tmp/zen_output >> "$LOG_FILE"
    fi
}

# Function to run Rust tests
run_rust_tests() {
    print_header "Running Rust Tests"
    
    if cargo test --quiet 2>&1 | tee -a "$LOG_FILE"; then
        print_color "$GREEN" "✓ All Rust tests passed"
    else
        print_color "$RED" "✗ Some Rust tests failed"
    fi
}

# Function to run syntax checker
run_syntax_check() {
    local file=$1
    local name=$(basename "$file")
    
    ((TOTAL_TESTS++))
    
    if ./zen-check.sh "$file" > /dev/null 2>&1; then
        print_color "$GREEN" "✓ SYNTAX OK: $name"
        ((PASSED_TESTS++))
        echo "SYNTAX OK: $file" >> "$LOG_FILE"
    else
        print_color "$RED" "✗ SYNTAX ERROR: $name"
        ((FAILED_TESTS++))
        echo "SYNTAX ERROR: $file" >> "$LOG_FILE"
    fi
}

# Main test execution
main() {
    print_color "$BLUE" "Starting Zen Language Test Suite"
    print_color "$BLUE" "Log file: $LOG_FILE"
    echo "Test run started at $(date)" > "$LOG_FILE"
    
    # Build the compiler first
    print_header "Building Zen Compiler"
    if cargo build 2>&1 | tee -a "$LOG_FILE"; then
        print_color "$GREEN" "✓ Build successful"
    else
        print_color "$RED" "✗ Build failed"
        exit 1
    fi
    
    # Run Rust tests
    run_rust_tests
    
    # Test Zen files
    print_header "Testing Zen Files"
    
    # Test example files
    if [ -d "examples" ]; then
        print_color "$BLUE" "\nTesting example files..."
        for file in examples/*.zen; do
            if [ -f "$file" ]; then
                run_zen_test "$file"
            fi
        done
    fi
    
    # Test test files
    if [ -d "tests" ]; then
        print_color "$BLUE" "\nTesting test files..."
        for file in tests/*.zen; do
            if [ -f "$file" ]; then
                run_zen_test "$file"
            fi
        done
    fi
    
    # Test stdlib files
    if [ -d "stdlib" ]; then
        print_color "$BLUE" "\nTesting stdlib files..."
        for file in stdlib/*.zen; do
            if [ -f "$file" ]; then
                run_syntax_check "$file"
            fi
        done
    fi
    
    # Print summary
    print_header "Test Summary"
    echo ""
    echo -e "Total Tests:    $TOTAL_TESTS"
    echo -e "${GREEN}Passed:         $PASSED_TESTS${NC}"
    if [ $FAILED_TESTS -gt 0 ]; then
        echo -e "${RED}Failed:         $FAILED_TESTS${NC}"
    else
        echo -e "Failed:         $FAILED_TESTS"
    fi
    if [ $SKIPPED_TESTS -gt 0 ]; then
        echo -e "${YELLOW}Skipped:        $SKIPPED_TESTS${NC}"
    else
        echo -e "Skipped:        $SKIPPED_TESTS"
    fi
    
    # Calculate pass rate
    if [ $TOTAL_TESTS -gt 0 ]; then
        PASS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
        echo -e "Pass Rate:      ${PASS_RATE}%"
    fi
    
    echo ""
    echo "Test results saved to: $LOG_FILE"
    echo ""
    
    # Exit with appropriate code
    if [ $FAILED_TESTS -eq 0 ]; then
        print_color "$GREEN" "✓ All tests passed!"
        exit 0
    else
        print_color "$RED" "✗ Some tests failed!"
        exit 1
    fi
}

# Run main function
main "$@"