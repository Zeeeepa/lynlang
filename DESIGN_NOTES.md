# Zen Language Design Notes

**Last Updated: 2025-01-27**

This document consolidates the architectural decisions, design rationale, and implementation notes for the Zen programming language.

## 🎯 Core Design Principles

### Zero Keywords Philosophy
Zen eliminates all traditional keywords in favor of pattern matching and expression-based control flow:

- **No `if/else/while/for/match/async/await/impl/trait/class/interface/null`**
- **Pattern matching with `?`** replaces all conditional keywords
- **UFC (Uniform Function Call)** enables method-style syntax for any function
- **Allocator-driven concurrency** eliminates function coloring

### Expression-First Design
Every construct in Zen is an expression, enabling powerful composition:

```zen
// Traditional: if (condition) { action() }
condition ? { action() }

// Traditional: match value { Some(x) => x, None => 0 }
value ? | Some(x) => x | None => 0

// Traditional: for (i in 0..10) { process(i) }
(0..10).loop((i) { process(i) })
```

## 🏗️ Architecture Decisions

### 1. Generic Type System

#### Design Rationale
The generic type system was designed to support nested generics while maintaining type safety and performance. The implementation uses heap allocation for complex nested types to avoid stack overflow issues.

#### Current Implementation
- **Basic Generics**: `Option<T>`, `Result<T,E>`, `HashMap<K,V>` fully functional
- **Nested Generics**: Partial support with known limitations
- **Type Inference**: Variables don't track generic types properly (known issue)

#### Technical Challenges
1. **Payload Extraction**: Nested generics require careful payload extraction
2. **Type Context Tracking**: Variables lose type information through assignments
3. **Multiple raise()**: Second raise() fails with type errors

#### Future Improvements
- Enhanced variable type tracking for generic types
- Complete nested generic support
- Better type inference for complex scenarios

### 2. Error Handling with .raise()

#### Design Rationale
Zen uses `.raise()` for error propagation instead of exceptions, providing:
- **Explicit error handling** - no hidden control flow
- **Type safety** - errors are part of the type system
- **Performance** - no exception overhead

#### Implementation Status
- **Basic .raise()**: Works for single-level Result/Option
- **Nested .raise()**: Partial support with type tracking issues
- **Error Propagation**: Correctly propagates errors up the call stack

#### Known Issues
- Multiple `.raise()` calls fail with type errors
- Variable type inference doesn't preserve generic types
- Complex nested error handling needs improvement

### 3. Allocator-Driven Concurrency

#### Design Rationale
Concurrency is determined by the allocator used, not function coloring:
- **Sync Allocator (GPA)**: All operations block
- **Async Allocator (AsyncPool)**: Non-blocking operations
- **No async/await keywords**: Eliminates function coloring

#### Implementation Status
- **Basic Allocators**: GPA and AsyncPool implemented
- **Allocator Selection**: Functions can use different allocators
- **Concurrency Primitives**: Partial implementation

#### Future Work
- Complete actor model implementation
- Channel-based concurrency
- Advanced allocator strategies

### 4. Pattern Matching System

#### Design Rationale
Pattern matching with `?` replaces all conditional constructs:
- **Unified syntax** for all conditionals
- **Type-safe pattern matching** with exhaustiveness checking
- **Expression-based** - every pattern match returns a value

#### Implementation Status
- **Basic Patterns**: Boolean, enum patterns working
- **Nested Patterns**: Partial support for complex types
- **Exhaustiveness**: Basic checking implemented

#### Technical Details
```zen
// Boolean patterns
condition ? { action() }

// Enum patterns
value ? | Some(x) => x | None => 0

// Nested patterns
result ? | Ok(inner) => inner ? | Some(x) => x | None => 0
```

### 5. UFC (Uniform Function Call)

#### Design Rationale
UFC allows any function to be called as a method:
- **Consistent API** - same function can be called both ways
- **Method chaining** - enables fluent interfaces
- **No special syntax** - works with any function

#### Implementation Status
- **Basic UFC**: Method chaining working
- **Function Overloading**: Based on enum variants
- **Complex UFC**: Needs completion for all function types

#### Example
```zen
// Both calls are equivalent
shape.area()
area(shape)

// Method chaining
result.map(transform).filter(predicate).collect()
```

## 🔧 Implementation Challenges

### 1. Nested Generic Payload Extraction

#### Problem
When extracting values from nested generics like `Result<Result<i32, string>, string>`:
- ✅ First level extraction works (gets inner Result as struct)
- ✅ Pattern matching on inner Result works (discriminant check passes)
- ❌ Final payload extraction returns struct instead of actual value

#### Root Cause
The payload extraction logic correctly preserves nested enums as structs for recursive pattern matching, but when the final pattern is just a variable binding, it doesn't extract the actual payload from the struct.

#### Solution Approach
Distinguish between:
1. **Intermediate payload extraction** (keep as struct for next match)
2. **Final payload extraction** (extract actual value for binding)

### 2. Range Loop Parser Issue

#### Problem
`(0..10).loop((i) { ... })` only executes once instead of iterating properly.

#### Root Cause
Parser issue where `(range).loop()` is not generating the correct AST for method calls on range expressions.

#### Solution Approach
- Fix parser to generate correct MethodCall AST for range expressions
- Ensure loop body executes for each iteration
- Handle both exclusive and inclusive ranges

### 3. Type System Integration

#### Problem
Generic types lose type information through variable assignments and function calls.

#### Root Cause
- `infer_expression_type()` doesn't handle raise() results properly
- Generic types not tracked in `VariableInfo` struct
- Type context updates not propagated correctly

#### Solution Approach
- Enhance variable type tracking to preserve generic information
- Fix `infer_expression_type()` for raise() expressions
- Improve type context propagation in variable assignments

## 📊 Performance Considerations

### 1. Heap Allocation for Nested Generics

#### Design Decision
Use heap allocation for complex nested types to avoid stack overflow issues.

#### Trade-offs
- **Pros**: Prevents stack overflow, enables complex nested types
- **Cons**: Memory allocation overhead, garbage collection needed

#### Future Optimizations
- Stack allocation for simple nested types
- Custom allocators for different use cases
- Compile-time optimization for known patterns

### 2. Type System Performance

#### Current Approach
- Runtime type checking for generics
- Heap allocation for complex types
- Pattern matching with discriminant checks

#### Future Improvements
- Compile-time generic monomorphization
- Optimized pattern matching
- Type erasure for simple cases

## 🚀 Future Architecture Plans

### 1. Metaprogramming System

#### Design Goals
- **Compile-time AST manipulation** - Full AST access
- **Macro system** - Code generation and transformation
- **Reflection** - Runtime type inspection

#### Implementation Plan
- Extend `@meta` token support
- Add compile-time evaluation
- Implement AST transformation APIs

### 2. Advanced Concurrency

#### Design Goals
- **Actor model** - Message passing concurrency
- **Channels** - CSP-style concurrency
- **Advanced allocators** - Specialized concurrency patterns

#### Implementation Plan
- Complete actor model implementation
- Add channel primitives
- Implement advanced allocator strategies

### 3. Complete Generic System

#### Design Goals
- **Full nested generic support** - Complete type tracking
- **Generic constraints** - Bounds and requirements
- **Advanced type inference** - Complex generic scenarios

#### Implementation Plan
- Fix variable type tracking
- Complete nested generic support
- Add generic constraints and bounds

## 📚 Design References

### Influences
- **Rust**: Type system and memory safety
- **Haskell**: Pattern matching and functional programming
- **Erlang**: Actor model and fault tolerance
- **Go**: Simple concurrency model

### Unique Contributions
- **Zero keywords** - Complete elimination of reserved words
- **Allocator-driven concurrency** - No function coloring
- **Expression-first design** - Everything is an expression
- **UFC system** - Uniform function call syntax

## 🔍 Technical Debt

### High Priority
1. **Fix range loop parser issue** - Critical for basic functionality
2. **Complete nested generic support** - Core language feature
3. **Resolve segfaults** - Stability issue

### Medium Priority
1. **Improve type system integration** - Better generic support
2. **Complete error handling** - Full .raise() support
3. **Enhance pattern matching** - Complex nested patterns

### Low Priority
1. **Performance optimizations** - Generic type handling
2. **Advanced features** - Metaprogramming, concurrency
3. **Documentation** - Complete API documentation

---

*This document is updated as the language evolves and new architectural decisions are made.*
