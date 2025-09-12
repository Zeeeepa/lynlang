# Zenlang Global Memory

## Project Overview
Zenlang is a modern systems programming language with a focus on clarity, safety, and performance. It follows a unique philosophy of no traditional control flow keywords (if/else/match), instead using the `?` operator for all pattern matching.

## Key Language Features
- **No if/else/match keywords** - Use `?` operator exclusively for pattern matching
- **No exceptions** - All errors are values (Result/Option types)
- **No null pointers** - Use Option<T> for optional values
- **No implicit conversions** - All type conversions must be explicit
- **No lifetime annotations** - Smart pointers handle safety
- **No raw `&` or `*`** - Use Ptr<T> and .value/.address
- **No tuples** - Use structs for all product types
- **Colorless async** - No function coloring with async/await

## Current Implementation Status (as of 2025-09-12)

### Completed Features
✅ Lexer and Parser with comprehensive error handling
✅ Type system with generics
✅ Pattern matching with `?` operator
✅ Loop constructs (no while/for keywords)
✅ FFI builder pattern for safe C interop
✅ Enhanced LSP with detailed error diagnostics
✅ Module system with @std namespace
✅ Behaviors (structural contracts)
✅ Compile-time evaluation (comptime blocks)
✅ String interpolation
✅ Fixed arrays and slices
✅ LLVM code generation backend

### In Progress
🔄 Self-hosting compiler
🔄 Standard library implementation
🔄 Async system with allocators
🔄 Complete test coverage

### TODO
- [ ] Full stdlib modules (io, fs, net, etc.)
- [ ] Package manager
- [ ] Documentation generator
- [ ] REPL improvements
- [ ] Debugger integration
- [ ] Cross-compilation support
- [ ] Optimization passes

## Technical Details

### Build System
- Primary compiler: Rust-based implementation in `src/`
- LLVM backend for code generation
- Self-hosted compiler in progress in `bootstrap/` and `compiler/`

### Testing
- Rust tests in `tests/` directory
- Zen test files prefixed with `zen_` in `tests/` directory
- Integration tests for all major features

### LSP Features
- Syntax error detection with helpful suggestions
- Keyword rejection (if/else/match/fn/let/var/const/while/for)
- Import validation (must be at module level)
- Pattern matching validation
- Type checking integration

## Important Files
- `LANGUAGE_SPEC.md` - Authoritative language specification (v1.1.0)
- `src/main.rs` - Main compiler entry point
- `src/lsp/mod.rs` - Language server implementation
- `src/ffi/mod.rs` - FFI builder pattern implementation
- `src/error.rs` - Enhanced error handling with detailed messages

## Development Notes
- Memory safety without lifetime annotations through smart pointers
- Zero-cost abstractions as a core principle
- Emphasis on compile-time computation where possible
- Strong focus on developer experience through LSP