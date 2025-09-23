# Zen Programming Language

A modern systems programming language with a unique philosophy: **no keywords**. Instead of traditional keywords like `if`, `else`, `while`, `for`, `match`, `async`, `await`, `impl`, `trait`, `class`, `interface`, and `null`, Zen uses a minimal set of operators and patterns to achieve the same expressiveness.

> **[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative specification** - All language features, syntax, and design principles are defined there.

## Key Design Principles

- **No keywords**: Only operators and two special symbols (`@std` and `@this`)
- **Pattern matching everywhere**: The `?` operator for all conditionals
- **UFC (Uniform Function Call)**: Any function can be called as a method
- **No null**: Only `Option<T>` with `.Some(T)` and `.None`
- **Allocator-driven concurrency**: Sync/async behavior determined by allocator choice
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type annotation)
- **Compile-time metaprogramming**: Full AST access and manipulation

## Installation

```bash
# Build the compiler
cargo build --release

# The compiler will be available at
./target/release/zen
```

## Quick Start

### Hello World

```zen
{ io } = @std

main = () void {
    io.println("Hello, World!")
}
```

### Variables

```zen
main = () void {
    // Immutable variables (default)
    x = 10
    y: i32 = 20  // With type annotation
    
    // Mutable variables  
    count ::= 0
    value :: i32 = 42
    
    // Forward declarations
    z: i32      // Immutable forward declaration
    z = 30      // First assignment
    
    w :: i32    // Mutable forward declaration
    w = 40
    w = 50      // Can reassign
}
```

### Pattern Matching

Instead of `if/else` statements, Zen uses pattern matching with the `?` operator:

```zen
main = () void {
    is_ready = true
    
    // Simple boolean check
    is_ready ? {
        io.println("Ready!")
    }
    
    // With branches
    x = 10
    is_positive = x > 0
    is_positive ?
        | true { io.println("Positive") }
        | false { io.println("Not positive") } }
}
```

### Loops

```zen
main = () void {
    // Range loop
    (0..10).loop((i) {
        io.println("Count: ${i}")
    })
    
    // Step ranges
    (0..100).step(10).loop((i) {
        io.println("Step: ${i}")  // 0, 10, 20, ...
    })
    
    // Infinite loop
    counter ::= 0
    loop(() {
        counter = counter + 1
        counter > 10 ?
            | true { break }
            | false { io.println("Count: ${counter}") }
    })
}
```

### Structs and Enums

```zen
// Struct definition
Point: {
    x: f64,
    y: f64
}

// Enum definition (sum type)
Shape: Circle | Rectangle | Triangle

// Enum with data
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)

main = () void {
    p = Point { x: 10.0, y: 20.0 }
    io.println("Point: (${p.x}, ${p.y})")
    
    shape = Shape.Circle
    shape ?
        | Circle { io.println("It's a circle") }
        | Rectangle { io.println("It's a rectangle") }
        | Triangle { io.println("It's a triangle") }
}
```

### UFC (Uniform Function Call)

Any function can be called as a method on its first parameter:

```zen
// Define functions
length = (s: string) i32 { 
    // Implementation
    return 5
}

double = (x: i32) i32 {
    return x * 2
}

main = () void {
    text = "hello"
    
    // Traditional call
    len1 = length(text)
    
    // UFC - same function as method
    len2 = text.length()
    
    // Chain calls
    result = 10.double()
}
```

### String Interpolation

```zen
main = () void {
    name = "Zen"
    version = 1
    
    message = "Welcome to ${name} v${version}!"
    io.println(message)
    
    // In expressions
    io.println("Result: ${10 + 20}")
}
```

## Current Implementation Status

### ✅ Working Features

- **Core Language**
  - Variable declarations (all forms: `=`, `::=`, `:`, `::`)
  - Forward declarations (immutable and mutable)
  - Basic types (`i32`, `i64`, `f32`, `f64`, `bool`, `string`)
  - String interpolation
  - Pattern matching with `?` operator
  - Boolean patterns
  
- **Data Structures**
  - Struct definitions and field access
  - Basic enum definitions
  - Arrays and indexing
  
- **Control Flow**
  - Pattern matching
  - Loops (range, step, infinite)
  - Break/continue statements
  
- **Functions**
  - Function definitions and calls
  - UFC (Uniform Function Call)
  - Function overloading
  
- **Module System**
  - Basic `@std` imports
  - Destructuring imports

### 🚧 Partially Implemented

- **Enums**: Basic enum types work, but variant data payloads have issues
- **Option/Result Types**: Pattern matching works but payload values are corrupted
- **Error Propagation**: `.raise()` syntax parsed but has codegen issues

### ❌ Not Yet Implemented

- **Traits**: `.implements()` and `.requires()` methods
- **Allocators**: GPA, AsyncPool, and allocator-driven concurrency
- **Pointer Types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`
- **Compile-time Metaprogramming**: `@meta.comptime()`, AST reflection
- **Module Exports**: `module.exports` syntax
- **Actors and Channels**: Concurrency primitives
- **FFI**: Foreign function interface
- **SIMD Operations**: Vector operations

## Known Issues

1. **Enum Payloads**: Values passed with enum variants (e.g., `Some(42)`) are corrupted
2. **Error Propagation**: `.raise()` causes LLVM verification errors
3. **Complex Pattern Matching**: Some nested patterns may not work correctly

## Language Specification

The complete language specification is in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen), which serves as both documentation and a comprehensive example of Zen syntax.

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen       # Complete language specification (source of truth)
├── src/                    # Rust compiler implementation
│   ├── lexer.rs           # Tokenization
│   ├── parser/            # AST generation
│   ├── typechecker/       # Type checking
│   └── codegen/llvm/      # LLVM code generation
├── stdlib/                # Standard library modules
│   ├── core/              # Core types (Option, Result)
│   ├── io.zen             # I/O operations
│   └── ...               
├── tests/                 # Test suite (zen_test_*.zen)
│   └── zen_test_language_spec_validation.zen  # Main spec test
└── examples/              # Example programs
```

## Running Tests

```bash
# Run a specific test
./target/release/zen tests/zen_test_hello_world.zen

# Run all tests
./run_tests.sh
```

## Contributing

Zen is under active development. Priority areas for contribution:

1. Fixing enum payload corruption
2. Implementing traits system
3. Implementing module exports/imports
4. Adding allocator support
5. Implementing pointer types

## Philosophy

Zen embraces simplicity and consistency. By eliminating keywords and using a minimal set of operators, the language becomes more regular and predictable. Every construct follows the same patterns:

- **No special cases**: Everything is an expression
- **No function coloring**: Sync/async determined by context, not syntax
- **No null**: Explicit option types prevent null pointer errors
- **Pattern first**: Pattern matching is the primary control flow mechanism

## License

[License information to be added]

## Contact

[Contact information to be added]