# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and compile-time metaprogramming.

> "No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`"  
> "Only two @ symbols: `@std` (standard library) and `@this` (current scope)"  
> "Pattern matching with `?` operator, no `match` or `switch`"  
> "UFC (Uniform Function Call) - any function can be called as method"  
> "Allocators determine sync/async behavior (no function coloring)"  
> — LANGUAGE_SPEC.zen, lines 2-6

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run hello world
./target/release/zen tests/zen_test_simple.zen

# Run showcase of working features
./target/release/zen tests/zen_test_working_showcase.zen

# Compile to executable
./target/release/zen hello.zen
./test_output/hello
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
12. **Traits** - `.implements()` and `.requires()` from `@std.meta`
13. **Compile-time metaprogramming** - Full AST access

## Language Tour

### Variables and Assignment (LANGUAGE_SPEC.zen lines 299-306)

```zen
x: i32          // Forward declaration
x = 10          // Immutable assignment
y = 10          // Immutable (type inferred)
z: i32 = 20     // Immutable with type
w:: i32         // Mutable forward declaration
w = 20          // Assignment to mutable
v ::= 30        // Mutable (type inferred)
u:: i32 = 40    // Mutable with type
```

### Pattern Matching - No if/else/switch (lines 352-361)

```zen
// Boolean pattern matching
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

// Full pattern match for if-else
has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }

// Pattern matching on enums
maybe_value ?
    | Some(v) { io.println("Got value: ${v}") }
    | None { io.println("No value") }
```

### Structs and Enums (lines 115-165)

```zen
// Structs - record types
Point: {
    x:: f64,    // mutable field
    y:: f64 = 0 // with default
}

// Enums - sum types
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)
Shape: Circle | Rectangle
```

### Traits (lines 123-163)

```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64
}

// Implementation using .implements()
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    }
})

// Enforce all variants implement trait
Shape.requires(Geometric)
```

### UFC - Uniform Function Call (lines 173-182)

```zen
// Define functions
get_health = (e: GameEntity.Player) u32 { return 100 }
get_speed = (e: GameEntity.Player) f64 { return 5.0 }

// Call as methods via UFC
player = GameEntity.Player
health = player.get_health()  // Same as get_health(player)
speed = player.get_speed()    // Same as get_speed(player)
```

### Loops and Ranges (lines 431-459)

```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges (when implemented)
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection loops
shapes.loop((shape) {
    total_area = total_area + shape.area()
})

// Infinite loops
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
    io.println("Count: ${counter}")
})
```

### Error Handling (lines 199-211)

```zen
// Result type for errors
parse_number = (s: string) Result<i32, string> {
    // parsing logic...
    return Ok(42)
}

// Error propagation with .raise()
load_data = () Result<Data, Error> {
    file = File.open(path).raise()     // Returns early if Err
    contents = file.read_all().raise()  // Returns early if Err
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Memory Management (lines 363-372)

```zen
// Explicit pointer types - no * or &
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to deref
circle_mut.val.radius = 75
io.println("Address: ${circle_ptr.addr}")
```

### RAII with @this.defer (lines 217, 313-314, 379, 416, 484)

```zen
main = () void {
    file = File.open("data.txt")
    @this.defer(file.close())  // Runs at scope exit
    
    // Use file...
    contents = file.read_all()
    
}  // file.close() called here
```

### Generics (lines 184-196)

```zen
// Generic function with constraints
print_area<T: Geometric>(shape: T) void {
    io.println("Area: ${shape.area()}")
}

// Generic container with multiple constraints
Container<T: Geometric + Serializable>: {
    items: DynVec<T>,
    add: (item: T) void,
    total_area: () f64
}
```

## Current Implementation Status

### ✅ Working Features

- **Variables**: All declaration forms (`=`, `::=`, `:`)
- **Pattern Matching**: Boolean and enum patterns with `?`
- **Structs**: Definition, instantiation, field access, mutation
- **Enums**: Sum types with `Option<T>` and `Result<T,E>`
- **Functions**: Definition, calling, return values
- **UFC**: Uniform function call syntax
- **Traits**: `.implements()` for adding methods to types
- **Loops**: Range loops `(0..10).loop()`, infinite `loop()`
- **RAII**: `@this.defer()` for cleanup
- **String Interpolation**: `"Value: ${expr}"`
- **Imports**: Module imports with `@std`

### 🚧 In Progress

- **Generics**: Basic support, constraints need work
- **Error Propagation**: `.raise()` for Result types
- **Collections**: `Vec<T>` and `DynVec<T>`
- **Pointers**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`

### ❌ Not Yet Implemented

- **Step Ranges**: `(0..100).step(10)`
- **Loop with Index**: `items.loop((item, i) { })`
- **Trait Requirements**: `.requires()` for enum variants
- **Async/Allocators**: Multisync via allocator choice
- **Actors**: Concurrency primitives
- **Metaprogramming**: Compile-time AST manipulation
- **FFI**: External function bindings
- **SIMD**: Vector operations

## Examples

### Hello World
```zen
io = @std.io

main = () void {
    io.println("Hello from Zen!")
}
```

### FizzBuzz (No if/else!)
```zen
io = @std.io

main = () void {
    (1..101).loop((i) {
        fizz = i % 3 == 0
        buzz = i % 5 == 0
        
        (fizz && buzz) ? { io.println("FizzBuzz") }
        fizz ? { io.println("Fizz") }
        buzz ? { io.println("Buzz") }
        (!fizz && !buzz) ? { io.println("${i}") }
    })
}
```

### Error Handling
```zen
io = @std.io

Result<T, E>: Ok(T) | Err(E)

divide = (a: f64, b: f64) Result<f64, string> {
    b == 0.0 ?
        | true { return Result.Err("Division by zero") }
        | false { return Result.Ok(a / b) }
}

main = () void {
    divide(10.0, 2.0) ?
        | Ok(v) { io.println("Result: ${v}") }
        | Err(e) { io.println("Error: ${e}") }
}
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/zen-lang
cd zen-lang

# Build the compiler
cargo build --release

# Run tests
cargo test

# Run a Zen program
./target/release/zen examples/hello.zen
```

## Project Structure

```
.
├── LANGUAGE_SPEC.zen    # The authoritative language specification
├── src/
│   ├── ast/            # Abstract syntax tree definitions
│   ├── parser/         # Parser implementation
│   ├── typechecker/    # Type checking and inference
│   ├── codegen/        # LLVM code generation
│   │   └── llvm/       # LLVM backend
│   └── main.rs         # Compiler entry point
├── tests/              # Test suite (all .zen files)
│   └── zen_test_*.zen  # Individual test files
└── examples/           # Example programs
```

## Contributing

This is an experimental language. Contributions should align with the principles in `LANGUAGE_SPEC.zen`.

## License

MIT

## Philosophy

Zen represents a radical simplification of programming languages. By eliminating keywords and using only pattern matching and uniform function calls, we achieve:

- **Simplicity**: One way to do conditionals (`?`), one way to call functions (UFC)
- **Consistency**: No special syntax for control flow
- **Power**: Full metaprogramming without macros
- **Safety**: No null, explicit error handling, RAII

The language proves that we can build expressive, powerful systems without the complexity of traditional keyword-based languages.

---

*"The best code is no code. The best syntax is no syntax. The best keyword is no keyword."*