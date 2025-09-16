# Zen Language

A modern systems programming language that eliminates keywords in favor of symbols and patterns, featuring uniform function calls, allocator-based async/sync control, and powerful metaprogramming.

## Core Philosophy

Zen rejects traditional keyword-heavy syntax in favor of a minimal, symbol-based approach that treats code as data and enables powerful compile-time metaprogramming.

## Key Design Principles

### No Keywords
Zen has **zero traditional keywords**. No `if`, `else`, `while`, `for`, `match`, `async`, `await`, `impl`, `trait`, `class`, `interface`, or `null`.

### Only Two @ Symbols
- `@std` - Standard library access
- `@this` - Current scope reference

### Pattern Matching Everything
All control flow uses the `?` operator with pattern matching:
```zen
// Boolean conditions
something_true ? { doSomething() }

// Pattern matching with alternatives
value ?
    | .Some(x) { process(x) }
    | .None { handleEmpty() }
```

### Uniform Function Call (UFC)
Any function can be called as a method on its first parameter:
```zen
// These are equivalent:
add(5, 3)
5.add(3)

// Works with any function:
shapes.loop((shape) { ... })
loop(shapes, (shape) { ... })
```

### Allocator-Driven Concurrency
No function coloring! Allocators determine sync/async behavior:
```zen
sync_alloc = GPA.init()      // Synchronous allocator
async_alloc = AsyncPool.init() // Async allocator

// Same function, different behavior based on allocator
data = fetch_data(url, sync_alloc)  // Blocks
data = fetch_data(url, async_alloc) // Non-blocking
```

### Explicit Pointer Types
No `*` or `&` symbols. Clear, explicit pointer types:
- `Ptr<T>` - Immutable pointer
- `MutPtr<T>` - Mutable pointer  
- `RawPtr<T>` - Raw pointer for FFI

### No Null
Only `Option<T>` with `.Some(T)` and `.None` variants. No null pointer exceptions.

### Simple Type System
- Structs and enums only (no unions, no tuples)
- Pattern matching on enum variants
- Traits via `.implements()` and `.requires()`

## Language Features

### Type Definitions
```zen
// Struct
Point: {
    x: f64,
    y: f64,
}

// Enum (sum type) - comma separated variants
Shape: Circle, Rectangle

// Enum with data
GameEntity: Player, Enemy: {health: u32}, Powerup: {type: PowerupType}

// Option type (built-in)
Option<T>: Some: {value: T}, None

// Result type for errors
Result<T, E>: Ok: {value: T}, Err: {error: E}
```

### Assignment Operators
- `=` - Immutable binding
- `::=` - Mutable binding
- `:` - Type definition

```zen
x = 10           // Immutable
y ::= 20         // Mutable
Point: {x: f64}  // Type definition
```

### Imports
```zen
// Import multiple from module
{ io, math } = @std
{ Vec, DynVec } = @std.collections

// Import entire module
core = @std.core

// Import from build dependencies
sdl2 = @std.build.import("sdl2")
```

### Control Flow
```zen
// Boolean conditions
is_ready ? { start_game() }

// Pattern matching
result ?
    | .Ok(value) { process(value) }
    | .Err(e) { handle_error(e) }

// Loops
loop(() { ... })              // Infinite loop
(0..10).loop((i) { ... })     // Range iteration
items.loop((item) { ... })    // Collection iteration
```

### Error Handling
```zen
// Propagate errors with .raise()
load_file = (path: string) Result<Data, Error> {
    file = File.open(path).raise()     // Returns early if Err
    contents = file.read_all().raise()
    return .Ok(parse(contents))
}
```

### Traits
```zen
// Define trait
Drawable: {
    draw: (self) void,
    bounds: (self) Rectangle,
}

// Implement trait
Circle.implements(Drawable, {
    draw = (self) void { ... },
    bounds = (self) Rectangle { ... },
})

// Require trait implementation
Shape.requires(Drawable)
```

### Metaprogramming
```zen
// Compile-time code generation
@meta.comptime(() {
    ast = reflect.ast(MyType)
    // Modify AST...
    meta.replace(MyType, modified_ast)
})

// Runtime reflection
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    ast.kind ?
        | .Struct(s) { ... }
        | .Enum(e) { ... }
}
```

### Memory Management
```zen
// Explicit resource management
alloc = GPA.init()
@this.defer(alloc.deinit())

vec = DynVec<Item>(alloc)
@this.defer(vec.deinit())
```

### Concurrency
```zen
// Channels
chan = Channel<Message>(10)
chan.send(msg)
chan.receive() ? | .Some(m) { ... } | .None { ... }

// Actors
actor = Actor(() { ... }).spawn()

// Atomics
counter = AtomicU32(0)
counter.fetch_add(1)
```

## Example Program

```zen
{ io } = @std

main = () void {
    // Pattern matching for FizzBuzz
    (1..100).loop((n) {
        output = n % 15 == 0 ? { "FizzBuzz" }
               | n % 3 == 0  ? { "Fizz" }
               | n % 5 == 0  ? { "Buzz" }
               | _ { n.to_string() }
        
        io.println(output)
    })
}
```

## Building

```zen
// build.zen
Build = @std.build

builder = (b :: Build) void {
    exe = b.add_executable("my-app", "src/main.zen")
    
    b.args.contains("--release") ?
        | true { b.optimization(.O3) }
        | false { b.debug_info(true) }
}
```

## Getting Started

1. Write your code following Zen's principles
2. Use `?` for all control flow
3. Leverage UFC for method-like syntax
4. Choose allocators based on concurrency needs
5. Pattern match everything

## License

[License information here]