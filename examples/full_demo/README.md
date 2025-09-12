# Zenlang Full Demo Suite

This directory contains comprehensive demonstrations of Zenlang's capabilities, showcasing the language's unique features and design philosophy.

## 🎯 Demo Files

### 1. `main.zen` - Complete Feature Showcase
The main demonstration file that covers all major language features:
- Pattern matching with `?` operator
- Memory management with smart pointers
- Behaviors (traits/interfaces)
- Compile-time evaluation
- Colorless async
- UFCS (Uniform Function Call Syntax)
- Error handling without exceptions
- String interpolation
- Advanced loop patterns

### 2. `builder_demo.zen` - FFI Builder Pattern
Demonstrates Zenlang's powerful Foreign Function Interface:
- Building bindings for SQLite3, OpenGL, and system libraries
- Platform-specific configuration
- Opaque types for FFI safety
- Callback definitions with trampolines
- Database and graphics programming examples

### 3. `self_hosting_demo.zen` - Compiler in Zenlang
A simplified compiler implementation written in Zenlang itself:
- Lexer with complete tokenization
- Parser with AST generation
- Code generator
- Demonstrates self-hosting capabilities

## 🚀 Running the Demos

### Prerequisites
```bash
# Build the Zenlang compiler
cd ../..
cargo build --release
```

### Run Individual Demos
```bash
# Run the main comprehensive demo
cargo run --bin zen -- run examples/full_demo/main.zen

# Run the FFI builder demo
cargo run --bin zen -- run examples/full_demo/builder_demo.zen

# Run the self-hosting compiler demo
cargo run --bin zen -- run examples/full_demo/self_hosting_demo.zen
```

## 📚 Key Language Features

### No Traditional Control Flow
```zen
// No if/else - use pattern matching
age ? | 0..=17 => "Minor"
      | 18..=64 => "Adult"
      | _ => "Senior"

// No while/for - use loop
loop (condition) { body }
(0..10).loop((i) => { process(i) })
```

### Pattern Matching Everywhere
```zen
// Result handling
result ? | .Ok -> value => process(value)
         | .Err -> error => handle(error)

// Option handling
option ? | .Some -> val => use(val)
         | .None => default_value()

// Guards
n ? | x -> x > 100 => "Large"
    | x -> x > 0 => "Small"
    | _ => "Zero or negative"
```

### Memory Safety Without Annotations
```zen
// No raw pointers - use Ptr<T>
ptr := Ptr<i32>::new(42)
value := ptr.value      // Safe access
addr := ptr.address     // Get address

// RAII with defer
resource := acquire()
defer resource.release()  // Automatic cleanup
```

### Compile-Time Evaluation
```zen
// Generate lookup tables at compile time
TABLE := comptime {
    table:: [256, u8]
    (0..256).loop((i) => {
        table[i] = compute(i)
    })
    table
}
```

### Colorless Async
```zen
// Same function works sync or async based on allocator
read_file = (path: string, alloc: Ptr<Allocator>) Result<string, Error> {
    // No async/await keywords needed
    file := fs.open(path, alloc)?
    defer file.close()
    file.read_all(alloc)
}
```

## 🔧 FFI Integration

The FFI builder pattern provides safe, ergonomic foreign function interfaces:

```zen
sqlite := ffi.FFI.lib("sqlite3")
    .platform_path({
        .linux => "/usr/lib/libsqlite3.so",
        .macos => "/usr/local/lib/libsqlite3.dylib",
        .windows => "sqlite3.dll"
    })
    .function("sqlite3_open", signature)
    .constant("SQLITE_OK", ffi.Type::I32)
    .validate()
    .build()
```

## 🏗️ Self-Hosting Progress

The `self_hosting_demo.zen` shows how Zenlang can compile itself:
- **Lexer**: Complete tokenization ✅
- **Parser**: Full AST generation ✅
- **Type Checker**: Semantic analysis 🚧
- **Code Generator**: LLVM backend 🚧

## 📖 Learning Path

1. Start with `main.zen` to understand core concepts
2. Explore `builder_demo.zen` for FFI and system programming
3. Study `self_hosting_demo.zen` to see language implementation

## 🎯 Design Philosophy

Zenlang follows the "NO" manifesto:
- **NO** `if`/`else`/`match` keywords → Use `?` operator
- **NO** exceptions → Errors are values
- **NO** null pointers → Option<T> for optional values
- **NO** implicit conversions → All conversions explicit
- **NO** lifetime annotations → Smart pointers handle safety
- **NO** raw pointers → Use Ptr<T> with .value/.address
- **NO** function coloring → Colorless async via allocators

## 📝 Notes

- These demos showcase the intended language design per specification v1.1.0
- Some features may still be under development
- Check the main README for current implementation status

## 🤝 Contributing

Found an issue or want to improve the demos? Contributions are welcome!
- Report issues: [GitHub Issues](https://github.com/lantos1618/zenlang/issues)
- Submit PRs with improvements or new examples

---

**Keep it Zen. 🧘**