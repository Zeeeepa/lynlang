# Zen Language Global Memory

## Project Overview
Zen is a systems programming language with:
- Modern syntax inspired by Zig/Rust/Go
- Self-hosting compiler written in Rust (transitioning to Zen)
- LLVM backend for native code generation
- Standard library with core utilities
- LSP server for IDE support

## Key Language Features
1. **Import System**: Module-level imports without comptime blocks
   - `core := @std.core` - Standard library imports
   - `io := build.import("io")` - Build system imports
   
2. **Comptime**: For meta-programming and compile-time evaluation
   - NOT for imports
   - Used for generating lookup tables, constants, etc.

3. **Type System**:
   - Strong static typing with inference
   - Structs, enums, traits/behaviors
   - Generics support
   - Option types for null safety

4. **Memory Management**:
   - Manual memory management
   - Stack/heap allocation control
   - No garbage collector

## Project Structure
```
/home/ubuntu/zenlang/
├── src/           # Rust compiler implementation
├── compiler/      # Self-hosted Zen compiler
├── stdlib/        # Standard library in Zen
├── tools/         # Build tools and utilities
├── lsp/           # Language server
├── tests/         # Test suite
├── examples/      # Example Zen programs
└── .agent/        # Meta information for AI assistance
```

## Current Status
- Import system: ✅ Implemented correctly
- Parser: ✅ Handles module-level imports
- Semantic analyzer: ✅ Validates imports
- LLVM codegen: ✅ Generates code correctly
- Self-hosting: 🚧 In progress
- Stdlib: 🚧 Being expanded

## Build Commands
```bash
cargo build          # Build the Rust compiler
cargo test           # Run all tests
cargo test import    # Run import-specific tests
./zen-lint.sh        # Run linter
```

## Testing Strategy
- Unit tests in Rust (src/**/*.rs)
- Integration tests (tests/*.rs)
- Zen test files (tests/*.zen)
- Self-hosted component tests

## Code Principles
- DRY (Don't Repeat Yourself)
- KISS (Keep It Simple, Stupid)
- Simplicity and elegance
- Practical solutions
