# Zen Programming Language

A modern systems programming language that emphasizes simplicity, consistency, and safety without sacrificing performance.

## Key Design Principles

### No Traditional Keywords
Zen eliminates traditional control flow keywords to achieve a more uniform and composable syntax:
- ❌ No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- ✅ Pattern matching with `?` operator
- ✅ Uniform function call syntax (UFC)
- ✅ Everything is an expression

### Only Two Special Symbols
- `@std` - Standard library access
- `@this` - Current scope reference

### Core Language Features

#### Pattern Matching
Replace traditional conditionals with the `?` operator:

```zen
// Simple boolean check
is_ready ? { 
    io.println("Starting!") 
}

// Full pattern match
value ?
    | true { process() }
    | false { wait() }

// Enum pattern matching
shape ?
    | Circle { io.println("Circle area: ${shape.area()}") }
    | Rectangle { io.println("Rectangle area: ${shape.area()}") }
```

#### Assignment Operators
- `=` - Immutable binding
- `::=` - Mutable binding
- `:` - Type definition

```zen
x = 42           // Immutable
y ::= 100        // Mutable
Point: {x: f64, y: f64}  // Type definition
```

#### No Null - Only Option Types
```zen
Option<T>: .Some(T) | .None

maybe_value: Option<i32> = .Some(42)
maybe_value ?
    | .Some(val) { io.println("Value: ${val}") }
    | .None { io.println("No value") }
```

#### Error Handling with Result
```zen
Result<T, E>: .Ok(T) | .Err(E)

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // Returns early if Err
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return .Ok(config)
}
```

#### Explicit Pointer Types
No `*` or `&` syntax - explicit pointer types instead:
- `Ptr<T>` - Immutable pointer
- `MutPtr<T>` - Mutable pointer
- `RawPtr<T>` - Raw pointer for FFI

```zen
value = 42
ptr: Ptr<i32> = value.ref()
mut_ptr: MutPtr<i32> = value.mut_ref()
io.println("Value: ${ptr.val}, Address: ${ptr.addr}")
```

#### Loops and Iteration
```zen
// Infinite loop
loop(() {
    // ...
})

// Collection iteration with UFC
items.loop((item) {
    process(item)
})

// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})
```

#### Uniform Function Call (UFC)
Any function can be called as a method:
```zen
// Traditional call
add(1, 2)

// UFC style
1.add(2)

// Works with any function
"hello".println()  // Same as io.println("hello")
```

## Type System

### Structs
```zen
Point: {
    x: f64,
    y: f64,
}

Circle: {
    center: Point,
    radius: f64,
}
```

### Enums (Sum Types)
```zen
Shape: Circle, Rectangle

GameEntity: .Player, .Enemy, .Powerup
```

### Traits
```zen
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})
```

### Generics
```zen
Container<T: Geometric>: {
    items: DynVec<T>,
    add: (item: T) void,
    total_area: () f64,
}
```

## Memory Management

### Allocator-Based Async/Sync
Functions are sync or async based on the allocator, not function coloring:

```zen
fetch_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)
    // This blocks or doesn't based on allocator!
    response = client.get(url)
    return response
}

// Usage
sync_alloc = GPA.init()          // Sync allocator - blocks
async_alloc = AsyncPool.init()   // Async allocator - non-blocking

data = fetch_data(url, sync_alloc)   // Blocking call
data = fetch_data(url, async_alloc)  // Non-blocking call
```

### Collections
```zen
// Fixed-size vector
shapes = Vec<Shape, 100>()

// Dynamic vector with allocator
dynamic_shapes = DynVec<Shape>(allocator)
@this.defer(dynamic_shapes.deinit())
```

## Metaprogramming

### Compile-Time Reflection
```zen
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    ast.kind ?
        | .Struct(s) {
            io.println("Struct: ${s.name}")
            s.fields.loop((f) {
                io.println("  Field: ${f.name}: ${f.type}")
            })
        }
        | .Enum(e) {
            io.println("Enum: ${e.name}")
        }
}
```

### AST Modification
```zen
@meta.comptime((){
    original = reflect.ast(my_function)
    new_body = original.body.prepend(
        AST.Call("io.println", ["Debug: entering function"])
    )
    meta.replace(my_function, original.with_body(new_body))
})
```

## Concurrency

### Actors
```zen
fibonacci_actor = Actor((receiver) {
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

### Channels
```zen
chan = Channel<string>(10)  // Buffered channel
chan.send("Hello")
chan.receive() ?
    | .Some(msg) { io.println(msg) }
    | .None { io.println("Channel closed") }
```

### Mutex and Atomics
```zen
counter = Mutex<u32>(0)
counter.lock() ?
    | .Ok(val) {
        val = val + 1
        counter.unlock()
    }
    | .Err(e) { handle_error(e) }

atomic = AtomicU32(0)
atomic.fetch_add(1)
```

## Build System

```zen
Build := @std.build

builder = (b :: Build) void {
    // Conditional compilation
    b.args.contains("--release") ?
        | true {
            b.optimization(.O3)
            b.strip_symbols(true)
        }
        | false {
            b.optimization(.O0)
            b.debug_info(true)
        }
    
    // Target selection
    b.target ?
        | .C { b.emit_c("output.c") }
        | .LLVM { b.emit_llvm_ir("output.ll") }
        | .Native { b.emit_native() }
    
    builder.add_executable("my-app", "src/main.zen")
}
```

## FFI Integration

```zen
sdl2_library :: FFI.Library = {
    name: "SDL2",
}

std.os ? 
    | .Linux { sdl2_library.path = "/usr/lib/libSDL2.so" }
    | .Macos { sdl2_library.path = "/usr/local/lib/libSDL2.dylib" }
    | .Windows { sdl2_library.path = "SDL2.dll" }

// Inline C for low-level operations
fast_memcpy = (dst: RawPtr<u8>, src: RawPtr<u8>, len: usize) void {
    inline.c("""
        memcpy(${dst.addr}, ${src.addr}, ${len});
    """)
}
```

## Module System

### Imports
```zen
// Import from standard library
{ io, maths } = @std
{ Vec, DynVec } = @std.collections

// Import specific module
core = @std.core
io = @std.io

// Build imports
sdl2 = @std.build.import("sdl2")
```

### Exports
```zen
module.exports = {
    Shape: Shape,
    Circle: Circle,
    Rectangle: Rectangle,
}
```

## Getting Started

### Installation
```bash
# Clone the repository
git clone https://github.com/zenlang/zen
cd zen

# Build the compiler
zen build
```

### Hello World
```zen
{ io } = @std

main = () void {
    io.println("Hello, Zen!")
}
```

### Running Programs
```bash
# Compile and run
zen run hello.zen

# Build executable
zen build hello.zen -o hello

# Build with optimizations
zen build hello.zen --release
```

## Project Structure

```
zenlang/
├── compiler/        # Self-hosted compiler implementation
├── stdlib/          # Standard library
├── lsp/            # Language server protocol implementation
├── examples/       # Example programs
├── tests/          # Test suite
├── tools/          # Development tools
└── bootstrap/      # Bootstrap compiler
```

## Philosophy

Zen is designed around the principle that **simplicity leads to power**. By removing traditional keywords and control structures in favor of uniform patterns, the language becomes more composable and easier to reason about. Every construct follows consistent rules:

- Everything is an expression
- Pattern matching replaces conditionals
- Functions are first-class and composable via UFC
- Memory safety through explicit pointer types and Option types
- Concurrency without function coloring through allocator-based async

## Contributing

Contributions are welcome! Please read our contributing guidelines and code of conduct before submitting PRs.

## License

Zen is licensed under the MIT License. See LICENSE file for details.