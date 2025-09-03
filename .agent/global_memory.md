# Zen Language - Global Memory

## Project Overview
Zen is a modern systems programming language with ~55% compiler implementation complete.
- **Language Version**: v1.0 specification complete
- **Binaries**: zen, zen-lsp, zen-check, zen-format  
- **LLVM Version**: 18.1 (inkwell 0.6.0 with llvm18-1 feature)
- **File Extension**: .zen
- **Repository**: https://github.com/zen-lang/zen

## Current Architecture

### Import System
- **Module Level Only**: Imports must be at top-level, not in functions/comptime
- **Syntax**: `identifier := @std.module` or `build.import("module")`
- **Validation**: Parser, TypeChecker, and zen-check all enforce rules

### Self-Hosting Status (~55% Complete)
- **Lexer**: 90% complete (stdlib/compiler/lexer.zen)
- **Parser**: 25% complete (stdlib/compiler/parser.zen) 
- **TypeChecker**: In Rust, needs Zen port
- **CodeGen**: In Rust, needs Zen port
- **Bootstrap**: Ready for testing

### LSP Server
- **Main**: src/lsp/mod.rs - Core LSP implementation
- **Enhanced**: src/lsp/enhanced.rs - Advanced features
- **Capabilities**: Syntax validation, imports, symbols, references, actions, hover, goto

### Standard Library
- **91 .zen files** across stdlib/
- **Core modules**: core, io, math, string, vec, result, option
- **Collections**: hashmap, list, stack, queue
- **System**: fs, process, net, thread
- **Utilities**: json, regex, datetime, crypto

## Testing Status
- **Library tests**: ✅ 11/11 passing
- **Parser tests**: ⚠️ Hanging (needs fix)
- **Integration**: 🚧 In progress
- **Self-hosting**: ✅ Test suite created

## Key Directories
- `src/` - Rust compiler source
- `stdlib/` - Standard library in Zen
- `compiler/` - Self-hosted compiler components
- `examples/` - Example programs
- `tests/` - Test suites
- `.agent/` - Project metadata and planning

## Development Principles
- **KISS & DRY**: Keep It Simple, Don't Repeat Yourself
- **80/20 Rule**: 80% implementation, 20% testing
- **Context**: Optimal at 100K-140K tokens
- **Git**: Frequent commits with clear messages
- **Testing**: Test-driven development where practical