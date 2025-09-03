# ZenLang Global Memory

## Project Overview
ZenLang is a systems programming language compiler built in Rust, using LLVM 18.1 for code generation.

## Key Components
- **Compiler**: Rust-based compiler using inkwell for LLVM bindings
- **LLVM Version**: 18.1 (locked in Cargo.toml with feature flag)
- **Language Server**: LSP implementation for IDE support
- **Tools**: zen-check, zen-format, zen-lsp binaries

## Architecture
- Frontend: Lexer, Parser (nom-based)
- Middle: Type checker, semantic analysis
- Backend: LLVM IR generation via inkwell
- Runtime: Links with LLVM runtime

## Self-Hosting Progress
Currently transitioning to self-hosted compiler with components:
- compiler/lexer.zen (pending)
- compiler/parser.zen (pending)
- compiler/codegen.zen (pending)
- compiler/type_checker.zen (pending)

## Build System
- Uses cargo with specific LLVM 18.1 configuration
- Environment variable: LLVM_SYS_181_PREFIX=/usr/lib/llvm-18
- GitHub Actions CI/CD with Ubuntu latest

## Testing Strategy
- Unit tests via cargo test --lib
- Integration tests via cargo test --test
- Example programs in examples/ directory
- Self-hosting validation checks

## Current State
- Core compiler working in Rust
- Stdlib modules being developed
- LSP server functional
- CI/CD workflows need fixes for LLVM 18.1