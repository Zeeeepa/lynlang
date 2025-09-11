# Zenlang Global Memory

## Project Overview
Zenlang is a modern systems programming language with a focus on clarity, safety, and performance. Version 1.1.0 of the language specification enforces strict rules including:
- NO if/else/match keywords - use ? operator exclusively
- NO exceptions - all errors are values (Result/Option types)
- NO null pointers - use Option<T> for optional values
- NO implicit conversions
- NO raw & or * - use Ptr<T> and .value/.address
- NO tuples - use structs for all product types
- Colorless async via allocators (no function coloring)

## Current Implementation Status

### Completed Features
- **FFI Builder Pattern**: Fully implemented safe C interop with builder pattern as per spec v1.1.0
  - LibBuilder for configuring foreign libraries
  - Function signature validation
  - Platform-specific library path resolution
  - Call statistics tracking
  - Safety levels (Safe, Unsafe, Trusted)
  
- **LSP Support**: Basic Language Server Protocol implementation
  - Semantic tokens
  - Hover information
  - Diagnostics
  
- **Core Compiler**: 
  - Lexer and Parser with pattern matching (? operator)
  - LLVM-based code generation
  - Type checking system
  - Module system with imports
  
- **Testing Infrastructure**: Comprehensive test suite with 100+ tests passing

### In Progress
- Enhanced pattern matching features
- Behavior system (structural contracts)
- Async/colorless execution via allocators
- Standard library modules

### Architecture
- Written in Rust for safety and performance
- LLVM backend for optimized code generation
- Modular design with clear separation of concerns
- Builder patterns throughout for safe API design

## Key Files
- `LANGUAGE_SPEC.md`: Authoritative language specification (v1.1.0)
- `src/ffi/mod.rs`: FFI builder pattern implementation
- `src/lsp/`: Language server protocol implementation
- `src/codegen/llvm/`: LLVM code generation backend
- `src/parser/`: Lexer and parser implementation
- `src/typechecker/`: Type checking and inference

## Build & Test
- `cargo build`: Build the compiler
- `cargo test`: Run all tests
- `cargo check`: Type check without building
- Tests are all passing as of latest run

## Recent Work (2025-09-11)
- Implemented FFI builder pattern following spec requirements
- Fixed all LSP deprecation warnings
- Cleaned up unused imports and variables across the codebase
- All tests passing successfully (100+ tests)

## Next Steps
- Continue implementing missing language features from spec
- Enhance standard library modules
- Implement behaviors system
- Add more comprehensive integration tests
- Work on async/colorless execution model