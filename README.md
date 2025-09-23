# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source of truth**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching, UFC (Uniform Function Call), and no function coloring.

## Core Philosophy (from LANGUAGE_SPEC.zen)

```zen
// Zen Language - Key Design Principles:
// - No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`
// - Only two @ symbols: `@std` (standard library) and `@this` (current scope)
// - Pattern matching with `?` operator, no `match` or `switch`
// - UFC (Uniform Function Call) - any function can be called as method
// - Allocators determine sync/async behavior (no function coloring)
// - Explicit pointer types: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
// - No null/nil - only `Option<T>` with `.Some(T)` and `.None`
// - No unions, no tuples - only structs and enums
// - Assignment operators: `=` (immutable), `::=` (mutable), `:` (type definition)
// - Error propagation with `.raise()` not exceptions
// - Loops: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
// - Traits via `.implements()` and `.requires()` from `@std.meta`
// - Compile-time metaprogramming with full AST access
```

## Implementation Status

### ✅ Fully Working (40% of spec implemented)
- **Zero keywords philosophy** - All control flow via pattern matching
- **Pattern matching with `?`** - Boolean and enum patterns  
- **UFC (Uniform Function Call)** - Method chaining works perfectly
- **Option type** - `Some(T) | None` with no null
- **Result type** - `Ok(T) | Err(E)` for errors
- **Variable declarations** - All 6 forms working:
  - Forward declaration: `x: i32` then `x = 10`
  - Immutable: `y = 20` and `z: i32 = 30`
  - Mutable forward: `w:: i32` then `w = 40`
  - Mutable: `v ::= 50` and `u:: i32 = 60`
- **String interpolation** - `"Value: ${expr}"` throughout
- **Structs** - With mutable fields working
- **Enums** - Sum types with pattern matching
- **Loops & ranges** - `(0..10).loop()` and `loop(() { ... })`
- **Functions** - First-class with closures
- **@std imports** - `{ io, math } = @std`
- **@std.math.pi** - Math constants working
- **Array literals** - `[1, 2, 3]` with `.loop()` method

### 🔧 Partially Working
- **Traits** - `.implements()` and `.requires()` parsed, self parameter fixed, method dispatch pending
- **.raise() error propagation** - Parsed but has type issues in codegen
- **Pointer types** - `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` parsed but not fully working

### ❌ Not Yet Implemented (Roadmap)
1. **Traits** - Complete `.implements()` and `.requires()` with method dispatch
2. **Generic functions and types** - `<T: Trait>`
3. **DynVec and Vec types** - Dynamic and static vectors
4. **Allocators** - For sync/async determination
5. **Actor system** - For lazy iteration and concurrency
6. **Channels, Mutex, Atomics** - Concurrency primitives
7. **AST reflection** - Runtime metaprogramming with `reflect.ast()`
8. **@meta.comptime** - Compile-time code generation
9. **@this.defer** - Cleanup at scope end
10. **Inline C/LLVM** - FFI integration
11. **SIMD operations** - Vector math
12. **Module system** - `module.exports` and `module.import`
13. **Build system** - Full `build.zen` support

## Quick Examples

### Pattern Matching (No if/else!)
```zen
{ io } = @std

main = () void {
    value: i32 = 42
    is_answer: bool = value == 42
    
    // Boolean pattern matching
    is_answer ?
        | true { io.println("Found the answer!") }
        | false { io.println("Keep searching...") }
}
```

### Option Types (No null!)
```zen
Option<T>: Some(T) | None

parse_number = (s: string) Option<i32> {
    s.is_numeric() ?
        | true { return Some(s.to_i32()) }
        | false { return None }
}
```

### UFC - Uniform Function Call
```zen
double = (n: i32) i32 { return n * 2 }
add = (x: i32, y: i32) i32 { return x + y }

main = () void {
    // All equivalent:
    result = add(double(10), 5)      // Traditional
    result = 10.double().add(5)      // UFC chaining
    result = double(10).add(5)       // Mixed style
}
```

### Traits and Implementations (In Progress)
```zen
// Trait definition - methods that types can implement
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

Circle: {
    center: Point,
    radius: f64,
}

Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})
```

### Loops and Ranges
```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { continue }
})
```

## Building & Running

### Build Compiler
```bash
cargo build --release
```

### Run Zen Programs
```bash
# Execute a program
./target/release/zen program.zen

# Interactive REPL
./target/release/zen
```

### Test Suite
```bash
# Core working features
./target/release/zen tests/zen_test_working_showcase.zen

# Hello world
./target/release/zen tests/zen_test_hello_world.zen

# Math constants test
./target/release/zen tests/zen_test_math_pi.zen
```

## Project Structure
```
zenlang/
├── LANGUAGE_SPEC.zen      # Source of truth - full language specification
├── IMPLEMENTATION_STATUS.md # Detailed implementation progress
├── src/
│   ├── parser/            # Zero-keyword parser
│   ├── typechecker/       # Type checking with traits
│   ├── codegen/llvm/      # LLVM code generation
│   └── stdlib/            # Standard library (@std)
└── tests/
    └── zen_test_*.zen     # Test files prefixed with zen_test_
```

## Philosophy Deep Dive

### Why Zero Keywords?
Traditional languages burden developers with dozens of keywords that could be functions or operators. Zen proves that a complete, expressive language needs **zero keywords**.

### Pattern Matching Everything
The `?` operator enables pattern matching on any expression, replacing:
- `if/else` statements
- `switch/match` expressions  
- Ternary operators
- Null checks

### UFC (Uniform Function Call)
Any function can be called as a method on its first argument:
```zen
// These are all equivalent:
length(string)
string.length()
"hello".length()
```

### No Function Coloring
Sync/async behavior is determined by allocators, not function signatures:
```zen
fetch_data = (url: string, alloc: Allocator) Data {
    // Same function works sync or async based on allocator!
    return http.get(url, alloc)
}

// Synchronous call
data = fetch_data("api.com", SyncAllocator)

// Asynchronous call  
data = fetch_data("api.com", AsyncAllocator)
```

## Contributing

The language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the source of truth. All implementations must conform to this spec.

## License

MIT - See LICENSE file for details.