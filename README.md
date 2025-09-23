# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and allocator-based async without function coloring.

**Implementation Status: ~50% Complete** - See [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md) for details.

## Core Philosophy (from LANGUAGE_SPEC.zen)

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

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen examples/01_hello_world.zen

# Start the REPL
./target/release/zen
```

## Language Features

### ✅ Fully Implemented

#### 1. **Zero Keywords** - Pattern Matching Controls Everything
```zen
// No if/else keywords - use pattern matching
is_ready ?
    | true { start_game() }
    | false { show_loading() }

// No while/for keywords - use ranges and loop
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Infinite loop with pattern-based break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

#### 2. **Variable Declarations** - Six Forms from LANGUAGE_SPEC
```zen
// Forward declarations
x: i32                  // Immutable forward declaration
x = 10                  // Assignment later
w:: i32                 // Mutable forward declaration  
w = 20                  // Can reassign: w = 30

// Direct assignments
y = 42                  // Immutable with inference
z: i32 = 100           // Immutable with type
v ::= 50               // Mutable with inference
u:: i32 = 60           // Mutable with type
```

#### 3. **UFC (Uniform Function Call)** - Any Function as Method
```zen
// Define a function
double = (n: i32) i32 { return n * 2 }

// Call as method via UFC
result = 5.double()           // Returns 10

// Chain method calls
value = 5.double().triple().add(1)

// Works with any function
numbers = [1, 2, 3]
doubled = numbers.map(double)  // UFC on collection methods
```

#### 4. **Option Type** - No Null/Nil
```zen
Option<T>: Some(T) | None

// Creating options
maybe_value: Option<i32> = Some(42)
empty: Option<i32> = None

// Pattern matching on options
maybe_value ?
    | Some(val) { io.println("Got: ${val}") }
    | None { io.println("Empty!") }

// Safe unwrapping
value = maybe_value ? 
    | Some(v) { v }
    | None { 0 }  // Default value
```

#### 5. **Result Type** - Error Handling
```zen
Result<T, E>: Ok(T) | Err(E)

parse_int = (s: string) Result<i32, string> {
    // Parsing logic...
    valid ? 
        | true { return Ok(parsed_value) }
        | false { return Err("Invalid number") }
}

// Handle results
result = parse_int("123")
result ?
    | Ok(n) { io.println("Parsed: ${n}") }
    | Err(e) { io.println("Error: ${e}") }
```

#### 6. **Structs and Enums**
```zen
// Struct definition
Point: {
    x:: f64,    // Mutable field
    y:: f64 = 0 // With default
}

// Enum (sum type) 
Shape: Circle | Rectangle | Triangle

// Create instances
p = Point { x: 10.0, y: 20.0 }
shape = Shape.Circle

// Pattern match on enums
shape ?
    | Circle { draw_circle() }
    | Rectangle { draw_rect() }
    | Triangle { draw_tri() }
```

#### 7. **String Interpolation**
```zen
name = "Zen"
version = 1.0
io.println("Welcome to ${name} v${version}")

// Works in any string
message = "Result: ${compute_value()}"
```

#### 8. **Imports from @std**
```zen
// Import specific items
{ io, math } = @std

// Use imported items
io.println("Pi is ${math.pi}")

// Import collections
{ Vec, DynVec } = @std
```

### ✅ Recently Implemented

#### **Traits** with `.implements()` and `.requires()`
```zen
// Define a trait (behavior contract)
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implement trait for type
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})

// Enforce enum variants implement trait
Shape: Circle | Rectangle
Shape.requires(Geometric)  // All variants must implement
```

#### **Error Propagation** with `.raise()`
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()      // Returns early if Err
    contents = file.read_all().raise()  // Propagates errors
    config = parse_json(contents).raise()
    return Ok(config)
}
```

### 🚧 Partially Implemented

- **Pointer Types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` - Basic parsing done
- **Error Propagation**: `.raise()` - Parsed but needs full codegen

### ❌ Not Yet Implemented

- **Generic Functions**: `func<T: Trait>(param: T)`
- **DynVec and Vec**: Dynamic and static vectors
- **Allocators**: Sync/async behavior control
- **Actor System**: Concurrency primitives
- **Channels, Mutex, Atomics**: Threading support
- **AST Reflection**: `reflect.ast()` for metaprogramming
- **@meta.comptime**: Compile-time code generation
- **Module System**: `module.exports` and `module.import`
- **Inline C/LLVM**: FFI support
- **SIMD Operations**: Vector math

## Example Programs

### Hello World
```zen
{ io } = @std

main = () void {
    io.println("Hello, Zen!")
}
```

### Working with Options
```zen
{ io } = @std

find_user = (id: i32) Option<string> {
    id == 42 ?
        | true { return Some("Alice") }
        | false { return None }
}

main = () void {
    user = find_user(42)
    user ?
        | Some(name) { io.println("Found: ${name}") }
        | None { io.println("User not found") }
}
```

### Pattern Matching and UFC
```zen
{ io, math } = @std

// Define operations
double = (n: i32) i32 { return n * 2 }
is_even = (n: i32) bool { return n % 2 == 0 }

main = () void {
    // UFC chaining
    result = 21.double()
    
    // Pattern matching
    result.is_even() ?
        | true { io.println("${result} is even") }
        | false { io.println("${result} is odd") }
    
    // Range loop
    (1..5).loop((i) {
        i.is_even() ? {
            io.println("${i} is even")
        }
    })
}
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/zen
cd zen

# Build with Rust/Cargo
cargo build --release

# Run tests
cargo test

# Run Zen test suite
./target/release/zen tests/zen_test_spec_minimal.zen
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen          # The source of truth
├── IMPLEMENTATION_STATUS.md    # Detailed implementation status
├── src/
│   ├── main.rs                # Entry point and REPL
│   ├── compiler.rs            # Compilation orchestration
│   ├── lexer.rs              # Tokenization
│   ├── parser/                # AST construction
│   ├── ast/                   # AST definitions
│   ├── codegen/llvm/         # LLVM code generation
│   ├── typechecker/          # Type system
│   ├── stdlib/               # Built-in modules
│   └── behaviors/            # Trait system
├── tests/                     # Test suite (zen_ prefix)
└── examples/                  # Example programs
```

## Contributing

Zen is actively being developed. The [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative specification. Current priorities:

1. Complete trait system (`.implements()` and `.requires()`)
2. Implement generic functions and type constraints
3. Add collection types (DynVec, Vec)
4. Implement allocator system for sync/async
5. Add Actor system and concurrency primitives

## Philosophy

Zen eliminates keywords in favor of:
- **Pattern matching** (`?`) for all control flow
- **UFC** for method syntax on any function
- **Allocators** for sync/async without coloring
- **Explicit types** for pointers and optionals
- **Structural simplicity** with only structs and enums

No magic. No implicit behavior. No keywords. Just patterns, functions, and types.

## License

MIT License - See LICENSE file for details

---

**Remember: [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the source of truth!**