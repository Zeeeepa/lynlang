#!/bin/bash

# Unified test runner for Zen Language
# Runs both Rust tests and Zen language tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test mode (all, rust, zen)
MODE="${1:-all}"

print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Build the compiler
build_compiler() {
    print_header "Building Zen Compiler"
    if cargo build --release; then
        print_success "Compiler built successfully"
        return 0
    else
        print_error "Build failed"
        return 1
    fi
}

# Run Rust tests
run_rust_tests() {
    print_header "Running Rust Tests"
    if cargo test --verbose; then
        print_success "All Rust tests passed"
        return 0
    else
        print_error "Some Rust tests failed"
        return 1
    fi
}

# Run Zen language tests
run_zen_tests() {
    print_header "Running Zen Language Tests"
    
    local passed=0
    local failed=0
    local total=0
    
    # Find all zen test files
    for test_file in tests/zen_test_*.zen; do
        if [ ! -f "$test_file" ]; then
            continue
        fi
        
        ((total++))
        test_name=$(basename "$test_file" .zen)
        
        # Skip archived or work-in-progress tests
        if [[ "$test_file" == *"_wip.zen" ]] || [[ "$test_file" == *"_broken.zen" ]]; then
            print_info "Skipping $test_name"
            continue
        fi
        
        # Try to compile
        if ./target/release/zen "$test_file" -o "/tmp/${test_name}" 2>/dev/null; then
            # Try to run if it has a main function
            if grep -q "main\s*=" "$test_file"; then
                if "/tmp/${test_name}" > /dev/null 2>&1; then
                    print_success "$test_name"
                    ((passed++))
                else
                    print_error "$test_name (runtime error)"
                    ((failed++))
                fi
                rm -f "/tmp/${test_name}"
            else
                print_success "$test_name (compiled)"
                ((passed++))
            fi
        else
            print_error "$test_name (compilation error)"
            ((failed++))
        fi
    done
    
    print_header "Zen Test Summary"
    echo "Total:  $total"
    echo "Passed: $passed"
    echo "Failed: $failed"
    
    if [ $failed -eq 0 ]; then
        print_success "All Zen tests passed!"
        return 0
    else
        print_error "Some Zen tests failed"
        return 1
    fi
}

# Main
main() {
    local rust_result=0
    local zen_result=0
    
    case "$MODE" in
        rust)
            build_compiler || exit 1
            run_rust_tests
            rust_result=$?
            ;;
        zen)
            build_compiler || exit 1
            run_zen_tests
            zen_result=$?
            ;;
        all|*)
            build_compiler || exit 1
            run_rust_tests
            rust_result=$?
            run_zen_tests
            zen_result=$?
            ;;
    esac
    
    # Exit with failure if any tests failed
    if [ $rust_result -ne 0 ] || [ $zen_result -ne 0 ]; then
        exit 1
    fi
    
    exit 0
}

main