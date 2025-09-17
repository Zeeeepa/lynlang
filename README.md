# Zen Programming Language

A revolutionary systems programming language that eliminates traditional keywords in favor of pattern-first design and allocator-based async/sync behavior.

**LANGUAGE_SPEC.zen is the authoritative source** - All language features and syntax are defined in `LANGUAGE_SPEC.zen`

## 🎯 Core Philosophy

Zen's revolutionary design eliminates complexity through simplicity:

- **No keywords**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null` 
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)
- **Pattern matching with `?`**: All control flow through pattern matching
- **UFC (Uniform Function Call)**: Any function callable as method
- **Allocator-based concurrency**: No function coloring - sync/async from allocator
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` - no `*` or `&`
- **No null/nil**: Only `Option<T>` with `Some(T)` and `None`
- **No unions, no tuples**: Only structs and enums
- **Three assignment forms**: `=` (immutable), `::=` (mutable), `:` (type)
- **Error propagation**: `.raise()` for automatic error propagation
- **Modern loops**: `loop()` infinite, `.loop()` collections, `(0..10)` ranges
- **Trait system**: `.implements()` and `.requires()` from `@std.meta`
- **Metaprogramming**: Full compile-time AST access and modification

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/zenlang/zen
cd zen

# Build the compiler
cargo build --release

# Run a Zen program
cargo run --bin zen -- examples/hello.zen

# Compile to executable
cargo run --bin zen -- examples/hello.zen -o hello
./hello
```

### Hello World

```zen
// Only @std and @this are special
{ io } = @std

main = () void {
    io.println("Hello, Zen World!")
}
```

## 📚 Language Features

### Variables

```zen
// Immutable (default)
x = 42              // Type inferred
y: i32 = 42         // Type explicit

// Mutable  
count ::= 0         // Type inferred
total :: i32 = 100  // Type explicit

// Forward declaration
z: i32              // Declare type
z = 10              // Assign later
```

### Structs and Enums

```zen
// Struct with mutable and immutable fields
Point: {
    x :: f64,       // Mutable field
    y :: f64 = 0    // Default value
}

// Enum (sum type)
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)

// Simple enum
Shape: Circle | Rectangle
```

### Pattern Matching - The Only Control Flow

```zen
// Boolean patterns (if-else replacement)
is_ready ?
    | true { start_game() }
    | false { show_loading() }

// Option handling
maybe_value: Option<i32> = Some(42)
maybe_value ?
    | Some(n) { io.println("Value: ${n}") }
    | None { io.println("No value") }

// Enum matching
shape ?
    | Circle { draw_circle() }
    | Rectangle { draw_rectangle() }

// Number patterns
score ?
    | 90..100 { grade = "A" }
    | 80..90 { grade = "B" }
    | _ { grade = "F" }
```

### Loops and Ranges

```zen
// Range iteration
(0..10).loop((i) {
    io.println("Index: ${i}")
})

// Step ranges
(0..100).step(5).loop((n) {
    io.println(n)  // 0, 5, 10, 15...
})

// Collection iteration with UFC
numbers = [1, 2, 3, 4, 5]
numbers.loop((n) {
    io.println(n)
})

// With index
items.loop((item, i) {
    io.println("${i}: ${item}")
})

// Infinite loop
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

### Functions and UFC

```zen
// Simple function
add = (a: i32, b: i32) i32 {
    return a + b
}

// UFC - call as method
double = (x: f64) f64 {
    return x * 2
}

value = 10.0
result = value.double()  // Same as double(value)

// Generic function
print_item<T>(item: T) void {
    io.println("Item: ${item}")
}
```

### Traits

```zen
// Define trait
Drawable: {
    draw: (self) void,
    bounds: (self) Rectangle,
}

// Implement for type
Circle.implements(Drawable, {
    draw = (self) void {
        // Drawing logic
    },
    bounds = (self) Rectangle {
        // Return bounding box
    }
})

// Require trait on enum
Shape: Circle | Rectangle
Shape.requires(Drawable)

// Generic with constraint
render<T: Drawable>(item: T) void {
    item.draw()
}
```

### Error Handling

```zen
// Result type for errors
parse_int = (s: string) Result<i32, string> {
    s.to_i32() ?
        | Some(n) { return Ok(n) }
        | None { return Err("Invalid integer") }
}

// Error propagation with .raise()
process_file = (path: string) Result<Data, Error> {
    file = File.open(path).raise()      // Returns early if Err
    contents = file.read_all().raise()  // Returns early if Err
    data = parse(contents).raise()      // Returns early if Err
    return Ok(data)
}
```

### Pointers (No `*` or `&`)

```zen
// Explicit pointer types
value = 42
ptr: Ptr<i32> = value.ref()        // Immutable reference
mut_ptr: MutPtr<i32> = value.mut_ref()  // Mutable reference

// Dereference with .val
io.println("Value: ${ptr.val}")
mut_ptr.val = 100

// Get address
io.println("Address: ${ptr.addr}")
```

### Collections

```zen
// Static sized vector
vec = Vec<i32, 100>()
vec.push(42)

// Dynamic vector with allocator
alloc = GPA.init()
@this.defer(alloc.deinit())

dyn_vec = DynVec<i32>(alloc)
dyn_vec.push(1)
dyn_vec.push(2)

// Mixed type vectors!
entities = DynVec<Player, Enemy>(alloc)
entities.push(Player { health: 100 })
entities.push(Enemy { damage: 50 })

// Pattern match mixed types
entities.loop((entity) {
    entity ?
        | Player { heal(entity) }
        | Enemy { attack(entity) }
})
```

### Concurrency

```zen
// Allocator determines sync/async behavior
sync_alloc = GPA.init()      // Synchronous
async_alloc = AsyncPool.init()  // Asynchronous

// Same function, different behavior!
data1 = fetch_data(url, sync_alloc)   // Blocks
data2 = fetch_data(url, async_alloc)  // Non-blocking

// Channels
chan = Channel<string>(10)
chan.send("Hello")
msg = chan.receive()

// Actors
fib = Actor((receiver) {
    a ::= 0
    b ::= 1
    loop(() {
        receiver.send(a)
        temp = a + b
        a = b
        b = temp
    })
}).spawn()
```

### Metaprogramming

```zen
// Compile-time reflection
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    ast.kind ?
        | Struct(s) {
            s.fields.loop((f) {
                io.println("Field: ${f.name}: ${f.type}")
            })
        }
        | Enum(e) {
            e.variants.loop((v) {
                io.println("Variant: ${v.name}")
            })
        }
}

// AST modification at compile time
@meta.comptime(() {
    original = reflect.ast(my_function)
    new_body = original.body.prepend(
        AST.Call("io.println", ["Entering function"])
    )
    meta.replace(my_function, original.with_body(new_body))
})
```

## 📦 Standard Library

The standard library (`@std`) provides:

- **Core Types**: `Option<T>`, `Result<T,E>`, `Vec<T>`, `DynVec<T>`
- **IO**: File operations, console I/O, networking
- **Concurrency**: `Actor`, `Channel`, `Mutex`, atomics
- **Memory**: `GPA`, `AsyncPool`, custom allocators
- **Collections**: HashMap, Set, Queue, Stack
- **Math**: Common math functions and constants
- **String**: String manipulation and formatting
- **Meta**: Reflection, traits, compile-time programming
- **FFI**: C interop and external libraries

## 🔧 Build System

Zen includes a built-in build system:

```zen
// build.zen
Build := @std

builder = (b :: Build) void {
    // Conditional compilation
    is_release = b.args.contains("--release")
    is_release ?
        | true {
            b.optimization(.O3)
            b.strip_symbols(true)
        }
        | false {
            b.optimization(.O0)
            b.debug_info(true)
        }
    
    // Add executable
    b.add_executable("my-app", "src/main.zen")
        .add_library("my-lib", "src/lib.zen")
        .add_test("my-tests", "tests/test.zen")
}
```

## 🎓 Examples

See the `examples/` directory for complete examples:

- `01_hello_world.zen` - Basic hello world
- `02_variables_and_types.zen` - Variable declarations
- `03_pattern_matching.zen` - Pattern matching examples
- `04_loops.zen` - All loop forms
- `05_structs_and_methods.zen` - Structs and UFC
- `error_handling.zen` - Result types and .raise()
- `concurrent_web_server.zen` - Async web server
- `full_demo/` - Complete application example

## 🧪 Testing

Run the test suite:

```bash
./test_all.sh
```

Create a test file:

```zen
// tests/zen_test_myfeature.zen
{ test } = @std

test.describe("My Feature", () {
    test.it("should work", () {
        result = my_function(42)
        test.expect(result).to_equal(84)
    })
})
```

## 📖 Documentation

- `LANGUAGE_SPEC.zen` - Complete language specification
- `docs/` - Additional documentation
- `stdlib/` - Standard library source

## 🤝 Contributing

Zen is an open-source project. Contributions are welcome!

1. Read `LANGUAGE_SPEC.zen` to understand the language
2. Check existing issues and discussions
3. Submit PRs with tests
4. Follow the no-keywords philosophy

## 📄 License

MIT License - See LICENSE file for details

## 🌟 Philosophy

Zen believes that programming languages have accumulated too much complexity. By removing keywords and using patterns for everything, we create a simpler, more consistent language that's easier to learn and reason about.

No keywords. No complexity. Just patterns.

---

**Remember**: `LANGUAGE_SPEC.zen` is the source of truth. When in doubt, consult the spec!