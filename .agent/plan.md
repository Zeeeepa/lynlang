# ZenLang Development Plan

## Current Phase: Infrastructure & Stability
**Goal**: Fix CI/CD, organize project, establish development workflow

### Immediate Actions
1. Fix GitHub workflows for LLVM 18.1 compatibility
2. Organize .agent directory for persistent memory
3. Establish git workflow with frequent commits

## Next Phase: Self-Hosting (Q1 2025)
**Goal**: Complete self-hosted compiler in ZenLang

### Milestones
1. **Lexer Migration** (2 weeks)
   - Port Rust lexer to ZenLang
   - Maintain compatibility with existing parser
   - Add comprehensive tests

2. **Parser Migration** (3 weeks)
   - Implement recursive descent parser in ZenLang
   - Support full language syntax
   - Error recovery and reporting

3. **Code Generation** (4 weeks)
   - LLVM bindings in ZenLang
   - IR generation
   - Optimization passes

4. **Type System** (3 weeks)
   - Type inference engine
   - Constraint solving
   - Generic support

## Future Phases

### Phase 3: Ecosystem (Q2 2025)
- Package manager
- Build system
- Documentation generator
- Testing framework

### Phase 4: Advanced Features (Q3 2025)
- Async/await
- Macros system
- Compile-time execution
- Advanced optimizations

### Phase 5: Production Ready (Q4 2025)
- Performance optimizations
- Security audit
- Stable API
- 1.0 release

## Development Principles
- **Simplicity**: Keep design minimal and elegant
- **Practicality**: Focus on real-world use cases
- **Performance**: Zero-cost abstractions where possible
- **Safety**: Memory safety without garbage collection
- **Interop**: Seamless C/LLVM integration