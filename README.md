# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and compile-time metaprogramming.

> "No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`"  
> — LANGUAGE_SPEC.zen, line 2

## Core Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

1. **No keywords** - No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
2. **Only two @ symbols** - `@std` (standard library) and `@this` (current scope)
3. **Pattern matching with `?`** - Replaces all conditional keywords
4. **UFC (Uniform Function Call)** - Any function can be called as method
5. **Allocators determine sync/async** - No function coloring problem
6. **Explicit pointer types** - `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
7. **No null/nil** - Only `Option<T>` with `.Some(T)` and `.None`
8. **No unions, no tuples** - Only structs and enums
9. **Assignment operators** - `=` (immutable), `::=` (mutable), `:` (type annotation)
10. **Error propagation** - `.raise()` not exceptions
11. **Loops** - `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
12. **Traits** - Via `.implements()` and `.requires()` from `@std.meta`
13. **Compile-time metaprogramming** - Full AST access

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen examples/01_hello_world.zen

# Start the REPL
./target/release/zen

# Run tests
./target/release/zen tests/zen_test_spec_working_complete.zen
```

## Language Examples

### No Keywords - Only Pattern Matching

```zen
// No if/else - use ? operator
is_ready ?
    | true { io.println("Ready!") }
    | false { io.println("Not ready") }

// Single branch pattern
has_data ? { process() }
```

### Variables - Six Forms

```zen
// Immutable
x: i32          // forward declaration
x = 10          // assignment
y = 20          // inferred type
z: i32 = 30     // with type

// Mutable (:: prefix)
w:: i32         // mutable forward declaration  
w = 40          // assignment
v ::= 50        // mutable with inference
u:: i32 = 60    // mutable with type
```

### Loops Without Keywords

```zen
// Range loop (no 'for')
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Infinite loop (no 'while')
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { }
})

// Collection iteration
shapes.loop((shape) {
    io.println("Area: ${shape.area()}")
})
```

### No Null - Only Option

```zen
Option<T>: Some(T) | None

maybe_value: Option<i32> = Some(42)
maybe_value ?
    | Some(val) { io.println("Value: ${val}") }
    | None { io.println("No value") }
```

### Error Handling with Result

```zen
Result<T, E>: Ok(T) | Err(E)

parse_number = (s: string) Result<i32, string> {
    // Parsing logic...
    valid ?
        | true { return Ok(42) }
        | false { return Err("Invalid") }
}

// Error propagation with .raise()
process = () Result<Data, Error> {
    data = fetch().raise()  // Returns early if Err
    parsed = parse(data).raise()
    return Ok(parsed)
}
```

### Traits via .implements()

```zen
// Define trait
Geometric: {
    area: (self) f64,
    perimeter: (self) f64
}

// Implement for type
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    }
})
```

### UFC and Function Overloading

```zen
GameEntity: Player | Enemy | Powerup

// Overload for each variant
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }

// UFC - call as method
player = GameEntity.Player
health = player.get_health()  // 100
```

## Implementation Status

The compiler currently implements approximately **70%** of the language specification.

### ✅ Working Features

- **Core Language**: No keywords, pattern matching with `?`
- **Variables**: All 6 forms from spec (immutable/mutable, forward declarations)
- **Pattern Matching**: Boolean, Option, Result, custom enums
- **Loops**: Range loops `(0..10).loop()`, infinite `loop()` with break
- **String Interpolation**: With support for all types
- **Standard Library**: Basic `@std` with `io`, `math`
- **Arithmetic**: Full operator support
- **Functions**: Basic functions, some UFC support

### 🚧 In Progress

- **Structs**: Basic structs work, nested field access needs improvement
- **Traits**: `.implements()` partially working, `.requires()` not implemented
- **Generics**: Type parameters `<T: Trait>` not yet supported
- **Pointers**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` not implemented
- **Error Propagation**: `.raise()` not fully implemented
- **Allocators**: Sync/async behavior not implemented
- **Metaprogramming**: AST manipulation not available

### ❌ Not Implemented

- `@this` scope reference
- `.defer()` cleanup
- Step ranges `.step(n)`
- Collection `.loop()` on custom types
- Full UFC for all types
- Compile-time evaluation
- FFI and C interop
- Actor model for concurrency

## Test Suite

The main test validating LANGUAGE_SPEC.zen features:

```bash
# Test working features
./target/release/zen tests/zen_test_spec_working_complete.zen

# Run all tests
./scripts/test_all.sh
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen       # The source of truth
├── src/                    # Rust compiler implementation
│   ├── lexer.rs           # Tokenization
│   ├── parser/            # AST generation
│   ├── typechecker/       # Type checking
│   ├── codegen/           # LLVM code generation
│   └── stdlib/            # Built-in standard library
├── tests/                  # Test suite (prefix with zen_test_)
├── examples/              # Example programs
└── stdlib/                # Zen standard library sources
```

## Contributing

This project implements the specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen). All contributions must align with this specification. When adding features:

1. Reference the spec line numbers
2. Add tests prefixed with `zen_test_` in the `tests/` folder
3. Update implementation status in this README
4. Ensure no keywords are introduced

## Philosophy

Zen demonstrates that a powerful, expressive programming language can exist without any keywords. Every language construct traditionally requiring keywords (conditionals, loops, error handling, traits) is achieved through:

- Pattern matching with `?`
- Method-style function calls (UFC)
- Compile-time metaprogramming
- Library-defined abstractions

This creates a minimal, orthogonal language where everything composes naturally.

## License

MIT

## Links

- [Language Specification](./LANGUAGE_SPEC.zen)
- [Implementation Status](./IMPLEMENTATION_STATUS_2025.md)
- [Working Features Demo](./tests/zen_test_spec_working_complete.zen)