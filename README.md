# Zen Programming Language

A revolutionary systems programming language that eliminates traditional keywords in favor of pattern-first design and allocator-based async/sync behavior.

**Implementation Status**: Bootstrap C compiler (`zenc3.c`) implementing core features
- ✅ Basic features working: variables, functions, arithmetic, @std.io.println
- ⚠️ Parser recognizes advanced syntax but code generation incomplete
- ❌ Advanced features not yet implemented: pattern matching, traits, allocators, loops

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
# Build the C bootstrap compiler
gcc -std=c99 -o zenc3 zenc3.c

# Compile a Zen program
./zenc3 myprogram.zen

# Run the generated executable
./output.c.out

# Or specify custom output
./zenc3 myprogram.zen -o myprogram.c
```

### Hello World

```zen
// hello.zen
main = () void {
    @std.io.println("Hello from Zen!")
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
y ::= 20         // Mutable assignment (parsed, codegen incomplete ⚠️)
z: i32 = 30      // Immutable with type annotation (parsed, codegen incomplete ⚠️)
w :: i32 = 40    // Mutable with type annotation (parsed, codegen incomplete ⚠️)
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
// Infinite loop (not yet implemented ❌)
loop(() {
    @std.io.println("Forever...")
    should_stop ? { break }
})

// Range iteration (not yet implemented ❌)
(0..10).loop((i) {
    @std.io.println("Index:")
})

// Inclusive range (not yet implemented ❌)
(1..=5).loop((i) {
    @std.io.println(i)
})

// Collection iteration with UFC (not yet implemented ❌)
items.loop((item) {
    process(item)
})
```

### Types and Structs

```zen
// Simple struct (not yet implemented ❌)
Point: {
    x: i32,
    y: i32,
}

// Create struct instance (not yet implemented ❌)
p ::= Point { x: 10, y: 20 }
@std.io.println(p.x)  // Field access planned

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
├── zenc3.c              # Current working bootstrap C compiler ✅
├── zenc2.c              # Previous attempt (has parser bugs)
├── zenc.c               # Original simple compiler
├── compiler/            # Self-hosted Zen compiler (future)
│   ├── lexer.zen       # Tokenization 
│   ├── parser.zen      # AST generation
│   └── errors.zen      # Error handling
├── stdlib/              # Standard library (in progress)
│   ├── io.zen          # I/O operations
│   ├── math.zen        # Mathematical functions
│   └── mem.zen         # Memory management
├── tests/               # Test suite
│   └── zen_test_*.zen  # Test files
└── .agent/              # Development tracking
    └── context.md       # Current implementation status
```

## 🧪 Running Tests

```bash
# Compile the compiler
gcc -std=c99 -o zenc3 zenc3.c

# Run a specific test
./zenc3 tests/zen_test_basic_working.zen
./output.c.out

# Test simple program
./zenc3 test_simple.zen && ./output.c.out
```

## 📊 Implementation Status

### Working Features (✅)
Core features implemented in `zenc3.c`:
- **Basic Variables**: Immutable assignment with `=`
- **Functions**: Basic function declaration `main = () void { }`
- **Arithmetic**: Basic operators (+, -, *, /)
- **Number Literals**: Integer and floating-point
- **String Literals**: Basic string support
- **Boolean Literals**: `true` and `false`
- **Comments**: Single-line `//` comments
- **@std.io.println**: Output strings to console

### Partially Implemented (⚠️)
Features that are parsed but not fully working:
- **Mutable Assignment**: `::=` operator recognized
- **Type Annotations**: `:` for types parsed
- **@std Module System**: Only `@std.io.println` works

### Not Yet Implemented (❌)
Features defined in LANGUAGE_SPEC.zen but not implemented:
- **Pattern Matching**: `?` operator
- **Option/Result Types**: `Option<T>` with `Some/None`
- **Struct Types**: Definition and instantiation
- **String Interpolation**: `"Value: ${x}"`
- **Loops**: `loop()` and `.loop()` syntax
- **Ranges**: `(0..10)` syntax
- **UFC**: Uniform Function Call
- **Enums**: Variant syntax
- **Destructuring**: `{ io, math } = @std`
- **Traits**: `.implements()` and `.requires()`
- **Pointer Types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`
- **Defer**: `@this.defer()`
- **Allocators**: Sync/async behavior control
- **Metaprogramming**: `@meta.comptime()`
- **Concurrency**: Actors, Channels, Mutex

## 🤝 Contributing

Zen is actively being developed. The language specification in `LANGUAGE_SPEC.zen` is the source of truth for all language features.

## 📜 License

This project is open source. See LICENSE file for details.

## 🔗 Resources

- [Language Specification](./LANGUAGE_SPEC.zen) - Complete language design
- [Compiler Documentation](./compiler/README.md) - Compiler internals
- [Standard Library Docs](./std/README.md) - Standard library reference