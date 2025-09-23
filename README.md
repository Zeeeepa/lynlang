# Zen Programming Language

A modern systems programming language with unique design philosophy based on minimalism and expressiveness. The language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the **source of truth**.

## Core Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

- **No keywords**: `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)
- **Pattern matching with `?` operator**, no `match` or `switch`
- **UFC (Uniform Function Call)** - any function can be called as method
- **Allocators determine sync/async behavior** (no function coloring)
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **No null/nil** - only `Option<T>` with `.Some(T)` and `.None`
- **No unions, no tuples** - only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type definition)
- **Error propagation with `.raise()`** not exceptions
- **Loops**: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
- **Traits via `.implements()` and `.requires()`** from `@std.meta`
- **Compile-time metaprogramming** with full AST access

## Quick Start

### Building the Compiler
```bash
cargo build --release
```

### Running Zen Programs
```bash
# Run directly
./target/release/zen program.zen

# Compile to executable
./target/release/zen program.zen -o program
./program

# Start REPL
./target/release/zen
```

### Run Test Suite
```bash
# Run all tests
./run_tests.sh

# Test specific feature demo
./target/release/zen tests/zen_test_spec_working_comprehensive.zen
```

## Language Features

All examples below are from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

### Variable Declarations (lines 298-306)

```zen
// All variable declaration forms
x: i32              // forward declaration (must be in same scope)
x = 10              
y = 10              // Immutable assignment
z: i32 = 20         // Immutable assignment with type
w:: i32             // mutable forward declaration (must be in same scope)
w = 20              
v ::= 30            // Mutable assignment
u:: i32 = 40        // mutable assignment with type
```

### Imports (lines 92-107)

```zen
// Only @std and @this are special
{ io, maths } = @std
{ String, StringBuilder } = @std
{ Vec, DynVec } = @std

// Module imports
sdl2 = @std.import("sdl2")
ecs = @std.import("ecs")
```

### Pattern Matching with ? (lines 29-43, 352-361)

```zen
// Boolean pattern matching - no ternary
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

// For if-else, use full pattern match
has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }

// Conditional compilation style
is_release ?
    | true {
        b.optimization(.O3)
        b.strip_symbols(true)
    }
    | false {
        b.optimization(.O0)
        b.debug_info(true)
    }
```

### No Null - Option Types (lines 109-110, 462-473)

```zen
// Option type definition
Option<T>: Some(T) | None

// Usage
maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle {
            center: Point { x: 100.0, y: 100.0 },
            radius: r,
        }
        io.println("Created circle with area: ${circle.area()}")
    }
    | None {
        io.println("No radius provided")
    }
```

### Result Type for Error Handling (lines 113-114, 199-211)

```zen
// Result type definition
Result<T, E>: Ok(T) | Err(E)

// Parse function returning Result
parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid radius") }
}

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // If Err, returns early
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Structs (lines 117-120, 129-163)

```zen
// Simple struct
Point: {
    x:: f64,  // mutable field
    y:: f64 = 0  // with default value
}

// Complex struct
Circle: {
    center: Point,
    radius: f64,
}

// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implement trait for type using .implements()
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})
```

### Enums (lines 165-182)

```zen
// Enum type (sum type)
Shape: Circle | Rectangle

// Enforce all Shape variants must implement Geometric
Shape.requires(Geometric)

// Enum with different variants
GameEntity: Player | Enemy | Powerup

// UFC overloading based on enum variants
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }
```

### Loops and Ranges (lines 432-459)

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
dynamic_shapes.loop((shape) {
    total_area = total_area + shape.area()
})

// Loop with index
dynamic_shapes.loop((shape, i) {
    io.println("Shape ${i}: ${shape.area()}")
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { io.println("Count: ${counter}") }
})
```

### Pointers (lines 363-372)

```zen
// Explicit pointer types - no * or &
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75
io.println("New area: ${circle_mut.val.area()}")
io.println("Address: ${circle_ptr.addr}")
```

### Allocators and Async (lines 213-224, 308-314)

```zen
// Sync allocator - everything blocks
sync_alloc = GPA.init()
@this.defer(sync_alloc.deinit())

// Async allocator - everything is non-blocking
async_alloc = AsyncPool.init()
@this.defer(async_alloc.deinit())

// Multisync function - sync or async based on allocator!
fetch_game_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)
    @this.defer(client.deinit())
    
    // This blocks or doesn't based on allocator!
    response = client.get(url)
    response ?
        | Ok(data) { return Ok(parse_data(data)) }
        | Err(e) { return Err(e) }
}
```

### Actors and Concurrency (lines 227-241, 398-412)

```zen
// Actor for lazy/streaming iteration
create_fibonacci = () Actor {
    outer = 100  // Will be captured automatically
    return Actor((receiver) {
        a ::= 0
        b ::= 1
        loop(() {
            receiver.send(a + outer)
            temp = a + b
            a = b
            b = temp
        })
    })
}

// Channel communication
message_chan = Channel<string>(10)  // Buffered channel
@this.defer(message_chan.close())

// Spawn actor
sender = Actor(() {
    (0..5).loop((i) {
        message_chan.send("Message ${i}")
    })
}).spawn()

// Receive messages
loop(() {
    message_chan.receive() ?
        | Some(msg) { io.println("Received: ${msg}") }
        | None { break }
})
```

### Metaprogramming (lines 244-282)

```zen
// AST reflection at runtime
inspect_type = (T: type) void {
    ast = reflect.ast(T)
    ast.kind ?
        | Struct(s) {
            io.println("Struct: ${s.name}")
            s.fields.loop((f) {
                io.println("  Field: ${f.name}: ${f.type}")
            })
        }
        | Enum(e) {
            io.println("Enum: ${e.name}")
            e.variants.loop((v) {
                io.println("  Variant: ${v.name}")
            })
        }
}

// Compile-time AST modification
@meta.comptime((){
    original = reflect.ast(parse_radius)
    new_body = original.body.prepend(
        AST.Call("io.println", ["Parsing radius from: ${s}"])
    )
    meta.replace(parse_radius, original.with_body(new_body))
})
```

### Inline C/LLVM and SIMD (lines 285-294)

```zen
// Inline C for low-level control
fast_memcpy = (dst: RawPtr<u8>, src: RawPtr<u8>, len: usize) void {
    inline.c("""
        memcpy(${dst.addr}, ${src.addr}, ${len});
    """)
}

// SIMD operations
vector_add = (a: Vec<f32, 8>, b: Vec<f32, 8>) Vec<f32, 8> {
    return simd.add(a, b)
}
```

### Module System (lines 491-510)

```zen
// Module exports - simple record syntax
module.exports = {
    Shape: Shape,
    Circle: Circle,
    Rectangle: Rectangle,
    Geometric: Geometric,
}

// Imports in other files
Circle2D = module.import("shapes2d").Circle
Rectangle2D = module.import("shapes2d").Rectangle

// Or grab the whole module
shapes = module.import("shapes2d")
my_circle = shapes.Circle { ... }
```

## Current Implementation Status

### Working Features
- All variable declaration forms (immutable, mutable, forward declarations)
- Pattern matching with `?` operator for booleans
- Structs with mutable fields and default values
- UFC (Uniform Function Call) - functions callable as methods
- Range syntax `(start..end)` with `.loop()` iteration
- String interpolation with `${expr}`
- Basic arithmetic and comparison operators
- Loop control flow with `break`
- Standard library imports with `@std`
- LLVM backend code generation

### In Progress
- Option and Result types with full pattern matching
- Error propagation with `.raise()`
- Traits via `.implements()` and `.requires()`
- Enum types with variant-based UFC overloading
- Allocator-based async/sync behavior
- Actor model and channels
- Compile-time metaprogramming
- Module system with exports/imports

### Testing

The test suite in `/tests` validates compliance with `LANGUAGE_SPEC.zen`:

- `zen_test_spec_working_comprehensive.zen` - Tests all working features
- `zen_test_spec_core_demo.zen` - Core language features
- `zen_test_spec_ranges_demo.zen` - Range and loop features  
- `zen_test_spec_structs_demo.zen` - Struct features

## Development

### Project Structure
```
zenlang/
├── LANGUAGE_SPEC.zen      # Source of truth specification
├── src/                   # Rust implementation
│   ├── ast/              # Abstract syntax tree
│   ├── parser/           # Parser implementation
│   ├── typechecker/      # Type system
│   ├── codegen/          # LLVM code generation
│   └── stdlib/           # Standard library
├── tests/                # Test suite (zen_test_* prefix)
└── examples/             # Example programs
```

### Contributing

All contributions must align with `LANGUAGE_SPEC.zen`. The specification is the authoritative source for language behavior.

## License

MIT

## Contact

For questions or feedback, open an issue on GitHub.