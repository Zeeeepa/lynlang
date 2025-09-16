# Zen Programming Language

A revolutionary systems programming language that eliminates traditional keywords in favor of pattern-first design and allocator-based async/sync behavior.

**Implementation Status**: ~50% of LANGUAGE_SPEC.zen implemented
- ✅ Core features working: variables, functions, structs, arithmetic, range loops
- ⚠️ Pattern matching partially working
- ❌ Advanced features not yet implemented: traits, allocators, metaprogramming

## 🎯 Core Philosophy

Zen follows the principles defined in `LANGUAGE_SPEC.zen`:
- **No traditional keywords**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Pattern matching everywhere**: All control flow via the `?` operator
- **UFC (Uniform Function Call)**: Any function can be called as a method
- **Allocator-determined behavior**: Sync/async determined by allocator, not function coloring
- **No null**: Only `Option<T>` with `.Some(T)` and `.None`
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)

## 🚀 Quick Start

### Building the Compiler

```bash
# Build the Rust-based compiler
cargo build --release

# Run a Zen program directly
./target/release/zen myprogram.zen

# Or compile to executable (coming soon)
./target/release/zen myprogram.zen -o myprogram
```

### Hello World

```zen
// hello.zen
io = @std

main = () i32 {
    io.println("Hello from Zen!")
    return 0
}
```

## 📖 Language Features

### Special Symbols
Only two `@` symbols exist in Zen:
- `@std` - Access to standard library
- `@this` - Reference to current scope

### Assignment Operators
```zen
x = 10           // Immutable binding (working ✅)
y ::= 20         // Mutable assignment (working ✅)
z: i32 = 30      // Immutable with type annotation (working ✅)
w :: i32 = 40    // Mutable with type annotation (working ✅)
```

### Pattern Matching
All control flow uses the `?` operator:

```zen
// Simple boolean pattern
is_ready ? { start_game() }

// Multi-branch pattern
score ?
    | 0..50 { io.println("Beginner") }
    | 50..90 { io.println("Intermediate") }
    | 90..=100 { io.println("Expert") }
    | _ { io.println("Invalid score") }

// Option type matching (no null!)
result ?
    | Some(value) { process(value) }
    | None { io.println("No value") }

// Result type for errors
file_result ?
    | Ok(content) { io.println(content) }
    | Err(error) { io.eprintln("Error: ${error}") }
```

### Loops
No `while` or `for` keywords - only `loop`:

```zen
// Infinite loop (partially working ⚠️)
loop(() {
    io.println("Forever...")
    should_stop ? { break }
})

// Range iteration (working ✅)
(0..10).loop((i) {
    io.print("Index: ")
    io.print_int(i)
    io.println("")
})

// Inclusive range (working ✅)
(1..=5).loop((i) {
    io.print_int(i)
})

// Collection iteration with UFC (not yet implemented ❌)
items.loop((item) {
    process(item)
})
```

### Types and Structs

```zen
// Simple struct (working ✅)
Point: {
    x: i32,
    y: i32,
}

// Create struct instance
p ::= Point { x: 10, y: 20 }
io.print_int(p.x)  // Field access works!

// Enum (sum type) - parsing works, codegen incomplete ⚠️
Shape: Circle(radius: f64) | Rectangle(width: f64, height: f64)

// Generic types
Container<T>: {
    items: DynVec<T>,
    add: (item: T) void,
}
```

### Traits and Implementation

```zen
// Define a trait
Drawable: {
    draw: (self) void,
}

// Implement trait for type
Circle.implements(Drawable, {
    draw = (self) void {
        io.println("Drawing circle with radius ${self.radius}")
    }
})

// Require trait implementation
Shape.requires(Drawable)
```

### Memory Management

```zen
// No * or & operators - explicit pointer types
ptr: Ptr<Circle> = circle.ref()
mut_ptr: MutPtr<Circle> = circle.mut_ref()

// Dereference with .val
area = ptr.val.area()

// Allocators determine sync/async behavior
sync_alloc = GPA.init()
async_alloc = AsyncPool.init()
@this.defer(sync_alloc.deinit())
@this.defer(async_alloc.deinit())

// Same function, different behavior based on allocator!
data1 = fetch_data(url, sync_alloc)   // Blocks
data2 = fetch_data(url, async_alloc)  // Non-blocking
```

### Error Handling

```zen
// No exceptions - explicit error propagation
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // .raise() returns early if Err
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Metaprogramming

```zen
// Compile-time reflection
@meta.comptime(() {
    ast = reflect.ast(MyType)
    ast.kind ?
        | Struct(s) {
            s.fields.loop((field) {
                io.println("Field: ${field.name}: ${field.type}")
            })
        }
        | _ {}
})

// AST manipulation
original = reflect.ast(my_function)
new_body = original.body.prepend(logging_statement)
meta.replace(my_function, original.with_body(new_body))
```

## 🏗️ Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen    # Complete language specification (source of truth)
├── src/                 # Rust compiler implementation
│   ├── lexer.rs        # Tokenization (working ✅)
│   ├── parser/         # AST generation (mostly working ✅)
│   ├── typechecker/    # Type checking (basic features ✅)
│   └── codegen/llvm/   # LLVM code generation (partial ⚠️)
├── compiler/            # Self-hosted Zen compiler (in progress)
│   ├── lexer.zen       # Tokenization 
│   ├── parser.zen      # AST generation
│   └── type_checker.zen # Type checking
├── stdlib/              # Standard library
│   ├── option_result.zen # Option and Result types
│   ├── io.zen         # I/O operations
│   └── math.zen       # Mathematical functions
├── tests/               # Test suite
│   └── zen_test_*.zen # Test files
└── zenc.c              # Bootstrap C compiler (deprecated)
```

## 🧪 Running Tests

```bash
# Run a specific test
./target/release/zen tests/zen_test_basic.zen

# Test working features
./target/release/zen tests/zen_test_language_spec_working.zen
```

## 📊 Implementation Status

### Working Features (✅)
Core features fully implemented and tested:
- **Variables**: All declaration patterns (`=`, `:=`, `:: i32 =`, etc.)
- **Functions**: Definition, calls, return values
- **Structs**: Definition, instantiation, field access
- **Arithmetic**: All basic operators (+, -, *, /, %)
- **Comparisons**: All comparison operators (<, >, <=, >=, ==, !=)
- **Range Loops**: `(0..10).loop()` and `(0..=10).loop()` 
- **I/O**: Basic print functions
- **@std Reference**: Standard library access

### Partially Working (⚠️)
- **Pattern Matching**: Simple patterns work, complex patterns have issues
- **Enums**: Parsing complete, codegen incomplete
- **Infinite Loops**: Basic structure, needs break/continue support

### Not Yet Implemented (❌)
- **Option/Result Types**: Defined but not integrated
- **Pointer Types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`
- **Container Types**: `Vec<>`, `DynVec<>`
- **UFC**: Uniform Function Call for all functions
- **Traits**: `.implements()` and `.requires()`
- **Error Propagation**: `.raise()`
- **Defer Statements**: `@this.defer()`
- **Allocator System**: Sync/async behavior control
- **Metaprogramming**: Compile-time reflection and AST manipulation
- **Concurrency**: Actors, Channels, Mutex
- **Module System**: Import/export

Original core features from bootstrap compiler:
- ✅ Lexer with all operators including `::=`
- ✅ Parser with pattern matching support
- ✅ @std and @this special symbols
- ✅ Basic C code generation
- ✅ Option and Result types
- ✅ Standard library foundation

In progress:
- 🚧 Type system and semantic analysis
- 🚧 Full pattern matching compilation
- 🚧 UFC implementation
- 🚧 Allocator framework
- 🚧 Metaprogramming support

## 🤝 Contributing

Zen is actively being developed. The language specification in `LANGUAGE_SPEC.zen` is the source of truth for all language features.

## 📜 License

This project is open source. See LICENSE file for details.

## 🔗 Resources

- [Language Specification](./LANGUAGE_SPEC.zen) - Complete language design
- [Compiler Documentation](./compiler/README.md) - Compiler internals
- [Standard Library Docs](./std/README.md) - Standard library reference