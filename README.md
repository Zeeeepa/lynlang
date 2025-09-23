# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and structural typing via traits.

> "No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`"  
> — LANGUAGE_SPEC.zen, line 2

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
12. **Traits** - Via `.implements()` and `.requires()` from `@std.meta`
13. **Compile-time metaprogramming** - Full AST access

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
cargo run --bin zen examples/hello.zen

# Compile to executable
cargo run --bin zen hello.zen -o hello

# Start the REPL
cargo run --bin zen
```

## Language Features

### Variables and Assignment

```zen
// Three forms of variable declaration
x = 10           // Immutable
y ::= 20         // Mutable with ::=
z: i32 = 30      // Immutable with type annotation
w :: i32 = 40    // Mutable with type annotation

// Forward declarations
a: i32           // Forward declaration (immutable)
a = 100          // Assignment must be in same scope

b :: i32         // Mutable forward declaration
b = 200
```

### Structs and Enums

```zen
// Simple struct
Point: {
    x:: f64,     // Mutable field (note :: for mutable)
    y:: f64 = 0  // With default value
}

// Enum type (sum type)
Shape: Circle | Rectangle

// Enum with values  
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)
```

### Pattern Matching (No if/else!)

```zen
// Boolean pattern matching
is_ready = true
is_ready ? { 
    io.println("Starting!") 
}

// Full pattern match (replaces if-else)
has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting...") }

// Enum pattern matching
maybe_value: Option<i32> = Some(42)
maybe_value ?
    | Some(val) { io.println("Value: ${val}") }
    | None { io.println("No value") }

// Pattern matching with different types
entity: GameEntity = Player
entity ?
    | Player { io.println("Health: 100") }
    | Enemy { io.println("Health: 50") }
    | Powerup { io.println("Health: 0") }
```

### Traits and Implementation

```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Struct that will implement trait
Circle: {
    center: Point,
    radius: f64,
}

// Implement trait using .implements()
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})

// Enforce all variants implement trait
Shape: Circle | Rectangle
Shape.requires(Geometric)
```

### Loops and Ranges

```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration with UFC
items.loop((item) {
    io.println("Item: ${item}")
})

// With index
items.loop((item, index) {
    io.println("${index}: ${item}")
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

### Pointers (No * or &)

```zen
// Explicit pointer types
value = 42
ptr: Ptr<i32> = value.ref()          // Immutable reference
mut_ptr: MutPtr<i32> = value.mut_ref() // Mutable reference
raw: RawPtr<i32> = value.raw_ptr()   // Raw pointer

// Dereferencing with .val
io.println("Value: ${ptr.val}")
mut_ptr.val = 100

// Get address with .addr
io.println("Address: ${ptr.addr}")
```

### Error Handling

```zen
// Result type for errors
Result<T, E>: Ok(T) | Err(E)

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()     // Returns early if Err
    contents = file.read_all().raise() // Propagates errors up
    config = json.parse(contents).raise()
    return Ok(config)
}

// Handle Result with pattern matching
parse_number("123") ?
    | Ok(num) { io.println("Parsed: ${num}") }
    | Err(e) { io.println("Error: ${e}") }
```

### UFC (Uniform Function Call)

```zen
// Any function can be called as method
add = (a: i32, b: i32) i32 { a + b }

// Both work:
result1 = add(5, 3)      // Traditional
result2 = 5.add(3)       // UFC style

// Great for chaining
text = "hello"
    .to_upper()
    .trim()
    .append("!")
```

### Imports and Modules

```zen
// Importing from @std
{ io, math } = @std
{ Vec, DynVec } = @std
{ Actor, Channel, Mutex } = @std

// Module exports
module.exports = {
    Shape: Shape,
    Circle: Circle,
    area: area,
}

// Importing from modules
shapes = module.import("shapes")
my_circle = shapes.Circle { radius: 5 }
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

// Compile-time code modification
@meta.comptime(() {
    original = reflect.ast(my_function)
    // Modify AST...
    meta.replace(my_function, modified)
})
```

### Concurrency

```zen
// Allocators determine sync/async behavior
sync_alloc = GPA.init()        // Synchronous allocator
async_alloc = AsyncPool.init() // Asynchronous allocator

// Same function, different behavior based on allocator!
data1 = fetch_data(url, sync_alloc)  // Blocks
data2 = fetch_data(url, async_alloc) // Non-blocking

// Channels for communication
chan = Channel<string>(10)
chan.send("Hello")
chan.receive() ?
    | Some(msg) { io.println(msg) }
    | None { io.println("Empty") }

// Actors for concurrent tasks
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

## Build System

Zen includes its own build system written in Zen:

```zen
// build.zen
Build := @std

builder = (b :: Build) void {
    is_release = b.args.contains("--release")
    
    // Conditional compilation
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
        .add_library("utils", "src/utils.zen")
        .add_test("tests", "tests/test.zen")
}
```

## Examples Directory

The `examples/` directory contains working demonstrations:

- `01_hello_world.zen` - Basic hello world
- `02_variables_and_types.zen` - Variable declarations
- `03_pattern_matching.zen` - Pattern matching examples
- `04_loops.zen` - Loop constructs
- `05_structs_and_methods.zen` - Structs and methods
- `error_handling.zen` - Error handling patterns
- `concurrent_web_server.zen` - Async web server example

## Testing

All tests should be prefixed with `zen_` and placed in the `tests/` folder:

```bash
# Run all tests
./scripts/test_all.sh

# Run specific test
cargo run --bin zen tests/zen_test_simple.zen
```

## Implementation Status

The compiler is implemented in Rust using LLVM for code generation. Current features include:

✅ Basic types and variables  
✅ Functions and UFC  
✅ Structs and enums  
✅ Pattern matching with `?`  
✅ String interpolation  
✅ Range iteration  
✅ Module system  
✅ Traits via .implements()  
✅ Error propagation with .raise()  
✅ Pointer types  
✅ Standard library basics  

## Contributing

All contributions must align with [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen). The spec is the authoritative source for language behavior.

## License

MIT