# Zen Programming Language

A revolutionary systems programming language that eliminates traditional keywords in favor of pattern-first design and allocator-based async/sync behavior.

**LANGUAGE_SPEC.zen is the authoritative source** - All language features and syntax are defined in `LANGUAGE_SPEC.zen`

**Current Status**: Rust-based Zen compiler with LLVM backend
- ✅ **WORKING**: Core language features from LANGUAGE_SPEC.zen
  - Variable declarations (immutable `=`, mutable `::=`, typed `: Type`)
  - Structs with mutable (`::`) and immutable (`:`) fields
  - Enums and Option types (`Option<T>: Some(T) | None`)
  - Pattern matching with `?` operator
  - Loops and ranges (`(0..10).loop()`, infinite `loop()`)
  - Module imports (`@std`, destructuring `{ io } = @std`)
  - Functions with UFC support
- 🚧 **IN PROGRESS**: Advanced features
  - Traits via `.implements()` and `.requires()`
  - Generics and type parameters
  - Compile-time metaprogramming
  - Allocator-based async/sync
  - String interpolation
  - `.raise()` error propagation

## 🎯 Core Philosophy (from LANGUAGE_SPEC.zen)

Zen's revolutionary design principles:
- **No keywords**: `if/else/while/for/match/async/await/impl/trait/class/interface/null` - none exist
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)
- **Pattern matching with `?` operator**: All control flow through pattern matching
- **UFC (Uniform Function Call)**: Any function callable as method
- **Allocators determine sync/async**: No function coloring - behavior from allocator
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` - no `*` or `&`
- **No null/nil**: Only `Option<T>` with `.Some(T)` and `.None`
- **No unions, no tuples**: Only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type definition)
- **Error propagation**: `.raise()` not exceptions
- **Loops**: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
- **Traits**: Via `.implements()` and `.requires()` from `@std.meta`
- **Compile-time metaprogramming**: Full AST access

## 🚀 Quick Start

### Building the Compiler

```bash
# Build the Rust-based Zen compiler
cargo build --release

# Compile a Zen program
cargo run --bin zen -- myprogram.zen -o output

# Run the generated executable
./output
```

### Hello World (from LANGUAGE_SPEC.zen)

```zen
// Imports - only @std and @this are special
{ io } = @std

main = () void {
    io.println("Hello, Zen World!")
}
```

## 📝 Language Examples (from LANGUAGE_SPEC.zen)

### Variable Declarations

```zen
main = () void {
    x: i32          // Forward declaration
    x = 10          // Immutable assignment
    y = 10          // Immutable (inferred type)
    z: i32 = 20     // Immutable with type
    
    w :: i32        // Mutable forward declaration
    w = 20          // Initial assignment
    v ::= 30        // Mutable (inferred type)
    u :: i32 = 40   // Mutable with type
}
```

### Structs and Enums

```zen
// Simple struct
Point: {
    x :: f64,       // Mutable field
    y :: f64 = 0    // With default value
}

// Enum type (sum type)
Option<T>: Some(T) | None

// Using enums
Shape: Circle | Rectangle

main = () void {
    // Create struct instances
    circle = Circle { 
        center: Point { x: 0, y: 0 }, 
        radius: 5 
    }
    
    // Option handling - no null!
    maybe_radius: Option<f64> = Option.Some(5.5)
    maybe_radius ?
        | Some(r) {
            // Use radius value r
        }
        | None {
            // No radius provided
        }
}
```

### Pattern Matching

```zen
// Boolean pattern matching
is_ready = true
is_ready ? { 
    io.println("Starting!") 
}

// Full pattern match
has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting...") }

// Enum pattern matching
shape ?
    | Circle { io.println("It's a circle") }
    | Rectangle { io.println("It's a rectangle") }
```

### Loops and Ranges

```zen
// Range iterations
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection loops with UFC
shapes.loop((shape) {
    io.println("Area: ${shape.area()}")
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

### Error Handling with Result

```zen
Result<T, E>: Ok(T) | Err(E)

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()     // Returns early if Err
    contents = file.read_all().raise()  // Propagates errors
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Traits and UFC

```zen
// Define trait
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

// UFC - any function can be called as method
shapes.loop((shape) {
    io.println("Area: ${shape.area()}")  // UFC method call
})
```

### Pointers (No * or &)

```zen
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75                     // Modify through pointer
io.println("Address: ${circle_ptr.addr}")      // Get address
```

## 🏗️ Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen     # ⭐ THE authoritative language specification
├── src/                  # Rust compiler implementation
│   ├── main.rs          # Compiler entry point
│   ├── lexer.rs         # Lexical analysis
│   ├── parser/          # Parsing modules
│   ├── ast/             # Abstract syntax tree
│   ├── typechecker/     # Type checking
│   ├── codegen/         # LLVM code generation
│   └── stdlib/          # Standard library
├── tests/               # Test suite
│   └── zen_test_*.zen   # Language feature tests
├── examples/            # Example programs
└── stdlib/              # Standard library (Zen)
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test file
cargo run --bin zen -- tests/zen_test_spec_vars.zen -o test_output
./test_output

# Test files demonstrate all LANGUAGE_SPEC.zen features
```

## 📚 Documentation

- `LANGUAGE_SPEC.zen` - Complete language specification with examples
- `examples/` - Working examples demonstrating language features
- `tests/zen_test_spec_*.zen` - Tests validating spec compliance

## 🤝 Contributing

Contributions must align with `LANGUAGE_SPEC.zen`. The specification is the single source of truth for language features and behavior.

## 📄 License

MIT License - See LICENSE file for details