# Zenlang Development Plan

## Current Phase: Stabilization & Self-Hosting

### Immediate Goals (Next 2 Weeks)
1. **Fix Remaining Test** - Get to 100% test pass rate
2. **Complete Pattern Matching** - Finish codegen for `?` operator
3. **Comptime Framework** - Enable compile-time code execution

### Q1 2025 Roadmap
1. **Self-Hosted Compiler** (Month 1-2)
   - Complete lexer in Zen (70% remaining)
   - Complete parser in Zen (80% remaining)
   - Integrate with existing type checker

2. **Language Features** (Month 2-3)
   - Behaviors (traits) implementation
   - Complete UFCS
   - String interpolation in codegen
   - Smart pointers (Ptr<T>, Ref<T>)

3. **Tooling & Ecosystem** (Month 3)
   - Package manager design
   - LSP improvements
   - Documentation generator

### Long-Term Vision
- **Year 1**: Fully self-hosted compiler, stable 1.0 release
- **Year 2**: Production-ready ecosystem, major adoption
- **Year 3**: Industry standard for systems programming

## Technical Strategy

### Architecture Decisions
- LLVM backend (keep for performance)
- Incremental compilation support
- Module-based standard library
- Zero-cost abstractions throughout

### Quality Standards
- 100% test coverage for core features
- All examples must compile and run
- Performance within 10% of C
- Compilation speed < 100ms for small programs

### Community Building
- Clear documentation
- Interactive tutorials
- Example projects
- Active GitHub discussions

## Risk Mitigation
- **Self-hosting complexity**: Incremental approach, keep Rust fallback
- **Performance regression**: Continuous benchmarking
- **Feature creep**: Strict adherence to spec
- **Adoption barriers**: Focus on developer experience