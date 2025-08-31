# Zen Language Development Plan

## Phase 1: Import System Fix (Current)
- [x] Analyze current import patterns
- [ ] Fix remaining comptime-wrapped imports
- [ ] Update parser to reject imports in comptime
- [ ] Update all test files
- [ ] Verify all examples work correctly

## Phase 2: Standard Library Enhancement
- [ ] Complete core module
- [ ] Enhance io module with more operations
- [ ] Add missing string utilities
- [ ] Implement full collections library
- [ ] Add comprehensive error handling

## Phase 3: Self-Hosting Compiler
- [ ] Port lexer to Zen
- [ ] Port parser to Zen
- [ ] Port type checker to Zen
- [ ] Port code generator to Zen
- [ ] Bootstrap testing

## Phase 4: LSP Implementation
- [ ] Basic syntax checking
- [ ] Type checking integration
- [ ] Auto-completion
- [ ] Go-to-definition
- [ ] Error diagnostics

## Phase 5: Testing & Documentation
- [ ] Unit tests for all stdlib modules
- [ ] Integration tests for compiler
- [ ] Performance benchmarks
- [ ] Complete documentation
