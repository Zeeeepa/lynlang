#!/bin/bash

# Build script for zen compiler

# Build in release mode
echo "Building zen compiler..."
cargo build --release

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo "Build successful!"
    
    # Create symlink for easy access
    if [ ! -L "zenc" ]; then
        ln -sf target/release/zen zenc
    fi
    
    echo "Zen compiler built successfully at ./zenc"
    echo ""
    echo "Usage:"
    echo "  ./zenc <file.zen>        - Compile a zen file"
    echo "  ./zenc --help            - Show help"
    echo ""
    echo "To run tests:"
    echo "  cargo test               - Run Rust tests"
    echo "  ./scripts/run_tests.sh   - Run zen tests"
else
    echo "Build failed!"
    exit 1
fi