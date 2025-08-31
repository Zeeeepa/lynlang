# Zen Language Bootstrap Sequence

## Overview

The Zen language follows a multi-stage bootstrap process to achieve self-hosting. This document outlines the bootstrap stages and the components involved in each stage.

## Bootstrap Stages

### Stage 0: Rust Compiler (Current)
- **Status**: ✅ Complete
- **Components**: 
  - Rust-based lexer, parser, and code generator
  - Basic type checking
  - LLVM backend for code generation
- **Purpose**: Provides initial compiler to compile Zen code

### Stage 1: Core Compiler in Zen
- **Status**: 🚧 In Progress
- **Components**:
  - Self-hosted lexer (`stdlib/compiler/lexer.zen`)
  - Self-hosted parser (`stdlib/parser_enhanced.zen`)
  - Type system (`stdlib/compiler/type_system.zen`)
  - Symbol table (`stdlib/compiler/symbol_table.zen`)
  - Type checker (`stdlib/compiler/type_checker.zen`)
- **Purpose**: Implement core compiler components in Zen

### Stage 2: Code Generation
- **Status**: 📋 Planned
- **Components**:
  - AST to IR transformation
  - LLVM IR generation in Zen
  - Optimization passes
- **Purpose**: Complete backend implementation in Zen

### Stage 3: Self-Compilation
- **Status**: 📋 Planned
- **Components**:
  - Full compiler written in Zen
  - Compiler can compile itself
  - Bootstrap verification
- **Purpose**: Achieve full self-hosting

## Component Dependencies

```
┌─────────────────┐
│   Rust Compiler │ (Stage 0)
└────────┬────────┘
         │ Compiles
         ▼
┌─────────────────┐
│  Core Compiler  │ (Stage 1)
│   Components    │
│   - Lexer       │
│   - Parser      │
│   - Type System │
└────────┬────────┘
         │ Uses
         ▼
┌─────────────────┐
│ Code Generator  │ (Stage 2)
│   - IR Gen      │
│   - LLVM Backend│
└────────┬────────┘
         │ Produces
         ▼
┌─────────────────┐
│ Self-Hosted     │ (Stage 3)
│   Compiler      │
└─────────────────┘
```

## Current Progress

### Completed Components
- ✅ Lexer (multiple implementations)
- ✅ Parser (enhanced version with full language support)
- ✅ Type system module
- ✅ Symbol table for name resolution
- ✅ Type checker with semantic analysis
- ✅ Standard library foundations
- ✅ Data structures (array, queue, stack)
- ✅ String utilities
- ✅ Memory management
- ✅ Test framework

### In Progress
- 🚧 Code generation in Zen
- 🚧 LLVM bindings
- 🚧 Optimization passes

### Planned
- 📋 Bootstrap verification suite
- 📋 Performance benchmarks
- 📋 Compiler driver in Zen
- 📋 Build system in Zen

## Bootstrap Process

### Step 1: Build Rust Compiler
```bash
cargo build --release
```

### Step 2: Compile Core Components
```bash
# Compile lexer
./target/release/zen stdlib/compiler/lexer.zen -o build/lexer

# Compile parser
./target/release/zen stdlib/parser_enhanced.zen -o build/parser

# Compile type system
./target/release/zen stdlib/compiler/type_system.zen -o build/type_system
```

### Step 3: Build Self-Hosted Compiler
```bash
# Use compiled components to build full compiler
./build/zen-compiler src/compiler/main.zen -o zen-self
```

### Step 4: Verify Bootstrap
```bash
# Self-compile to verify
./zen-self src/compiler/main.zen -o zen-self-2

# Compare binaries
diff zen-self zen-self-2
```

## Testing Bootstrap

### Unit Tests
Each component has comprehensive unit tests:
```bash
# Run all tests
./run_tests.sh

# Test specific component
./target/debug/zen tests/test_lexer.zen
./target/debug/zen tests/test_parser.zen
./target/debug/zen tests/test_type_system.zen
```

### Integration Tests
Full compiler integration tests:
```bash
# Test compilation pipeline
./test_bootstrap.sh
```

### Verification Suite
Ensures self-hosted compiler produces identical output:
```bash
# Run verification
./verify_bootstrap.sh
```

## Key Considerations

### Memory Management
- Manual memory management using `malloc`/`free`
- Future: Add garbage collection or RAII

### Type System
- Static typing with inference
- Generics support
- Interface-based polymorphism

### Optimization
- Stage 0: Basic optimizations in LLVM
- Stage 1: AST-level optimizations
- Stage 2: IR-level optimizations
- Stage 3: Full optimization pipeline

### Platform Support
- Initial: Linux x86_64
- Planned: macOS, Windows
- Future: ARM, WebAssembly

## Milestones

### Q1 2025
- [x] Complete lexer and parser in Zen
- [x] Implement type system
- [x] Build standard library foundation

### Q2 2025
- [ ] Complete code generator in Zen
- [ ] Achieve first self-compilation
- [ ] Release alpha version

### Q3 2025
- [ ] Optimization passes
- [ ] Platform portability
- [ ] Performance parity with Rust version

### Q4 2025
- [ ] Full self-hosting
- [ ] Production ready
- [ ] Version 1.0 release

## Contributing

To contribute to the bootstrap effort:

1. Pick a component from the "Planned" section
2. Implement in Zen following existing patterns
3. Add comprehensive tests
4. Update this documentation
5. Submit PR with frequent commits

## References

- [Compiler Architecture](./architecture.md)
- [Language Specification](./language_spec.md)
- [Standard Library](./stdlib.md)
- [Testing Guide](./testing.md)