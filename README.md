# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source of truth**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching, UFC (Uniform Function Call), and no function coloring.

## Key Design Principles (from LANGUAGE_SPEC.zen)

```zen
// - No keywords: if/else/while/for/match/async/await/impl/trait/class/interface/null
// - Only two @ symbols: @std (standard library) and @this (current scope)
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

## Current Implementation Status

### ✅ Working Features (70% Complete)

#### 1. Variable Declarations (All 6 Forms)
```zen
x: i32              // forward declaration
x = 10              // immutable assignment
y = 20              // immutable inference
z: i32 = 30         // immutable with type
w:: i32             // mutable forward declaration
w = 40              // mutable assignment
v ::= 50            // mutable inference  
u:: i32 = 60        // mutable with type
```

#### 2. UFC (Uniform Function Call)
```zen
double = (n: i32) i32 { return n * 2 }
result = 5.double()        // UFC: transforms to double(5)
chained = 5.double().triple() // Chaining!
```

#### 3. Pattern Matching with `?` (No keywords!)
```zen
// Boolean short form
is_ready ? { io.println("Go!") }

// Full pattern matching  
has_data ?
    | true { process_data() }
    | false { wait_for_data() }

// Option matching
maybe: Option<i32> = Some(42)
maybe ?
    | Some(n) { io.println("Value: ${n}") }
    | None { io.println("Empty") }
```

#### 4. @this.defer for Cleanup
```zen
main = () void {
    io.println("Starting")
    @this.defer(io.println("Cleaning up!"))
    io.println("Working...")
    // Output: Starting, Working..., Cleaning up!
}
```

#### 5. Option & Result Types (No null!)
```zen
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)

maybe: Option<i32> = Some(42)
nothing: Option<i32> = None
success: Result<i32, string> = Ok(100)
failure: Result<i32, string> = Err("error")
```

#### 6. Traits via .implements() and .requires() ✨
```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation
Circle.implements(Geometric, {
    area = (self) f64 {
        return 3.14159 * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * 3.14159 * self.radius
    },
})

// Require all Shape variants to implement Geometric
Shape.requires(Geometric)
```

#### 7. Error Propagation with .raise()
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // Returns early if Err
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

#### 8. Ranges and Loops
```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Infinite loop
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

#### 9. String Interpolation
```zen
name = "Zen"
version = 2025
io.println("Welcome to ${name} v${version}!")
```

#### 10. Structs and Enums
```zen
Point: {
    x:: f64,      // mutable field
    y:: f64 = 0.0 // with default
}

Shape: Circle | Rectangle  // Enum type
```

### 🚧 Not Yet Implemented (30% TODO)

From LANGUAGE_SPEC.zen:

- **Module System**: `module.exports`, `module.import`
- **Concurrency**: Actor, Channel, Mutex, AtomicU32
- **Allocator-based sync/async**: GPA, AsyncPool (no function coloring!)
- **Metaprogramming**: @meta.comptime with AST access
- **Reflection**: reflect.ast() for runtime reflection
- **Inline blocks**: inline.c(), inline.llvm()
- **SIMD**: simd.add() and vector operations
- **StringBuilder**: For efficient string building
- **DynVec**: Dynamic vectors with mixed types
- **FFI**: Foreign Function Interface

## Quick Start

```zen
{ io } = @std

main = () void {
    // No keywords! Only pattern matching
    is_ready = true
    is_ready ? { io.println("Starting!") }
    
    // UFC - any function as method
    double = (n: i32) i32 { return n * 2 }
    result = 5.double()
    io.println("5 doubled = ${result}")
    
    // No null - only Option types
    maybe: Option<i32> = Some(42)
    maybe ?
        | Some(value) { io.println("Got: ${value}") }
        | None { io.println("Got nothing") }
}
```

## Building & Running

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen main.zen

# Run tests
./target/release/zen tests/zen_test_language_spec_showcase_2025.zen
./target/release/zen tests/zen_test_trait_basic_demo.zen
```

## Language Spec Tests

The following tests demonstrate LANGUAGE_SPEC.zen features:
- `tests/zen_test_language_spec_showcase_2025.zen` - Comprehensive feature test
- `tests/zen_test_trait_basic_demo.zen` - Trait implementation
- `tests/zen_test_ufc_spec.zen` - UFC (Uniform Function Call)
- `tests/zen_test_spec_working_features.zen` - Working features

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen     # The source of truth
├── src/                  # Rust compiler implementation
│   ├── parser/          # Zen parser
│   ├── codegen/llvm/    # LLVM code generation
│   ├── typechecker/     # Type system
│   └── stdlib/          # Standard library
├── stdlib/              # Zen standard library
└── tests/               # Test suite (zen_test_*.zen)
```

## Contributing

All implementations MUST match [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) exactly. The spec is the authoritative source of truth for the language.

## Philosophy

Zen eliminates complexity through radical simplicity:
- **No keywords** means no special cases
- **UFC everywhere** means consistent syntax
- **No null** means no null pointer exceptions
- **Pattern matching** replaces all control flow
- **Allocators** determine sync/async, not function signatures

## License

MIT