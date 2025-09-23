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

## Current Implementation Status (September 2025)

The Zen compiler is approximately **65% complete**. See [`IMPLEMENTATION_STATUS_2025.md`](./IMPLEMENTATION_STATUS_2025.md) for detailed feature status.

### ✅ Working Features

- Core language (no keywords, pattern matching with `?`)
- All 6 variable forms from spec (lines 298-306)
- Structs with fields, default values, and nested structs
- Enums (Option, Result, custom enums)
- Pattern matching (boolean, Option, Result, enums)
- Traits with `.implements()` 
- Function overloading based on enum variants
- Range loops `(0..10).loop()`
- String interpolation with proper boolean/Option formatting
- Basic @std modules (io, math)
- @this.defer() for cleanup

### 🚧 In Progress

- Nested struct field access (partially working)
- Mutable field updates
- Generic types `<T: Trait>`
- .requires() for enum constraints

### ❌ Not Yet Implemented

- Pointer types (Ptr, MutPtr, RawPtr)
- Collections (Vec, DynVec)
- .raise() error propagation
- .step() for ranges
- Allocators (GPA, AsyncPool)
- Actors and channels
- Metaprogramming (@meta.comptime)
- FFI (inline.c, SIMD)
- Module system (exports/imports)

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen examples/01_hello_world.zen

# Run the main test suite
./target/release/zen tests/zen_test_spec_core_validation.zen

# Run all tests
for test in tests/zen_test_*.zen; do 
    echo "Running $test"
    ./target/release/zen "$test" 2>&1 | grep -v DEBUG
done
```

## Language Examples (from LANGUAGE_SPEC.zen)

### Variables and Assignment (Lines 298-306)

```zen
// Six forms of variable declaration from spec
x: i32           // Forward declaration (immutable)
x = 10           // Must assign in same scope
y = 10           // Immutable assignment
z: i32 = 20      // Immutable with type
w:: i32          // Mutable forward declaration  
w = 20           // Can reassign
v ::= 30         // Mutable assignment
u:: i32 = 40     // Mutable with type
```

### Structs and Enums (Lines 117-170)

```zen
// Simple struct with mutable fields
Point: {
    x:: f64,     // Mutable field (note ::)
    y:: f64 = 0  // With default value
}

// Enums - the only sum types
Option<T>: Some(T) | None        // No null!
Result<T, E>: Ok(T) | Err(E)     // Error handling
Shape: Circle | Rectangle        // Simple enum
GameEntity: Player | Enemy | Powerup  // For overloading
```

### Pattern Matching - No if/else! (Lines 352-361, 462-473)

```zen
// Boolean pattern - single branch
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

// Full pattern match - replaces if-else
has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }

// Option pattern matching - no null!
maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle { radius: r }
        io.println("Area: ${circle.area()}")
    }
    | None {
        io.println("No radius provided")
    }
```

### Traits (Lines 124-162)

```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation using .implements()
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})

// Enforce trait on enum
Shape: Circle | Rectangle
Shape.requires(Geometric)  // All variants must implement
```

### Loops - No for/while! (Lines 432-459)

```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges (not yet implemented)
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20...
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { io.println("Count: ${counter}") }
})

// Collection loops (when DynVec implemented)
shapes.loop((shape) {
    io.println("Area: ${shape.area()}")
})
```

### Memory Management (Lines 309-314, 217, 379)

```zen
// Allocators determine sync/async behavior
sync_alloc = GPA.init()
@this.defer(sync_alloc.deinit())  // Cleanup on scope exit

async_alloc = AsyncPool.init()
@this.defer(async_alloc.deinit())

// Same function works sync or async based on allocator!
data = fetch_game_data(url, sync_alloc)   // Blocks
data = fetch_game_data(url, async_alloc)  // Non-blocking
```

### Future: Metaprogramming (Lines 244-281)

```zen
// AST reflection at runtime
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    ast.kind ?
        | Struct(s) {
            io.println("Struct: ${s.name}")
            s.fields.loop((f) {
                io.println("  Field: ${f.name}: ${f.type}")
            })
        }
        | Enum(e) {
            io.println("Enum with ${e.variants.len()} variants")
        }
}

// Compile-time AST modification
@meta.comptime((){
    original = reflect.ast(parse_radius)
    new_body = original.body.prepend(
        AST.Call("io.println", ["Parsing radius"])
    )
    meta.replace(parse_radius, original.with_body(new_body))
})
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen         # THE SOURCE OF TRUTH - Complete language specification
├── IMPLEMENTATION_STATUS_2025.md  # Current implementation progress
├── src/                      # Rust compiler implementation
│   ├── main.rs              # Entry point
│   ├── lexer.rs             # Tokenizer
│   ├── parser/              # AST generation
│   ├── typechecker/         # Type system
│   ├── codegen/llvm/        # LLVM code generation
│   └── stdlib/              # Built-in functions
├── tests/                    # Test suite (zen_test_*.zen)
│   └── zen_test_spec_core_validation.zen  # Main spec validation
├── examples/                 # Example programs
└── stdlib/                   # Standard library (Zen code)
```

## Testing

The primary test is `tests/zen_test_spec_core_validation.zen` which validates features from LANGUAGE_SPEC.zen:

```bash
./target/release/zen tests/zen_test_spec_core_validation.zen
```

This tests:
- All 6 variable forms (lines 298-306)
- Pattern matching (lines 352-361)
- Structs and traits (lines 117-162) 
- Option types (lines 462-473)
- Ranges and loops (lines 432-439)
- Enum variants (lines 165-170)

## Contributing

The language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source. Any implementation must match this spec exactly.

Key areas needing implementation:
1. Pointer types (Ptr, MutPtr, RawPtr) - lines 364-372
2. Collections (Vec, DynVec) - lines 374-384, 317-350
3. Error propagation (.raise()) - lines 207-210
4. Allocators and async - lines 309-314, 213-224
5. Metaprogramming - lines 244-281
6. Module system - lines 491-510

## VSCode Extension

Install the Zen language extension for syntax highlighting:

```bash
cd vscode-zenlang
npm install
npm run compile
```

Then load the extension in VSCode.

## License

MIT

## Acknowledgments

Zen is inspired by Zig's comptime, Rust's safety, Go's simplicity, and seeks to eliminate all keywords through uniform design principles.