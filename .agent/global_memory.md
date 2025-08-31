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

## Current Status (Updated: 2025-08-31)
- Import system: ✅ Fully implemented (module-level only, no comptime wrapper)
- Parser: ✅ Correctly rejects imports in comptime blocks  
- Semantic analyzer: ✅ Validates imports at module level
- LLVM codegen: ✅ Generates code correctly
- Self-hosted lexer: ✅ Complete implementation with all tokens
- Self-hosted parser: ✅ Complete implementation
- Self-hosted type checker: ✅ Complete implementation
- Self-hosted codegen: ✅ Complete implementation
- Standard library: ✅ Full stdlib in Zen (io, mem, math, string, vec, fs, etc.)
- LSP syntax checker: ✅ Working implementation with zen-lint.sh
- Import validation tests: ⚠️ 3/4 passing (io.print_float not implemented)
- Self-hosting: ✅ Bootstrap script created, infrastructure ready

## Recent Accomplishments (2025-08-31)
- Fixed import system to use module-level imports without comptime wrapper
- All Zen files now use correct import syntax
- Created enhanced linter with multiple output formats (GitHub Actions compatible)
- Implemented comprehensive integration tests
- Created bootstrap script for self-hosting
- Documented complete self-hosting process
- Working LSP/linter implementation (zen-lint.sh and zen-lint-enhanced.sh)

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
