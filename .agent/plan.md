# Zen Language Development Plan

## Project Vision
Build a self-hosting systems programming language with minimal keywords and maximum expressiveness through composable primitives.

## Current Status (2025-08-31)
- ✅ Import system refactored - no comptime wrapper needed
- ✅ All 66 tests passing
- ✅ Test runner script created
- ✅ Syntax checking tools implemented
- ✅ Basic stdlib in Zen (io, math, collections, etc.)
- ✅ LSP implementation created (basic functionality)
- ✅ Type checking tool created (shell and Zen versions)
- 🔄 Working on self-hosting components

## Architecture Overview

### Phase 1: Foundation (COMPLETE)
- [x] Core language features
- [x] Import system without comptime
- [x] Basic stdlib modules
- [x] Test infrastructure

### Phase 2: Tools & Validation (COMPLETE)
- [x] Test runner (test_runner.sh)
- [x] Syntax checker (zen-check.sh)
- [x] Syntax validator in Zen (tools/syntax_checker.zen)
- [x] Type checker tool (zen-typecheck.sh)
- [x] LSP implementation (tools/zen-lsp.zen)

### Phase 3: Self-Hosting (NEXT)
- [ ] Complete lexer in Zen
- [ ] Parser in Zen  
- [ ] Type checker in Zen
- [ ] Code generator in Zen
- [ ] Full compiler bootstrap

### Phase 4: Optimization
- [ ] Performance tuning
- [ ] Memory optimization
- [ ] Compile-time evaluation
- [ ] Dead code elimination
- [ ] Inline optimization

## Technical Roadmap

### Immediate Tasks (1-2 days)
1. Fix remaining type system issues
2. Complete stdlib testing
3. Implement basic LSP
4. Create type checking tool

### Near-term Goals (1 week)
1. Complete self-hosted lexer
2. Implement self-hosted parser
3. Create documentation generator
4. Build package manager prototype

### Long-term Goals (1 month)
1. Full self-hosting compiler
2. Advanced optimization passes
3. Debug information generation
4. Cross-compilation support
5. Standard library completion

## File Organization

```
zenlang/
├── src/              # Rust compiler (transitioning out)
├── stdlib/           # Standard library in Zen
│   ├── io.zen       # I/O operations
│   ├── math.zen     # Math functions
│   ├── lexer.zen    # Self-hosted lexer
│   ├── parser.zen   # Self-hosted parser
│   └── ...
├── tools/            # Development tools
│   └── syntax_checker.zen
├── tests/            # Test files
├── examples/         # Example programs
└── .agent/          # Project metadata
```

## Key Design Principles

1. **Simplicity**: Minimal keywords, maximum expressiveness
2. **Composability**: Small, composable primitives
3. **Self-hosting**: Compiler written in Zen itself
4. **Performance**: Zero-cost abstractions
5. **Safety**: Memory safety without garbage collection

## Testing Strategy

- Unit tests for each compiler component
- Integration tests for language features
- Regression tests for bug fixes
- Performance benchmarks
- Self-hosting validation

## Success Metrics

- [ ] 100% test coverage
- [ ] Self-hosting compiler compiles itself
- [ ] < 1s compile time for 10k LOC
- [ ] Zero memory leaks
- [ ] Full language specification compliance

## Next Actions

1. Run full test suite
2. Fix any failing tests
3. Continue lexer implementation
4. Start parser implementation
5. Document progress

## Notes

- Focus on correctness over performance initially
- Keep backward compatibility during transition
- Document all design decisions
- Maintain clean git history with frequent commits