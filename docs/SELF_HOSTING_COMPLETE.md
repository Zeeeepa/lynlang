# Zen Language Self-Hosting Guide

## Overview

The Zen language is designed to be self-hosting, meaning the Zen compiler is written in Zen itself. This document describes the current status and process for bootstrapping the self-hosted compiler.

## Current Status (as of 2025-08-31)

### ✅ Completed Components

1. **Import System**
   - Module-level imports fully implemented
   - Comptime blocks no longer wrap imports
   - All Zen files updated to use correct syntax
   - Import validation tests passing

2. **Self-Hosted Compiler Components**
   - `compiler/lexer.zen` - Complete tokenization
   - `compiler/parser.zen` - Full AST generation
   - `compiler/type_checker.zen` - Type validation
   - `compiler/codegen.zen` - LLVM IR generation
   - `compiler/errors.zen` - Error handling
   - `compiler/main.zen` - Compiler entry point

3. **Standard Library**
   - Core modules implemented in Zen
   - `stdlib/core.zen` - Core types and functions
   - `stdlib/io.zen` - Input/output operations
   - `stdlib/math.zen` - Mathematical functions
   - `stdlib/string.zen` - String manipulation
   - `stdlib/vec.zen` - Dynamic arrays
   - `stdlib/fs.zen` - File system operations
   - And many more...

4. **Development Tools**
   - `bootstrap.sh` - Bootstrap script for self-hosting
   - `zen-lint.sh` - Basic linter
   - `zen-lint-enhanced.sh` - Advanced linter with multiple output formats
   - LSP server implementation in Zen

### ⚠️ In Progress

- Full self-hosting bootstrap (currently uses Rust compiler as interim)
- Additional stdlib functions (e.g., `io.print_float`)

## Bootstrap Process

### Prerequisites

1. Rust toolchain (for initial bootstrap)
2. LLVM 18+ 
3. Clang/GCC (for linking)

### Steps to Bootstrap

```bash
# 1. Clone the repository
git clone https://github.com/your-org/zenlang.git
cd zenlang

# 2. Run the bootstrap script
./bootstrap.sh

# 3. The script will:
#    - Build the Rust-based compiler
#    - Compile the standard library
#    - Attempt to compile self-hosted components
#    - Create build/zenc executable

# 4. Test the compiler
./build/zenc examples/hello.zen
```

### Manual Bootstrap Process

If you want to understand the bootstrap process in detail:

```bash
# Step 1: Build Rust compiler (stage 0)
cargo build --release

# Step 2: Use stage 0 to compile stdlib
./target/release/zen stdlib/core.zen -o build/stdlib_core.o
./target/release/zen stdlib/io.zen -o build/stdlib_io.o
# ... repeat for all stdlib modules

# Step 3: Compile self-hosted compiler with stage 0
./target/release/zen compiler/lexer.zen -o build/lexer.o
./target/release/zen compiler/parser.zen -o build/parser.o
./target/release/zen compiler/type_checker.zen -o build/type_checker.o
./target/release/zen compiler/codegen.zen -o build/codegen.o
./target/release/zen compiler/errors.zen -o build/errors.o
./target/release/zen compiler/main.zen -o build/main.o

# Step 4: Link stage 1 compiler
clang build/*.o -o build/zenc-stage1 -lm

# Step 5: Use stage 1 to compile itself (stage 2)
./build/zenc-stage1 compiler/main.zen -o build/zenc-stage2

# Step 6: Verify stage 2 can compile itself (stage 3)
./build/zenc-stage2 compiler/main.zen -o build/zenc

# If stage 2 and stage 3 are identical, bootstrap is successful!
```

## Import System

The Zen import system has been redesigned for clarity and simplicity:

### Correct Import Syntax

```zen
// Module-level imports (correct)
core := @std.core
io := @std.io
math := @std.math

// Use imported modules
main = () i32 {
    io.print("Hello, World!\n")
    return 0
}
```

### Incorrect Import Syntax (No Longer Supported)

```zen
// DO NOT USE - This is no longer valid
comptime {
    core := @std.core
    io := @std.io
}
```

### Import Rules

1. **Module-level only**: Imports must be at the top level of the file
2. **Before code**: Imports should appear before any function or type definitions
3. **No comptime wrapper**: Never wrap imports in comptime blocks
4. **Direct assignment**: Use simple assignment syntax `name := @std.module`

## Development Workflow

### Testing Changes

```bash
# Run the linter on your code
./zen-lint-enhanced.sh your_file.zen

# Run tests
cargo test

# Test specific functionality
cargo test import  # Test import system
cargo test self_hosted  # Test self-hosted components
```

### Using the LSP

The Zen LSP provides real-time syntax checking:

```bash
# Basic LSP server
./zen-lsp

# With your editor (example for VS Code)
# Add to settings.json:
{
    "zen.lsp.path": "/path/to/zenlang/zen-lsp"
}
```

### Linter Options

```bash
# Basic syntax check
./zen-lint.sh file.zen

# Enhanced linting with all checks
./zen-lint-enhanced.sh -v file.zen

# GitHub Actions compatible output
./zen-lint-enhanced.sh --output github src/

# Security checks
./zen-lint-enhanced.sh -S file.zen
```

## Contributing

### Adding New Features

1. Implement in Rust compiler first (for bootstrap)
2. Add tests to verify functionality
3. Implement in self-hosted compiler
4. Update documentation

### Testing Self-Hosted Components

```bash
# Run integration tests
./build/zenc tests/test_zen_integration.zen

# Test specific component
./build/zenc tests/test_self_hosted_lexer.zen
```

## Troubleshooting

### Common Issues

1. **Import errors**: Ensure imports are at module level, not in comptime blocks
2. **Linking errors**: Make sure LLVM is properly installed
3. **Type errors**: Check that all stdlib modules are compiled

### Debug Mode

```bash
# Enable verbose output
ZEN_DEBUG=1 ./build/zenc file.zen

# Check LLVM IR output
./build/zenc file.zen --emit-ir
```

## Future Work

1. Complete self-hosting bootstrap without Rust dependency
2. Implement remaining stdlib functions
3. Add incremental compilation
4. Optimize compiler performance
5. Package manager implementation

## Resources

- [Language Specification](./lang.md)
- [Style Guide](./STYLE_GUIDE.md)
- [Compiler Architecture](./COMPILER_ARCH.md)
- [Standard Library Reference](./STDLIB.md)

## Contact

For questions or issues, please open an issue on GitHub or contact the maintainers.