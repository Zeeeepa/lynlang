# Zen Programming Language

**`LANGUAGE_SPEC.zen` IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS** - everything is pattern matching, uniform function calls, and compile-time metaprogramming. The complete language specification is defined in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

## Core Philosophy

From `LANGUAGE_SPEC.zen` (lines 1-14):

```zen
// No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`
// Only two @ symbols: `@std` (standard library) and `@this` (current scope)
// Pattern matching with `?` operator, no `match` or `switch`
// UFC (Uniform Function Call) - any function can be called as method
// Allocators determine sync/async behavior (no function coloring)
// Explicit pointer types: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
// No null/nil - only `Option<T>` with `.Some(T)` and `.None`
// No unions, no tuples - only structs and enums
// Assignment operators: `=` (immutable), `::=` (mutable), `:` (type definition)
// Error propagation with `.raise()` not exceptions
// Loops: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
// Traits via `.implements()` and `.requires()` from `@std.meta`
// Compile-time metaprogramming with full AST access
```

## Quick Start

### Build the Compiler
```bash
cargo build --release
```

### Run Zen Programs
```bash
# Run directly
./target/release/zen program.zen

# Start REPL
./target/release/zen
```

### Run Test Suite
```bash
# All tests are in tests/ folder with zen_ prefix
./target/release/zen tests/zen_test_hello_world.zen
./target/release/zen tests/zen_test_working_showcase.zen
```

## Implementation Status

Based on `LANGUAGE_SPEC.zen`, here's what's working, partially implemented, and planned:

### ✅ Working Features

These features from `LANGUAGE_SPEC.zen` are fully functional:

| Feature | Spec Lines | Example |
|---------|------------|---------|
| **No Keywords** | 1-2 | All control flow uses pattern matching with `?` |
| **Pattern matching `?`** | 3-4, 29-71, 352-361 | `bool ? \| true {} \| false {}` |
| **UFC (Uniform Function Call)** | 5, 172-182 | `value.function()` → `function(value)` with chaining |
| **No null - Option type** | 8, 109-110, 462-473 | `Option<T>: Some(T) \| None` |
| **Result type** | 112-113, 199-211 | `Result<T, E>: Ok(T) \| Err(E)` |
| **Variable declarations** | 10, 298-306 | `=` immutable, `::=` mutable, `:` type |
| **Loops and ranges** | 11, 432-460 | `(0..10).loop()`, `loop { ... }` |
| **String interpolation** | Throughout | `"Value: ${expr}"` |
| **Structs** | 117-120, 364-372 | `Point: { x:: f64, y:: f64 = 0 }` |
| **Enums** | 165-182 | `Shape: Circle \| Rectangle` |
| **Functions & closures** | Throughout | First-class functions with type inference |
| **@std.io imports** | 92-94 | `{ io } = @std` |
| **Forward declarations** | 299-300 | `x: i32` then `x = 10` |
| **Boolean conditions** | 352-361 | `is_ready ? { action() }` |

### 🔧 Partially Implemented

These features are parsed but not fully working in codegen:

| Feature | Spec Lines | Status |
|---------|------------|--------|
| **@std.math module** | 93-94, 138-139 | Module exists but constants like `pi` not accessible |
| **Pointer types** | 6-7, 364-372 | `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` parsed |
| **.raise() propagation** | 10, 206-211 | Parsed but type issues in codegen |
| **@this.defer()** | 217, 314 | Parsed, needs runtime support |
| **Step ranges** | 437-439 | `(0..100).step(10)` parsed |

### ❌ Not Yet Implemented

Features from `LANGUAGE_SPEC.zen` that need implementation:

| Feature | Spec Lines | Description |
|---------|------------|-------------|
| **Traits .implements()** | 12-13, 136-143 | `Circle.implements(Geometric, {...})` |
| **Traits .requires()** | 12-13, 168 | `Shape.requires(Geometric)` |
| **Generic functions** | 185-188 | `print_area<T: Geometric>(shape: T)` |
| **Generic containers** | 190-196 | `Container<T: Geometric + Serializable>` |
| **DynVec** | 101, 317-350, 377-384 | Dynamic vectors with allocator |
| **Vec<T, N>** | 101, 292-293, 374 | Static-sized vectors |
| **Allocators** | 5-6, 99, 213-224, 309-314 | Sync/async behavior determination |
| **Actors** | 104, 228-240, 401-406 | Lazy iteration and concurrency |
| **Channels** | 104, 397-412 | Message passing |
| **Mutex** | 104, 415-424 | Shared state synchronization |
| **AtomicU32** | 104, 426-429 | Atomic operations |
| **StringBuilder** | 95, 387-394 | String building utilities |
| **AST reflection** | 13-14, 97, 243-272 | `reflect.ast(T)` for metaprogramming |
| **@meta.comptime** | 13-14, 97, 274-281 | Compile-time code generation |
| **Inline C/LLVM** | 285-289 | FFI for low-level control |
| **SIMD operations** | 97, 291-294 | `simd.add(a, b)` vector operations |
| **Module imports** | 105-106 | `@std.import("sdl2")` |
| **Module exports** | 491-502 | `module.exports = { ... }` |
| **Build system** | 19-85 | `build.zen` configuration |

## Example Programs

### Hello World (Working)
```zen
{ io } = @std

main = () void {
    io.println("Hello, World!")
}
```

### Pattern Matching (Working)
```zen
{ io } = @std

Option<T>: Some(T) | None

main = () void {
    maybe: Option<i32> = Some(42)
    maybe ?
        | Some(v) { io.println("Value: ${v}") }
        | None { io.println("No value") }
    
    is_ready = true
    is_ready ? {
        io.println("Ready!")
    }
}
```

### UFC - Uniform Function Call (Working)
```zen
{ io } = @std

double = (n: i32) i32 { return n * 2 }
add = (x: i32, y: i32) i32 { return x + y }

main = () void {
    result = 10.double().add(5)  // Chaining: double(10) then add(20, 5)
    io.println("Result: ${result}")  // Output: 25
}
```

### Loops and Ranges (Working)
```zen
{ io } = @std

main = () void {
    // Range loop
    (0..5).loop((i) {
        io.println("Count: ${i}")
    })
    
    // Infinite loop with break
    counter ::= 0
    loop(() {
        counter = counter + 1
        counter > 3 ?
            | true { break }
            | false { io.println("Loop ${counter}") }
    })
}
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen      # The source of truth - complete language specification
├── src/
│   ├── main.rs            # Entry point and REPL
│   ├── compiler.rs        # Compilation orchestration
│   ├── lexer.rs           # Tokenization
│   ├── parser/            # AST construction
│   ├── ast/               # AST type definitions
│   ├── codegen/llvm/      # LLVM backend
│   ├── typechecker/       # Type checking
│   └── stdlib/            # Standard library modules
├── tests/                 # All test files (zen_*.zen)
└── examples/              # Example programs
```

## Development Status

The Zen compiler is actively being developed to fully implement `LANGUAGE_SPEC.zen`. Current priorities:

1. **Module System**: Fix `@std.math` and other module access
2. **Traits**: Implement `.implements()` and `.requires()`
3. **Generics**: Add support for generic functions and types
4. **Collections**: Implement `DynVec` and `Vec` types
5. **Concurrency**: Add `Actor`, `Channel`, `Mutex` support
6. **Metaprogramming**: Implement AST reflection and compile-time execution

See [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md) for detailed progress tracking.

## Contributing

The goal is to make `LANGUAGE_SPEC.zen` a reality. All contributions should align with the specification defined there.

## License

MIT