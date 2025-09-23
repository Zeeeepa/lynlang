# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and compile-time metaprogramming.

> "No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`"  
> — LANGUAGE_SPEC.zen, line 2

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen examples/01_hello_world.zen

# Run the comprehensive validation test
./target/release/zen tests/zen_test_language_spec_validated_2025.zen

# Start the REPL
./target/release/zen
```

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

## Implementation Status (Based on LANGUAGE_SPEC.zen)

### ✅ Fully Working (Lines from LANGUAGE_SPEC.zen)

- **No Keywords** (L2): Pattern matching replaces all control flow
- **@std imports** (L92-107): Standard library access
- **Variable Declarations** (L298-306): All forms (`=`, `::=`, `:`, `::`)
- **Structs** (L117-163): Definition with mutable fields and defaults
- **Enums** (L165-166): Sum types with variants
- **Option Type** (L109-110): `Some(T) | None` - no null
- **Result Type** (L112-113): `Ok(T) | Err(E)` for errors
- **Pattern Matching** (L4): `?` operator for all conditionals
- **Traits** (L124-163): `.implements()` for trait implementation
- **Trait Requirements** (L168): `.requires()` for enum constraints
- **Loops** (L431-459): Range loops `(0..10).loop()`, infinite `loop()`
- **String Interpolation**: `"Value: ${expr}"`
- **Basic UFC** (L5): Methods on structs work

### 🚧 Partially Working

- **Error Propagation** (L205-211): `.raise()` syntax recognized but not fully functional
- **Pointers** (L363-371): Types defined but operations limited
- **Collections**: `Vec<T>` works, `DynVec<T>` partial
- **Generics** (L184-196): Basic type parameters work
- **@this** (L217, L484): `@this.defer()` recognized but limited

### ❌ Not Yet Implemented

- **UFC Overloading** (L171-181): Enum variant specialization
- **Compile-time Metaprogramming** (L274-281): `@meta.comptime`, AST reflection
- **Actors & Channels** (L226-239, L399-412): Concurrency primitives
- **Allocator-based Async** (L214-224): Sync/async determined by allocator
- **Inline C/LLVM** (L284-289): Foreign code embedding
- **SIMD Operations** (L291-294): Vector operations
- **Range Step** (L436-439): `(0..100).step(10)`
- **Build System** (L19-85): `Build.zen` configuration
- **Module Exports** (L491-510): `module.exports` syntax

## Working Examples

### No Keywords - Everything is Pattern Matching

```zen
// From LANGUAGE_SPEC.zen lines 351-361
value = true
value ?
    | true { io.println("True path") }
    | false { io.println("False path") }

// Single branch (line 353)
is_ready ? { start_game() }

// Option type (lines 461-473)
maybe_value: Option<i32> = Some(42)
maybe_value ?
    | Some(v) { io.println("Value: ${v}") }
    | None { io.println("No value") }
```

### Variable Declaration Forms

```zen
// From LANGUAGE_SPEC.zen lines 298-306
x: i32          // Forward declaration
x = 10          // Assignment

y = 20          // Immutable (inferred type)
z: i32 = 30     // Immutable with type

w:: i32         // Mutable forward declaration  
v ::= 40        // Mutable assignment
u:: i32 = 50    // Mutable with type
```

### Structs and Traits

```zen
// From LANGUAGE_SPEC.zen lines 117-163
Point: {
    x:: f64,
    y:: f64 = 0.0,  // Default value
}

Circle: {
    center: Point,
    radius: f64,
}

Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
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
// From LANGUAGE_SPEC.zen lines 431-459
// Range loop
(0..10).loop((i) {
    io.println("${i}")
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

## Test Suite

The comprehensive test validating LANGUAGE_SPEC.zen implementation:

```bash
# Run the main validation test
./target/release/zen tests/zen_test_language_spec_validated_2025.zen

# Test specific features
./target/release/zen tests/zen_test_trait_minimal_spec.zen      # Traits
./target/release/zen tests/zen_test_requires_spec.zen           # .requires()
./target/release/zen tests/zen_test_spec_minimal_working.zen    # Core features
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen          # The source of truth
├── src/                       # Rust compiler implementation
│   ├── ast/                   # Abstract syntax tree
│   ├── parser/                # Parser implementation
│   ├── codegen/               # LLVM code generation
│   └── typechecker/           # Type system
├── stdlib/                    # Standard library (Zen)
├── tests/                     # Test suite (all zen_test_*.zen files)
└── examples/                  # Example programs
```

## Contributing

All contributions must align with `LANGUAGE_SPEC.zen`. The specification is the authoritative source for language design.

## License

MIT