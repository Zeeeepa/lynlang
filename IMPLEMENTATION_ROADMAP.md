# ZenLang Implementation Roadmap

## Source of Truth
**`LANGUAGE_SPEC.zen` is the authoritative specification.** All features must align with this specification.

## Current Status (As of latest commit)

### ✅ Fully Working Features
- **Core Philosophy**: No keywords, only `?` operator for control flow
- **Pattern Matching**: Complete `?` operator implementation
- **Option Type**: `Some(T)` / `None` - no null in the language
- **Result Type**: `Ok(T)` / `Err(E)` for error handling
- **Variable System**: Immutable `=`, mutable `::=`, type annotations
- **Loops**: Range `(0..N).loop()`, infinite `loop{}`
- **String Interpolation**: `"Value: ${expr}"`
- **@std Imports**: Standard library import system
- **Basic Arithmetic**: All standard operators

### 🚧 Priority 1: Core Language Features (Next Sprint)

#### 1. UFC (Uniform Function Call) - LANGUAGE_SPEC.zen line 4
**Critical**: This is a fundamental language feature
- Any function should be callable as a method
- Example: `value.double()` equivalent to `double(value)`
- Enables fluent chaining: `value.double().add_ten()`
- Implementation: Modify parser to transform method calls

#### 2. Forward Declarations - LANGUAGE_SPEC.zen lines 299-300, 303-304
- Separate declaration from assignment
- `x: i32` then later `x = 10`
- `w:: i32` then later `w = 20`
- Requires symbol table modification

#### 3. Error Propagation `.raise()` - LANGUAGE_SPEC.zen lines 206-211
- Automatic error propagation for Result types
- `file.open(path).raise()` returns early on Err
- Simplifies error handling chains

### 🚧 Priority 2: Type System Features

#### 4. Traits System - LANGUAGE_SPEC.zen lines 123-163
- `.implements()` to add trait to type
- `.requires()` to enforce trait on variants
- Foundation for polymorphism

#### 5. Pointer Types - LANGUAGE_SPEC.zen lines 363-372
- `Ptr<T>` for immutable references
- `MutPtr<T>` for mutable references
- `RawPtr<T>` for unsafe operations
- `.ref()`, `.mut_ref()` methods
- `.val` for dereferencing

### 🚧 Priority 3: Advanced Collections

#### 6. DynVec - LANGUAGE_SPEC.zen lines 316-350
- Dynamic vectors with allocator support
- Mixed-type vectors for enum variants
- Essential for real-world applications

#### 7. Step Ranges - LANGUAGE_SPEC.zen lines 437-439
- `(0..100).step(10)` syntax
- Iterator protocol enhancement

### 🚧 Priority 4: Concurrency & Advanced Features

#### 8. Allocator System - LANGUAGE_SPEC.zen lines 308-314
- Determines sync/async behavior
- No function coloring problem
- `GPA`, `AsyncPool` allocators

#### 9. Concurrency Primitives - LANGUAGE_SPEC.zen lines 397-430
- `Actor` for message passing
- `Channel` for communication
- `Mutex` for shared state
- `AtomicU32` for lock-free ops

#### 10. Metaprogramming - LANGUAGE_SPEC.zen lines 243-282
- `@meta.comptime` for compile-time execution
- AST reflection and manipulation
- Code generation capabilities

### 🚧 Priority 5: FFI & Low-Level

#### 11. Inline C/LLVM - LANGUAGE_SPEC.zen lines 285-294
- `inline.c()` for C code embedding
- Direct LLVM IR injection
- Essential for system programming

#### 12. SIMD Operations - LANGUAGE_SPEC.zen lines 291-294
- `simd.add()` and vector operations
- Performance-critical computations

## Implementation Guidelines

1. **Always refer to LANGUAGE_SPEC.zen** for feature specifications
2. **Write tests first** - all tests go in `tests/` with `zen_` prefix
3. **Update README.md** as features are completed
4. **Maintain backward compatibility** with working features
5. **Document deviations** if spec needs clarification

## Test Coverage Requirements

Each implemented feature must have:
- Unit tests for parser changes
- Integration tests showing feature usage
- Test file named `zen_test_<feature>.zen`
- Examples directly from LANGUAGE_SPEC.zen

## Definition of Done

A feature is complete when:
1. ✅ Parser recognizes syntax from LANGUAGE_SPEC.zen
2. ✅ Type checker validates semantics
3. ✅ Code generator produces correct LLVM IR
4. ✅ Tests pass including spec examples
5. ✅ README.md updated with working examples
6. ✅ No regression in existing tests

## Next Immediate Steps

1. **Implement UFC**: Start with parser modification for method syntax
2. **Add forward declarations**: Modify symbol table to track declaration state
3. **Implement .raise()**: Add Result type propagation in type checker
4. **Create test suite**: One test file per LANGUAGE_SPEC.zen section

## Notes

- The compiler is already generating LLVM IR successfully
- Basic type system and pattern matching work well
- Focus should be on language features that enable real programs
- UFC is the most important missing feature as it's fundamental to the language design