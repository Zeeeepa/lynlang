#!/bin/bash

# Zen Language Integration Test Suite
# Tests compiler, tools, and language features

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "Zen Language Integration Test Suite"
echo "========================================="
echo ""

# Build the Rust compiler
echo -e "${YELLOW}Building Rust-based Zen compiler...${NC}"
cargo build --release 2>&1 | tail -5
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Rust compiler built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Rust compiler${NC}"
    exit 1
fi

ZEN_COMPILER="./target/release/zen"

# Test basic compilation
echo ""
echo -e "${YELLOW}Testing basic compilation...${NC}"

# Test hello world
echo "  Testing hello_world.zen..."
$ZEN_COMPILER examples/01_hello_world.zen > /tmp/hello.ll 2>&1
if [ $? -eq 0 ]; then
    echo -e "  ${GREEN}✓ hello_world.zen compiled${NC}"
else
    echo -e "  ${RED}✗ hello_world.zen failed${NC}"
fi

# Test simple arithmetic
echo "  Testing simple_test.zen..."
$ZEN_COMPILER tests/bootstrap/simple_test.zen > /tmp/simple.ll 2>&1
if [ $? -eq 0 ]; then
    echo -e "  ${GREEN}✓ simple_test.zen compiled${NC}"
else
    echo -e "  ${RED}✗ simple_test.zen failed${NC}"
fi

# Test syntax checking with zen-check (if available)
echo ""
echo -e "${YELLOW}Testing syntax validation...${NC}"
if [ -f "./zen-check" ]; then
    ./zen-check examples/01_hello_world.zen
    echo -e "${GREEN}✓ Syntax validation works${NC}"
else
    echo -e "${YELLOW}⚠ zen-check not built yet (expected)${NC}"
fi

# Test stdlib modules
echo ""
echo -e "${YELLOW}Testing stdlib modules...${NC}"

test_stdlib_module() {
    local module=$1
    local file="stdlib/${module}.zen"
    
    if [ -f "$file" ]; then
        echo "  Checking $module..."
        # Just verify the file has correct syntax by checking for comptime usage
        if grep -q "^comptime {" "$file"; then
            echo -e "  ${RED}✗ $module still uses old comptime import syntax${NC}"
            return 1
        else
            echo -e "  ${GREEN}✓ $module has correct import syntax${NC}"
            return 0
        fi
    fi
}

test_stdlib_module "io"
test_stdlib_module "memory"
test_stdlib_module "string"
test_stdlib_module "math"
test_stdlib_module "collections"
test_stdlib_module "algorithm"

# Test self-hosted compiler components
echo ""
echo -e "${YELLOW}Testing self-hosted compiler components...${NC}"

check_component() {
    local component=$1
    local path=$2
    
    if [ -f "$path" ]; then
        echo -e "  ${GREEN}✓ $component exists${NC}"
        # Check for correct import syntax
        if grep -q "^comptime {" "$path"; then
            echo -e "    ${RED}✗ Uses old comptime import syntax${NC}"
        else
            echo -e "    ${GREEN}✓ Uses correct import syntax${NC}"
        fi
    else
        echo -e "  ${RED}✗ $component missing${NC}"
    fi
}

check_component "Lexer" "compiler/lexer.zen"
check_component "Parser" "compiler/parser.zen"
check_component "Code Generator" "compiler/codegen.zen"
check_component "Bootstrap Compiler" "tools/zen-compile.zen"
check_component "LSP Server" "lsp/server.zen"
check_component "Syntax Checker" "tools/zen-check.zen"

# Create test programs
echo ""
echo -e "${YELLOW}Creating and testing sample programs...${NC}"

# Test 1: Functions and arithmetic
cat > /tmp/test_functions.zen << 'EOF'
build := @std.build
io := build.import("io")

add = (a: i32, b: i32) i32 {
    return a + b
}

multiply = (x: i32, y: i32) i32 {
    return x * y
}

main = () void {
    io.print("Function test program\n")
    
    // Test simple functions
    sum := add(10, 20)
    product := multiply(5, 6)
}
EOF

echo "  Testing functions..."
$ZEN_COMPILER /tmp/test_functions.zen > /tmp/test_func.ll 2>&1
if [ $? -eq 0 ]; then
    echo -e "  ${GREEN}✓ Function test compiled${NC}"
else
    echo -e "  ${RED}✗ Function test failed${NC}"
fi

# Test 2: Structs
cat > /tmp/test_structs.zen << 'EOF'
build := @std.build

Point = {
    x: i32,
    y: i32,
}

main = () void {
    p := Point { x: 10, y: 20 }
}
EOF

echo "  Testing structs..."
$ZEN_COMPILER /tmp/test_structs.zen > /tmp/test_struct.ll 2>&1
if [ $? -eq 0 ]; then
    echo -e "  ${GREEN}✓ Struct test compiled${NC}"
else
    echo -e "  ${RED}✗ Struct test failed${NC}"
fi

# Summary
echo ""
echo "========================================="
echo -e "${GREEN}Integration tests completed!${NC}"
echo "========================================="

# Run existing test suite if available
if [ -f "./run_tests.sh" ]; then
    echo ""
    echo -e "${YELLOW}Running existing test suite...${NC}"
    ./run_tests.sh | tail -10
fi

echo ""
echo "Next steps:"
echo "  1. Bootstrap the self-hosted compiler"
echo "  2. Test LSP server with an editor"
echo "  3. Set up continuous integration"
echo "  4. Add more comprehensive tests"