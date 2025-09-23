# Zen Programming Language

A modern systems programming language with a unique design philosophy based on minimalism and expressiveness.

## 🎯 Key Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

- **No keywords**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)
- **Pattern matching with `?` operator**: No `match` or `switch` keywords
- **UFC (Uniform Function Call)**: Any function can be called as a method
- **No function coloring**: Allocators determine sync/async behavior
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **No null/nil**: Only `Option<T>` with `.Some(T)` and `.None`
- **No unions, no tuples**: Only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type definition)
- **Error propagation**: `.raise()` for early returns, not exceptions
- **Loops**: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`

## 🚀 Quick Start

```zen
// hello.zen
{ io } = @std

main = () void {
    name = "World"
    io.println("Hello, ${name}!")
}
```

### Building the Compiler
```bash
cargo build --release
```

### Running Zen Programs
```bash
# Run directly
./target/release/zen hello.zen

# Compile to executable
./target/release/zen hello.zen -o hello
./hello

# Start REPL
./target/release/zen
```

## ✨ Language Features

### Variable Declarations (LANGUAGE_SPEC.zen lines 298-306)
```zen
x: i32              // Forward declaration
x = 10              // Immutable assignment
y = 20              // Immutable with type inference
z: i32 = 30         // Immutable with explicit type
w:: i32             // Mutable forward declaration
w = 40              // Assignment to mutable
v ::= 50            // Mutable with inference
u:: i32 = 60        // Mutable with explicit type
```

### Pattern Matching (LANGUAGE_SPEC.zen lines 352-361)
```zen
// Boolean short form
is_ready ? {
    io.println("Ready!")
}

// Full pattern matching
result ?
    | Ok(value) { process(value) }
    | Err(error) { handle_error(error) }
```

### No Null - Only Options (LANGUAGE_SPEC.zen lines 109-110, 462-473)
```zen
Option<T>: Some(T) | None

maybe: Option<i32> = Some(42)
maybe ?
    | Some(val) { io.println("Value: ${val}") }
    | None { io.println("No value") }
```

### Structs and Enums (LANGUAGE_SPEC.zen lines 117-170)
```zen
Point: {
    x:: f64,    // Mutable field
    y:: f64 = 0 // With default value
}

Shape: Circle | Rectangle

shape = Shape.Circle
shape ?
    | Circle { draw_circle() }
    | Rectangle { draw_rectangle() }
```

### UFC (Uniform Function Call)
```zen
// Define a function
double = (x: i32) i32 { return x * 2 }

// Call as function OR method
result1 = double(5)       // Traditional call
result2 = 5.double()      // UFC: becomes double(5)

// Works with any function!
shapes.map(calculate_area)      // Traditional
calculate_area.map(shapes)      // UFC style
```

### Loops and Ranges (LANGUAGE_SPEC.zen lines 432-460)
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

// With index
items.loop((item, i) {
    io.println("Item ${i}: ${item}")
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 100 ? { break }
})
```

### Error Propagation with `.raise()` (LANGUAGE_SPEC.zen lines 206-211)
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()        // Returns early if Err
    contents = file.read_all().raise()    // Returns early if Err
    config = json.parse(contents).raise() // Returns early if Err
    return Ok(config)
}
```

### String Interpolation (LANGUAGE_SPEC.zen line 173)
```zen
name = "Zen"
version = 1
message = "Language: ${name}, Version: ${version}"
io.println(message)  // "Language: Zen, Version: 1"
```

### Defer Statements (LANGUAGE_SPEC.zen lines 217, 309, 314)
```zen
file = File.open("data.txt")
@this.defer(file.close())  // Runs at scope end
// ... use file ...
// file.close() automatically called here
```

### Pointer Types (LANGUAGE_SPEC.zen lines 364-372)
```zen
circle = Circle { radius: 10.0 }
ptr: Ptr<Circle> = circle.ref()        // Immutable reference
mut_ptr: MutPtr<Circle> = circle.mut_ref()  // Mutable reference

io.println("Value: ${ptr.val.radius}")  // Dereference with .val
io.println("Address: ${ptr.addr}")      // Get address with .addr

mut_ptr.val.radius = 75  // Mutate through pointer
```

### Dynamic Vectors with Allocators (LANGUAGE_SPEC.zen lines 317-335)
```zen
// Create allocator
alloc = GPA.init()
@this.defer(alloc.deinit())

// Dynamic vector with single type
vec = DynVec<i32>(alloc)
vec.push(42)

// Mixed type vector (enum variants)
entities = DynVec<Player, Enemy>(alloc)
entities.push(Player)
entities.push(Enemy)

// Iterate with pattern matching
entities.loop((entity) {
    entity ?
        | Player { handle_player() }
        | Enemy { handle_enemy() }
})
```

## 📚 Standard Library

The standard library is accessed through `@std`:

```zen
{ io, math, fs } = @std              // Import modules
{ Vec, DynVec, String } = @std       // Import types
{ Option, Result } = @std            // Import generic types
{ GPA, AsyncPool } = @std            // Import allocators
```

## 🏗️ Project Structure

```
zenlang/
├── src/               # Rust compiler source
│   ├── ast/           # Abstract syntax tree
│   ├── codegen/       # LLVM code generation
│   ├── lexer.rs       # Tokenization
│   ├── parser/        # Parsing
│   └── typechecker/   # Type checking
├── stdlib/            # Standard library (Zen)
├── tests/             # Test suite (Zen)
├── examples/          # Example programs
└── LANGUAGE_SPEC.zen  # Language specification (source of truth)
```

## 🧪 Testing

Run the comprehensive test suite:
```bash
# Run all tests
./target/release/zen tests/zen_test_comprehensive_spec.zen

# Run specific test categories
./target/release/zen tests/zen_test_core_features.zen
./target/release/zen tests/zen_test_language_spec_alignment.zen
```

## 📈 Implementation Status

### ✅ Fully Implemented
- Variable declarations (all forms from spec)
- Pattern matching with `?` operator
- Option and Result types (no null!)
- Structs with mutable fields
- Enums and pattern matching
- Loops and ranges
- UFC (Uniform Function Call)
- String interpolation `${expr}`
- Error propagation with `.raise()`
- Defer statements with `@this.defer()`
- Pointer types (Ptr, MutPtr, RawPtr)
- DynVec with allocators
- @std imports
- Forward declarations

### 🚧 In Progress
- Traits with `.implements()` and `.requires()`
- Compile-time metaprogramming with `@std.meta`
- Full allocator-based async/sync
- Actor model for concurrency
- Channels and mutexes

### 📋 Planned
- SIMD operations
- Inline C/LLVM code
- Module system enhancements
- Package manager
- Self-hosting compiler

## 🎮 Advanced Features

### Multisync Functions (LANGUAGE_SPEC.zen lines 213-224)
Functions are sync or async based on the allocator:
```zen
fetch_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)
    @this.defer(client.deinit())
    
    // Blocks with sync allocator, async with async allocator!
    response = client.get(url)
    return parse_response(response)
}

// Usage
sync_data = fetch_data(url, sync_alloc)   // Blocking
async_data = fetch_data(url, async_alloc)  // Non-blocking
```

### Compile-time Metaprogramming (LANGUAGE_SPEC.zen lines 243-282)
```zen
{ reflect, meta } = @std

// Inspect types at compile time
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    // ... manipulate AST ...
}

// Modify functions at compile time
@meta.comptime(() {
    original = reflect.ast(my_function)
    // ... transform AST ...
    meta.replace(my_function, modified)
})
```

## 🤝 Contributing

Contributions are welcome! The language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the source of truth for all language features.

## 📄 License

MIT License - see LICENSE file for details

## 🔗 Resources

- [Language Specification](./LANGUAGE_SPEC.zen) - Complete language design
- [Working Features](./WORKING_FEATURES.md) - Current implementation status
- [Examples](./examples/) - Sample programs
- [Tests](./tests/) - Test suite
- [Standard Library](./stdlib/) - Built-in modules

---

*Zen: Where simplicity meets power - a language without keywords, only expressions*