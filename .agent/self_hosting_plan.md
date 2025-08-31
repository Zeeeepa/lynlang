# Zen Self-Hosting Plan

## Current State
- Import system fixed: imports at module level only (no comptime blocks)
- Basic bootstrap compiler exists but needs enhancement
- Rust compiler working well, generates LLVM IR

## Self-Hosting Strategy

### Phase 1: Core Compiler Components (Current)
1. **Lexer** - Tokenization
   - [x] Basic implementation exists
   - [ ] Need to handle all Zen syntax
   - [ ] Error recovery

2. **Parser** - AST generation
   - [x] Basic structure exists
   - [ ] Complete all statement types
   - [ ] Pattern matching support
   - [ ] Generic support

3. **Type Checker** - Type validation
   - [x] Basic structure exists
   - [ ] Type inference
   - [ ] Generic instantiation
   - [ ] Trait checking

4. **Code Generator** - Target code generation
   - [x] C backend started
   - [ ] LLVM IR backend
   - [ ] Optimization passes

### Phase 2: Standard Library in Zen
- [x] Basic modules (io, fs, string, vec, math)
- [ ] Complete implementations
- [ ] Testing framework
- [ ] Collections (hashmap, set, queue)

### Phase 3: Bootstrap Process
1. Use Rust compiler to compile Zen compiler
2. Use Zen compiler (compiled by Rust) to compile itself
3. Compare outputs to ensure correctness
4. Switch to self-hosted compiler

### Phase 4: Advanced Features
- Package manager
- Build system
- LSP server
- Debugger support

## Immediate Next Steps
1. Enhance lexer to handle all tokens
2. Complete parser for all AST nodes
3. Implement basic type checking
4. Generate executable code (via C or LLVM)
5. Test with simple programs
6. Gradually increase complexity

## Success Criteria
- Can compile hello world program
- Can compile itself
- Passes all existing tests
- Performance comparable to Rust version