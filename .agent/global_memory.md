# Zenlang Global Memory

## Project Overview
Zenlang is a systems programming language with:
- Clean import syntax (no comptime wrapping required)
- Self-hosting compiler (in progress)
- Comprehensive standard library
- LSP support

## Key Locations
- Parser: `src/parser/statements.rs:14-120`
- Module System: `src/module_system/mod.rs`
- Type Checker: `src/typechecker/validation.rs:160-181`
- Self-hosted Compiler: `bootstrap/compiler.zen`
- Standard Library: `stdlib/`

## Import Syntax Evolution
```zen
// OLD (deprecated)
comptime {
    core := @std.core
    io := build.import("io")
}

// NEW (current)
core := @std.core
io := @std.io
```

## Self-Hosting Status
- ✅ Lexer (stdlib/compiler/lexer.zen)
- ✅ Parser (compiler/parser.zen)
- ✅ Type Checker (stdlib/compiler/type_checker.zen)
- ✅ Symbol Table (stdlib/compiler/symbol_table.zen)
- 🚧 Code Generator (transitioning from Rust)
- 🚧 LLVM Backend (integration needed)

## Testing Strategy
- Unit tests for each component
- Integration tests for full compilation
- Import validation tests
- Self-hosting bootstrap tests

## Known Issues
- Type checker validation disabled (needs re-enabling)
- Some examples still use old import syntax
- LSP import validation incomplete

## Recent Changes (2025-08-31)
- Major import system refactoring
- Added comprehensive stdlib modules
- Enhanced self-hosting components
- Improved test coverage