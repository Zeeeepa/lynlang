#!/bin/bash

# Unified linting script for Zen Language

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Run cargo fmt
run_fmt() {
    print_header "Checking Rust Formatting"
    if cargo fmt -- --check; then
        print_success "All Rust files are properly formatted"
        return 0
    else
        print_error "Some Rust files need formatting"
        print_warning "Run 'cargo fmt' to fix formatting issues"
        return 1
    fi
}

# Run cargo clippy
run_clippy() {
    print_header "Running Clippy Lints"
    if cargo clippy -- -W clippy::all; then
        print_success "No clippy warnings found"
        return 0
    else
        print_error "Clippy found issues"
        return 1
    fi
}

# Run Zen-specific lints
run_zen_lints() {
    print_header "Checking Zen Files"
    
    local issues=0
    
    # Check for incorrect import syntax
    echo "Checking import syntax..."
    if ./scripts/check-imports.sh >/dev/null 2>&1; then
        print_success "Import syntax is correct"
    else
        print_error "Found incorrect comptime import usage"
        ./scripts/check-imports.sh || true  # Show detailed error
        ((issues++))
    fi
    
    # Check for duplicate Option definitions
    echo "Checking for duplicate type definitions..."
    duplicates=$(grep -l "Option<T>: Some(T) | None" tests/*.zen 2>/dev/null | wc -l)
    if [ "$duplicates" -gt 0 ]; then
        print_warning "Found $duplicates files with duplicate Option definitions (should import from stdlib)"
    fi
    
    if [ $issues -eq 0 ]; then
        print_success "All Zen lints passed"
        return 0
    else
        print_error "Found $issues linting issues in Zen files"
        return 1
    fi
}

# Main
main() {
    local fmt_result=0
    local clippy_result=0
    local zen_result=0
    
    print_header "Zen Language Linter"
    
    # Run all linters
    run_fmt || fmt_result=$?
    run_clippy || clippy_result=$?
    run_zen_lints || zen_result=$?
    
    # Summary
    print_header "Linting Summary"
    
    if [ $fmt_result -eq 0 ] && [ $clippy_result -eq 0 ] && [ $zen_result -eq 0 ]; then
        print_success "All linting checks passed!"
        exit 0
    else
        print_error "Some linting checks failed"
        [ $fmt_result -ne 0 ] && print_error "  - Rust formatting"
        [ $clippy_result -ne 0 ] && print_error "  - Clippy warnings"
        [ $zen_result -ne 0 ] && print_error "  - Zen file issues"
        exit 1
    fi
}

main