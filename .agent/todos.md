# Zen Language TODO List

## High Priority (Current Focus)
- [x] Fix import system - use module-level imports instead of comptime
- [x] Update parser for new import syntax
- [x] Update semantic analyzer for imports
- [x] Fix LLVM codegen for imports
- [ ] Complete self-hosted compiler components
- [ ] Expand standard library in Zen
- [ ] Comprehensive test coverage

## Self-Hosting Tasks
### Compiler Components
- [ ] lexer.zen - Complete tokenization
- [ ] parser.zen - Full syntax support
- [ ] type_checker.zen - Type inference and checking
- [ ] codegen.zen - Code generation backend
- [ ] llvm_backend.zen - LLVM integration
- [ ] errors.zen - Error handling and reporting

### Standard Library
- [ ] Core modules (io, mem, math, string)
- [ ] Collections (vec, hashmap, set)
- [ ] File system operations
- [ ] Network utilities
- [ ] Process management
- [ ] Threading support

### Build Tools
- [ ] zen-compile - Main compiler driver
- [ ] zen-build - Build system
- [ ] zen-pkg - Package manager
- [ ] zen-check - Syntax checker
- [ ] zen-fmt - Code formatter

## Testing Requirements
- [ ] Import system tests (comprehensive)
- [ ] Self-hosted lexer tests
- [ ] Self-hosted parser tests
- [ ] Integration tests for all stdlib modules
- [ ] Bootstrap test (compile compiler with itself)

## LSP Development
- [ ] Basic syntax checking
- [ ] Go to definition
- [ ] Find references
- [ ] Auto-completion
- [ ] Hover information
- [ ] Diagnostics

## Documentation
- [ ] Language specification
- [ ] Standard library docs
- [ ] Self-hosting guide
- [ ] Contributing guide
- [ ] Examples and tutorials

## Performance Optimizations
- [ ] Optimize lexer performance
- [ ] Improve parser speed
- [ ] LLVM optimization passes
- [ ] Compile-time evaluation caching

## Future Features
- [ ] Async/await support
- [ ] More advanced generics
- [ ] Compile-time reflection
- [ ] Package registry
- [ ] Cross-compilation support
