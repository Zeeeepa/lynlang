# Zen Programming Language

**`LANGUAGE_SPEC.zen` IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS** - everything is pattern matching, uniform function calls, and compile-time metaprogramming. The complete language specification is defined in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

## Core Philosophy (LANGUAGE_SPEC.zen lines 1-14)

```zen
// NO KEYWORDS - No if/else/while/for/match/async/await/impl/trait/class/interface/null
// Only two @ symbols: @std (standard library) and @this (current scope)
// Pattern matching with ? operator, no match or switch
// UFC (Uniform Function Call) - any function can be called as method
// Allocators determine sync/async behavior (no function coloring)
// Explicit pointer types: Ptr<>, MutPtr<>, RawPtr<> (no * or &)
// No null/nil - only Option<T> with .Some(T) and .None
// No unions, no tuples - only structs and enums
// Assignment operators: = (immutable), ::= (mutable), : (type definition)
// Error propagation with .raise() not exceptions
// Loops: loop() for infinite, .loop() for collections, ranges like (0..10)
// Traits via .implements() and .requires() from @std.meta
// Compile-time metaprogramming with full AST access
```

## Quick Start

### Building the Compiler
```bash
cargo build --release
```

### Running Zen Programs
```bash
# Run directly
./target/release/zen program.zen

# Compile to executable  
./target/release/zen program.zen -o program
./program

# Start REPL
./target/release/zen
```

### Run Test Suite
```bash
# All tests are in tests/ folder with zen_ prefix
./target/release/zen tests/zen_test_spec_aligned_working.zen
```

## Implementation Status

### ✅ Core Features (Working)

| Feature | LANGUAGE_SPEC.zen Reference | Example |
|---------|----------------------------|---------|
| **No Keywords** | Lines 1-2 | Pattern matching with `?` replaces all control flow |
| **@std imports** | Lines 92-107 | `{ io, math } = @std` |
| **@this scope** | Line 2, 217+ | `@this.defer(cleanup())` |
| **Pattern matching `?`** | Lines 29-71, 352-361 | `expr ? \| true {} \| false {}` |
| **Variable declarations** | Lines 298-306 | `=` immutable, `::=` mutable |
| **Option type** | Lines 109-110 | `Option<T>: Some(T) \| None` |
| **Result type** | Lines 112-113 | `Result<T, E>: Ok(T) \| Err(E)` |
| **UFC (Uniform Function Call)** | Line 4, throughout | `object.method()` → `method(object)` |
| **Loops & Ranges** | Lines 432-460 | `(0..10).loop()`, `loop {}` |
| **String interpolation** | Throughout | `"Value: ${expr}"` |
| **Structs** | Lines 117-120 | `Point: { x:: f64, y:: f64 }` |
| **Enums** | Lines 165-182 | `Shape: Circle \| Rectangle` |
| **Functions** | Throughout | First-class functions with type inference |
| **Closures** | Lines 130, 241+ | `(params) { body }` |

### 🚧 Partially Implemented

| Feature | LANGUAGE_SPEC.zen Reference | Status |
|---------|----------------------------|--------|
| **Pointer types** | Lines 363-372 | Parsed: `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` |
| **.raise() propagation** | Lines 206-211 | AST and basic codegen ready |
| **Traits .implements()** | Lines 135-143 | Parser support, needs codegen |
| **Traits .requires()** | Line 168 | Parser support, needs codegen |
| **DynVec** | Lines 316-350 | Type defined, needs runtime |
| **Step ranges** | Lines 437-439 | `(0..100).step(10)` parsed |

### ❌ Not Yet Implemented

| Feature | LANGUAGE_SPEC.zen Reference | Priority |
|---------|----------------------------|----------|
| **Generic functions** | Lines 185-188 | High |
| **Generic containers** | Lines 190-195 | High |
| **Allocators** | Lines 308-314 | Medium - Sync/async determination |
| **Actors** | Lines 228-240 | Medium - Lazy iteration |
| **Channels** | Lines 397-412 | Medium - Message passing |
| **Mutex** | Lines 415-424 | Medium - Shared state |
| **Atomics** | Lines 426-429 | Medium - Lock-free ops |
| **AST Reflection** | Lines 243-273 | Low - Runtime metaprogramming |
| **@meta.comptime** | Lines 275-281 | Low - Compile-time code gen |
| **Inline C/LLVM** | Lines 285-290 | Low - FFI integration |
| **SIMD operations** | Lines 292-294 | Low - Vector math |
| **Build system** | Lines 19-85 | Low - build.zen support |

## Language Examples from LANGUAGE_SPEC.zen

All examples below are directly from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

### Variable Declarations (Lines 298-306)
```zen
x: i32              // forward declaration
x = 10              
y = 10              // Immutable assignment
z: i32 = 20         // Immutable with type
w:: i32             // mutable forward declaration
w = 20              
v ::= 30            // Mutable assignment
u:: i32 = 40        // mutable with type
```

### Pattern Matching with ? (Lines 352-361)
```zen
// Boolean pattern matching - no ternary
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

// Full pattern match for if-else
has_data ?
    | true { process_data() }
    | false { io.println("Waiting...") }
```

### Option Type - No Null (Lines 462-473)
```zen
Option<T>: Some(T) | None

maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle { radius: r }
        io.println("Area: ${circle.area()}")
    }
    | None {
        io.println("No radius")
    }
```

### Result Type & Error Handling (Lines 199-211)
```zen
Result<T, E>: Ok(T) | Err(E)

parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid") }
}

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // Early return on Err
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### UFC - Uniform Function Call (Lines 318-350)
```zen
// Any function can be called as method
entities = [Player, Enemy, Player]

// Method-style call
entities.loop((entity) {
    entity ?
        | Player { io.println("Health: ${entity.get_health()}") }
        | Enemy { io.println("Health: ${entity.get_health()}") }
})

// Equivalent to: loop(entities, (entity) { ... })
```

### Loops and Ranges (Lines 432-460)
```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20...
})

// Infinite loop
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? 
        | true { break }
        | false { continue }
})
```

### Structs and Enums (Lines 117-182)
```zen
// Struct with mutable fields
Point: {
    x:: f64,  // mutable with ::
    y:: f64 = 0  // default value
}

// Enum type
Shape: Circle | Rectangle

// Pattern matching on enums
shape ?
    | Circle { io.println("It's round") }
    | Rectangle { io.println("It's boxy") }
```

### Traits (Lines 123-168)
```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})

// Requirement
Shape.requires(Geometric)
```

### Pointer Types (Lines 363-372)
```zen
// Explicit pointer types - no * or &
circle = Circle { radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Area: ${circle_ptr.val.area()}")  // .val to deref
circle_mut.val.radius = 75  // Modify through pointer
io.println("Address: ${circle_ptr.addr}")  // Get address
```

## Project Structure
```
zenlang/
├── LANGUAGE_SPEC.zen     # THE source of truth
├── src/
│   ├── main.rs          # Entry point & REPL
│   ├── lexer.rs         # Tokenization
│   ├── parser/          # AST construction
│   ├── ast/             # AST definitions
│   ├── codegen/         # LLVM code generation
│   ├── typechecker/     # Type system
│   └── stdlib/          # Standard library
└── tests/               # All tests with zen_ prefix
```

## Contributing

All contributions must align with `LANGUAGE_SPEC.zen`. The spec is the single source of truth - if something contradicts the spec, the spec wins.

## Philosophy

Zen achieves simplicity through uniformity:
- **No keywords** - Everything is patterns and functions
- **No null** - Option types only
- **No exceptions** - Result types with `.raise()`
- **No function coloring** - Allocators determine sync/async
- **Uniform calls** - Any function works as a method
- **Explicit pointers** - Clear memory semantics

The goal: a language so consistent that once you learn the patterns, you can predict how everything works.