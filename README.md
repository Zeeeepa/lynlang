# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and allocator-based async without function coloring.

## Key Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

- **No keywords**: `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)  
- **Pattern matching with `?` operator**, no `match` or `switch`
- **UFC (Uniform Function Call)** - any function can be called as method
- **Allocators determine sync/async behavior** (no function coloring)
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **No null/nil** - only `Option<T>` with `.Some(T)` and `.None`
- **No unions, no tuples** - only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type definition)
- **Error propagation with `.raise()`** not exceptions
- **Loops**: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
- **Traits via `.implements()` and `.requires()`** from `@std.meta`
- **Compile-time metaprogramming** with full AST access

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program  
./target/release/zen examples/hello.zen

# Run comprehensive tests
./target/release/zen tests/zen_test_traits_working.zen
./target/release/zen tests/zen_test_simple_from_spec.zen

# Start the REPL
./target/release/zen
```

## Core Features (Working ✅)

### 1. Zero Keywords - Pattern Matching Controls Everything

```zen
// No if/else - just ?
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

// Full pattern match for conditionals  
has_data ?
    | true { process_data() }
    | false { io.println("Waiting...") }

// Pattern matching on enums
result ?
    | Ok(val) { io.println("Success: ${val}") }
    | Err(e) { io.println("Error: ${e}") }

// Pattern matching on ranges
value ?
    | 0 { io.println("Zero") }
    | 1..10 { io.println("Small") }
    | _ { io.println("Large") }
```

### 2. Variable Declarations - Six Forms

```zen
x: i32          // forward declaration (immutable)
x = 10          // assignment

y = 10          // immutable assignment (type inferred)
z: i32 = 20     // immutable with explicit type

w:: i32         // mutable forward declaration
w = 20          // can be reassigned

v ::= 30        // mutable assignment (type inferred)
u:: i32 = 40    // mutable with explicit type
```

### 3. No Null - Only Option<T>

```zen
Option<T>: Some(T) | None

find_user = (id: i32) Option<User> {
    id > 0 ?
        | true { return Option.Some(User { id: id, name: "Alice" }) }
        | false { return Option.None }
}

user = find_user(123)
user ?
    | Some(u) { io.println("Found: ${u.name}") }
    | None { io.println("User not found") }
```

### 4. Result Type for Error Handling

```zen
Result<T, E>: Ok(T) | Err(E)

parse_number = (s: string) Result<i32, string> {
    // Parse logic...
    valid ?
        | true { return Result.Ok(42) }
        | false { return Result.Err("Invalid number") }
}
```

### 5. Error Propagation with .raise()

```zen
// No try/catch - errors propagate with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()      // Returns early if Err
    contents = file.read_all().raise()  // Chain operations
    config = json.parse(contents).raise()
    return Result.Ok(config)
}
```

### 6. Structs and Enums

```zen
// Struct with mutable fields
Point: {
    x:: f64,    // mutable field (::)
    y:: f64     // mutable field
}

// Immutable struct fields
Circle: {
    radius: f64  // immutable field
}

// Enum (sum type)
Shape: Circle | Rectangle
GameEntity: Player | Enemy | Powerup
```

### 7. Traits with .implements() and .requires()

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

// Require all enum variants implement trait
Shape: Circle | Rectangle
Shape.requires(Geometric)
```

### 8. Loops and Ranges

```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration
items.loop((item) {
    process(item)
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

### 9. UFC (Uniform Function Call)

```zen
// Any function can be called as method
double = (n: i32) i32 { return n * 2 }

result = 5.double()  // UFC: same as double(5)

// Works with all functions
(0..10).loop((i) { io.println(i) })  // UFC on range

// Overloading for different types
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }

player = GameEntity.Player
health = player.get_health()  // UFC dispatch
```

### 10. Explicit Pointer Types

```zen
// No * or & - explicit pointer types
value = 42
ptr: Ptr<i32> = value.ref()           // immutable pointer
mut_ptr: MutPtr<i32> = value.mut_ref() // mutable pointer

// Dereference with .val
io.println("Value: ${ptr.val}")

// Get address with .addr
io.println("Address: ${ptr.addr}")

// Modify through mutable pointer
mut_ptr.val = 100
```

### 11. String Interpolation

```zen
name = "Zen"
version = 2025
io.println("Welcome to ${name} v${version}!")

// Expressions in interpolation
x = 10
y = 20
io.println("${x} + ${y} = ${x + y}")
```

## Future Features (From LANGUAGE_SPEC.zen)

These features are specified but not yet implemented:

- **@this.defer()** - Automatic resource cleanup
- **Allocators** - GPA, AsyncPool for controlling sync/async behavior
- **DynVec** - Dynamic vectors supporting mixed variant types
- **Concurrency** - Actor, Channel, Mutex, AtomicU32
- **Reflection** - AST inspection with `reflect.ast()`
- **Metaprogramming** - `@meta.comptime()` for compile-time code generation
- **Module System** - `module.exports` and `module.import`
- **FFI** - Foreign function interface for C/LLVM integration
- **SIMD** - Vector operations with `simd.add()` etc.

## Examples

### Complete Example from LANGUAGE_SPEC.zen

```zen
{ io } = @std

// Struct with trait implementation
Circle: { radius: f64 }

Geometric: { area: (self) f64 }

Circle.implements(Geometric, {
    area = (self) f64 {
        return 3.14159 * self.radius * self.radius
    }
})

main = () void {
    // Variables
    x = 10           // immutable
    y ::= 20         // mutable
    y = 30           // can reassign

    // Pattern matching
    is_ready = true
    is_ready ? {
        io.println("Ready!")
    }

    // Option handling
    maybe: Option<i32> = Option.Some(42)
    maybe ?
        | Some(val) { io.println("Got: ${val}") }
        | None { io.println("No value") }

    // Ranges and UFC
    (0..5).loop((i) {
        io.println("${i}")
    })

    // Traits
    circle = Circle { radius: 5.0 }
    io.println("Area: ${circle.area()}")
}
```

## Philosophy

Zen eliminates programming language complexity by removing ALL keywords and using consistent, orthogonal features:

- **Pattern matching (`?`)** replaces all conditionals
- **UFC** makes any function callable as a method
- **Explicit types** remove ambiguity (no null, explicit pointers)
- **Traits** provide polymorphism without inheritance
- **Allocators** control execution model without function coloring

## Building and Testing

```bash
# Build compiler
cargo build --release

# Run all tests
./run_tests.sh

# Test specific feature
./target/release/zen tests/zen_test_traits_working.zen
./target/release/zen tests/zen_test_simple_from_spec.zen

# Start REPL
./target/release/zen
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen     # The source of truth
├── src/                  # Rust compiler implementation
│   ├── ast/             # Abstract syntax tree
│   ├── lexer.rs         # Tokenization
│   ├── parser/          # Parsing
│   ├── typechecker/     # Type checking
│   ├── codegen/llvm/    # LLVM code generation
│   └── stdlib/          # Built-in functions
├── tests/               # Test suite (zen_test_*.zen)
└── examples/            # Example programs
```

## Contributing

All contributions must align with `LANGUAGE_SPEC.zen`. The specification is the source of truth for language behavior.

## License

MIT