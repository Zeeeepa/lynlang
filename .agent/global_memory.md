# Zenlang Project Global Memory

## Project Status (Last Updated: 2025-01-11)

### Language Implementation Status
- **Core Language**: Functional
  - Lexer: ✅ Complete
  - Parser: ✅ Complete  
  - Type System: ✅ Complete with monomorphization
  - Code Generation: ✅ LLVM backend working
  - FFI: ✅ Builder pattern implemented
  - LSP: ✅ Basic implementation complete

### Key Features Implemented
1. **Pattern Matching (`?` operator)**: Full support for all pattern types
2. **Behaviors**: Structural typing system replacing traits
3. **Colorless Async**: Allocator-based async without function coloring
4. **Module System**: `@std` namespace with build system integration
5. **Memory Management**: Smart pointers (Ptr<T>, Ref<T>)
6. **FFI Builder Pattern**: Safe C interop with extensive configuration

### Test Status
- **Total Tests**: 358 passing
- **Ignored Tests**: 174 (mostly self-hosting related)
- **Failed Tests**: 0
- **Build Time**: ~36 seconds (release mode)
- **Memory Usage**: Stable at ~6.6GB during build

### Current Architecture
```
src/
├── ast.rs           - AST definitions
├── behaviors.rs     - Behavior system implementation
├── codegen/        
│   ├── mod.rs      - Main codegen module
│   ├── llvm.rs     - LLVM backend
│   └── behaviors.rs - Behavior codegen
├── error.rs        - Error handling
├── ffi/            
│   └── mod.rs      - FFI builder pattern
├── lexer.rs        - Tokenization
├── lsp/            
│   ├── main.rs     - LSP entry point
│   ├── mod.rs      - Core LSP server
│   └── enhanced.rs - Enhanced features
├── parser.rs       - Parsing implementation
├── runtime.rs      - Runtime support
├── stdlib.rs       - Standard library
└── type_system/    
    ├── mod.rs      - Type checking
    └── monomorphization.rs - Generic instantiation
```

### Critical Rules (from LANGUAGE_SPEC.md)
- **NO** `if`/`else`/`match` keywords - Use `?` operator exclusively
- **NO** exceptions - All errors are values
- **NO** null pointers - Use Option<T>
- **NO** lifetime annotations - Smart pointers handle safety
- **NO** raw `&` or `*` - Use Ptr<T> and .value/.address
- **NO** tuples - Use structs for all product types

### Recent Work (2025-01-11)
1. ✅ Reviewed and confirmed FFI builder pattern implementation
2. ✅ Verified LSP builds and runs correctly
3. ✅ Fixed test compilation errors
4. ✅ Confirmed 358 tests passing with no failures
5. ✅ Verified reasonable memory usage during builds

### Next Priority Tasks
1. Enable more self-hosting tests
2. Complete stdlib module implementations
3. Improve LSP features (hover, completion, etc.)
4. Add more comprehensive FFI tests
5. Optimize build times and memory usage

### Known Issues
- Some self-hosting tests are ignored pending full implementation
- Build can occasionally cause high memory usage (monitor for OOM)
- LSP needs more advanced features like code completion

### Development Notes
- Use `cargo test --no-fail-fast` to run all tests
- Monitor memory with `free -h` during builds
- FFI builder pattern follows Language Spec v1.1.0 requirements
- LSP server runs with `cargo run --bin zen-lsp`