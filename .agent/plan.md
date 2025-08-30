# Zen Language - Self-Hosting Plan

## Goal
Bootstrap the Zen compiler in Zen itself, achieving self-hosting capability.

## Current Progress
✅ Import syntax simplified (no comptime required)
✅ Basic token definitions created
✅ Initial lexer implementation started

## Phases

### Phase 1: Lexer (In Progress)
- [x] Token type definitions
- [x] Basic lexer structure
- [ ] Complete operator tokenization
- [ ] Comment handling
- [ ] Error reporting
- [ ] Test suite for lexer

### Phase 2: Parser
- [ ] AST node definitions
- [ ] Expression parser
- [ ] Statement parser
- [ ] Type parser
- [ ] Error recovery
- [ ] Test suite for parser

### Phase 3: Type Checker
- [ ] Type inference
- [ ] Type checking rules
- [ ] Generic type handling
- [ ] Error reporting
- [ ] Test suite for type checker

### Phase 4: Code Generator
- [ ] LLVM bindings
- [ ] Expression compilation
- [ ] Statement compilation
- [ ] Function compilation
- [ ] Module compilation
- [ ] Optimization passes

### Phase 5: Bootstrap
- [ ] Compile compiler with itself
- [ ] Verify output matches
- [ ] Performance benchmarks
- [ ] Documentation

## Technical Challenges
1. String handling without full stdlib
2. Dynamic memory allocation
3. LLVM FFI bindings
4. Error handling patterns
5. Module system implementation

## Testing Strategy
- Unit tests for each component
- Integration tests for compilation
- Bootstrap validation tests
- Performance regression tests

## Estimated Timeline
- Phase 1: 1 week
- Phase 2: 2 weeks  
- Phase 3: 2 weeks
- Phase 4: 3 weeks
- Phase 5: 1 week

Total: ~9 weeks for full self-hosting
