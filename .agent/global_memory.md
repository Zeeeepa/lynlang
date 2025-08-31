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
- ✅ Code Generator (stdlib/compiler/codegen.zen - complete with C and LLVM IR targets)
- ✅ LLVM Backend (stdlib/compiler/llvm_backend.zen - full integration)

## Testing Strategy
- Unit tests for each component
- Integration tests for full compilation
- Import validation tests
- Self-hosting bootstrap tests

## Known Issues
- Type checker validation disabled (needs re-enabling)
- ✅ Fixed: Examples now use correct import syntax
- ✅ Fixed: LSP import validation implemented
- ⚠️ Parser limitation: Nested conditionals require parentheses for correct parsing

## Recent Changes (2025-08-31)
- Major import system refactoring - imports must be at module level  
- Added comprehensive stdlib modules in Zen
- ✅ Completed self-hosted code generator with C and LLVM IR targets
- ✅ Integrated LLVM backend module with full IR generation
- ✅ Added comprehensive import validation tests
- ✅ Implemented LSP validation for import placement
- Enhanced test coverage with all tests passing (except nested pattern matching)
- Created stdlib modules: io.zen, core.zen, math.zen, string.zen, vec.zen
- Implemented self-hosted lexer.zen with full tokenization
- Fixed test_comptime_import_error to validate at type-check phase
- ✅ Verified all example files use correct import syntax
- ✅ Confirmed comprehensive stdlib implementation complete
- ✅ Self-hosted compiler implementation in bootstrap/compiler.zen
- ✅ Fixed remaining comptime import issues in test files
- ✅ LSP validation for imports already implemented and working
- ⚠️ Known issue: nested pattern matching test failure (parser limitation - requires parentheses)