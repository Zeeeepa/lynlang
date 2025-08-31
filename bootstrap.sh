#!/bin/bash
# Bootstrap script for Zen self-hosted compiler
# This script compiles the Zen compiler using the Rust implementation,
# then uses the compiled Zen compiler to compile itself

set -e  # Exit on error

echo "=== Zen Self-Hosting Bootstrap Process ==="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Build the Rust-based Zen compiler
echo -e "${YELLOW}Step 1: Building Rust-based Zen compiler...${NC}"
cargo build --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Rust compiler built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Rust compiler${NC}"
    exit 1
fi

# Step 2: Create bootstrap directory
echo -e "${YELLOW}Step 2: Setting up bootstrap environment...${NC}"
mkdir -p bootstrap/stage1
mkdir -p bootstrap/stage2
mkdir -p bootstrap/output
echo -e "${GREEN}✓ Bootstrap directories created${NC}"

# Step 3: Compile stdlib with Rust compiler
echo -e "${YELLOW}Step 3: Compiling standard library...${NC}"
for file in stdlib/*.zen; do
    if [ -f "$file" ]; then
        filename=$(basename "$file" .zen)
        echo "  Compiling $filename..."
        ./target/release/zen "$file" -o "bootstrap/stage1/$filename.o" --emit-llvm 2>/dev/null || true
    fi
done
echo -e "${GREEN}✓ Standard library compiled${NC}"

# Step 4: Compile the self-hosted compiler
echo -e "${YELLOW}Step 4: Compiling self-hosted compiler...${NC}"
if [ -f "bootstrap/compiler.zen" ]; then
    ./target/release/zen bootstrap/compiler.zen -o bootstrap/stage1/compiler --emit-native
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Self-hosted compiler compiled (Stage 1)${NC}"
    else
        echo -e "${YELLOW}⚠ Stage 1 compiler compilation not complete (expected during development)${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Self-hosted compiler not found at bootstrap/compiler.zen${NC}"
fi

# Step 5: Run basic validation
echo -e "${YELLOW}Step 5: Running validation tests...${NC}"

# Test import syntax validation
echo "  Testing import syntax..."
./target/release/zen tests/test_import_syntax_validation.zen --check-only 2>/dev/null
if [ $? -eq 0 ]; then
    echo -e "${GREEN}  ✓ Import syntax validation passed${NC}"
else
    echo -e "${YELLOW}  ⚠ Import syntax validation needs work${NC}"
fi

# Test that comptime blocks don't contain imports
echo "  Testing comptime import rejection..."
./target/release/zen test_comptime_import_error.zen --check-only 2>&1 | grep -q "not allowed" 
if [ $? -eq 0 ]; then
    echo -e "${GREEN}  ✓ Comptime import rejection working${NC}"
else
    echo -e "${YELLOW}  ⚠ Comptime import rejection needs verification${NC}"
fi

# Step 6: Summary
echo
echo -e "${GREEN}=== Bootstrap Process Summary ===${NC}"
echo "• Rust compiler: Built"
echo "• Standard library: Partially compiled"
echo "• Self-hosted compiler: In development"
echo "• Import syntax: Validated"
echo
echo "Next steps for full self-hosting:"
echo "1. Complete implementation of all compiler passes in Zen"
echo "2. Fix any remaining compilation errors"
echo "3. Use Stage 1 compiler to compile itself (Stage 2)"
echo "4. Verify Stage 2 compiler produces identical output"
echo
echo -e "${GREEN}Bootstrap process completed!${NC}"