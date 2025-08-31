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

## Current Status (Updated: 2025-08-31 - v2)
- Import system: ✅ Fully implemented (module-level only, no comptime wrapper)
- Parser: ✅ Correctly rejects imports in comptime blocks  
- Semantic analyzer: ✅ Validates imports at module level
- LLVM codegen: ✅ Generates code correctly
- Struct pointer field access: ✅ Fixed - proper type handling for pointer to struct
- Function return type inference: ✅ Fixed - correctly infers types from function calls
- Function argument coercion: ✅ Fixed - auto-casts integer arguments to match parameters
- Pointer loading: ✅ Fixed - correctly loads pointer values when used as identifiers
- Self-hosted lexer: ✅ Complete implementation with all tokens
- Self-hosted parser: ✅ Complete implementation
- Self-hosted type checker: ✅ Complete implementation
- Self-hosted codegen: ✅ Complete implementation
- Standard library: ✅ Full stdlib in Zen (io, mem, math, string, vec, fs, etc.)
- LSP syntax checker: ✅ Working implementation with zen-lint.sh
- Test status: ✅ 9/10 language feature tests passing (nested pattern matching needs fix)
- Self-hosting: ✅ Bootstrap script created, infrastructure ready

## Recent Accomplishments (2025-08-31 - Session 2)
- Fixed struct pointer field access (result.quotient where result is *DivModResult)
- Fixed type inference for function call results (correct type from function return)
- Fixed pointer loading in compile_identifier (loads pointer values correctly)
- Added automatic type coercion for function arguments (i32 to i64 casting)
- Improved Generic type handling for struct field access
- 9 out of 10 language feature tests now passing

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
