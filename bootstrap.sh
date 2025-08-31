#!/bin/bash
# Zen Language Bootstrap Script
# Bootstraps the self-hosted Zen compiler

set -e

echo "========================================="
echo "Zen Language Self-Hosting Bootstrap"
echo "========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[i]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the root of the Zen repository"
    exit 1
fi

# Step 1: Build the Rust-based compiler
print_info "Step 1: Building Rust-based compiler..."
cargo build --release
if [ $? -eq 0 ]; then
    print_status "Rust compiler built successfully"
else
    print_error "Failed to build Rust compiler"
    exit 1
fi

# Step 2: Create build directory
print_info "Step 2: Setting up build directory..."
mkdir -p build/bootstrap
print_status "Build directory created"

# Step 3: Compile stdlib first (needed by compiler)
print_info "Step 3: Compiling standard library..."

# Core modules
for module in core io mem math string vec fs result test; do
    if [ -f "stdlib/$module.zen" ]; then
        print_info "Compiling stdlib/$module.zen..."
        ./target/release/zen stdlib/$module.zen -c -o build/bootstrap/stdlib_$module.o 2>/dev/null || {
            print_error "Failed to compile stdlib/$module.zen"
        }
    fi
done
print_status "Standard library compiled"

# Step 4: Compile self-hosted compiler components
print_info "Step 4: Compiling self-hosted compiler components..."

# Check which compiler files exist
COMPILER_FILES=""
for component in lexer parser type_checker codegen errors main; do
    if [ -f "compiler/$component.zen" ]; then
        COMPILER_FILES="$COMPILER_FILES compiler/$component.zen"
        print_info "Found compiler/$component.zen"
    fi
done

if [ -z "$COMPILER_FILES" ]; then
    print_error "No compiler source files found in compiler/"
    print_info "Please ensure self-hosted compiler source files exist"
    exit 1
fi

# Compile each component
for component in lexer parser type_checker codegen errors main; do
    if [ -f "compiler/$component.zen" ]; then
        print_info "Compiling $component..."
        ./target/release/zen compiler/$component.zen -c -o build/bootstrap/${component}.o 2>/dev/null || {
            print_error "Failed to compile $component - may need implementation"
        }
    fi
done

# Step 5: Try to link if we have object files
print_info "Step 5: Checking for compiled objects..."
if ls build/bootstrap/*.o 1> /dev/null 2>&1; then
    print_info "Found object files, attempting to link..."
    clang build/bootstrap/*.o -o build/bootstrap/zenc-stage1 -lm 2>/dev/null || {
        print_info "Linking failed - components may not be fully implemented yet"
    }
else
    print_info "No object files generated yet - compiler needs more implementation"
fi

# Step 6: Test with Rust compiler for now
print_info "Step 6: Setting up interim solution..."
cp target/release/zen build/zenc
chmod +x build/zenc
print_status "Using Rust compiler as interim solution"

# Step 7: Run basic test
print_info "Step 7: Running basic test..."
echo 'main = () i32 { return 42 }' > build/bootstrap/test.zen
./build/zenc build/bootstrap/test.zen -c -o build/bootstrap/test.o 2>/dev/null
if [ $? -eq 0 ]; then
    print_status "Basic compilation test passed"
else
    print_error "Basic compilation test failed"
fi

# Step 8: Test import validation
print_info "Step 8: Testing import system..."
cat > build/bootstrap/test_imports.zen << 'EOF'
core := @std.core
io := @std.io

main = () i32 {
    return 0
}
EOF

./build/zenc build/bootstrap/test_imports.zen -c -o build/bootstrap/test_imports.o 2>/dev/null
if [ $? -eq 0 ]; then
    print_status "Import system working correctly"
else
    print_error "Import system test failed"
fi

echo ""
echo "========================================="
echo -e "${GREEN}Bootstrap Status${NC}"
echo "========================================="
echo ""

if [ -f "build/bootstrap/zenc-stage1" ]; then
    echo "Self-hosted compiler available at:"
    echo "  build/bootstrap/zenc-stage1"
else
    echo "Self-hosted compiler not yet fully compiled."
    echo "Using Rust compiler at:"
    echo "  build/zenc"
fi

echo ""
echo "You can test with:"
echo "  ./build/zenc examples/hello.zen"
echo ""
echo "To continue development:"
echo "  1. Complete implementation of compiler/*.zen files"
echo "  2. Re-run this bootstrap script"
echo "  3. Test with more complex examples"
echo ""