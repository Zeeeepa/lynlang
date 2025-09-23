# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and powerful metaprogramming.

> *"No keywords. Pure expression. Allocator-driven concurrency."*

## Core Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

1. **No keywords** - No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
2. **Only two @ symbols** - `@std` (standard library) and `@this` (current scope)  
3. **Pattern matching with `?`** - Replaces all conditional keywords
4. **UFC (Uniform Function Call)** - Any function can be called as method
5. **Allocators determine sync/async** - No function coloring
6. **Explicit pointer types** - `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
7. **No null/nil** - Only `Option<T>` with `.Some(T)` and `.None`
8. **No unions, no tuples** - Only structs and enums
9. **Assignment operators** - `=` (immutable), `::=` (mutable), `:` (type definition)
10. **Error propagation** - `.raise()` not exceptions
11. **Loops** - `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
12. **Traits** - via `.implements()` and `.requires()` from `@std.meta`
13. **Compile-time metaprogramming** - Full AST access

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
./target/release/zen <file.zen>

# Run tests
./run_tests.sh
```

## Language Features by Example

All examples directly from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

### Variables - Six Forms, No Keywords

```zen
// From LANGUAGE_SPEC.zen lines 299-306
x: i32               // Forward declaration (immutable)
x = 10               // Assignment
y = 10               // Immutable assignment (type inferred)
z: i32 = 20          // Immutable with type
w:: i32              // Mutable forward declaration
w = 20               // Assignment
v ::= 30             // Mutable assignment (type inferred)
u:: i32 = 40         // Mutable with type
```

### Pattern Matching - Replaces All Conditionals

```zen
// Boolean patterns (lines 352-361)
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }

// Enum patterns (lines 325-335)
entity ?
    | Player { 
        io.println("Player health: ${entity.get_health()}")
    }
    | Enemy { 
        io.println("Enemy health: ${entity.get_health()}")
    }
```

### Structs and Enums

```zen
// Struct definition (lines 117-120)
Point: {
    x:: f64,         // Mutable field
    y:: f64 = 0      // With default value
}

// Enum (sum type) definition (line 166)
Shape: Circle | Rectangle

// Enum with variants for overloading (line 172)
GameEntity: Player | Enemy | Powerup
```

### Traits - Behavioral Types

```zen
// Trait definition (lines 123-127)
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation (lines 136-143)
Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * math.pi * self.radius
    },
})

// Requirement (line 168)
Shape.requires(Geometric)  // All Shape variants must implement Geometric
```

### UFC (Uniform Function Call)

```zen
// UFC overloading based on enum variants (lines 174-181)
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }

// Can call as:
player.get_health()  // UFC style
get_health(player)   // Traditional style
```

### Option and Result - No Null

```zen
// Option type (line 110)
Option<T>: Some(T) | None

// Result type (line 113)
Result<T, E>: Ok(T) | Err(E)

// Option handling (lines 462-473)
maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle { center: Point { x: 100, y: 100 }, radius: r }
        io.println("Created circle with area: ${circle.area()}")
    }
    | None {
        io.println("No radius provided")
    }
```

### Error Propagation with .raise()

```zen
// Error propagation (lines 206-211)
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()      // If Err, returns early
    contents = file.read_all().raise()   // Propagates errors
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Loops and Ranges

```zen
// Range iteration (lines 432-434)
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges (lines 437-439)
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration with UFC (lines 442-445)
shapes.loop((shape) {
    total_area = total_area + shape.area()
})

// Infinite loop (lines 453-459)
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { continue }
})
```

### Allocator-Driven Concurrency

```zen
// Multisync function - sync or async based on allocator! (lines 215-224)
fetch_game_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)  // Behavior determined by allocator
    @this.defer(client.deinit())
    
    // This blocks or doesn't based on allocator!
    response = client.get(url)
    response ?
        | Ok(data) { return Ok(parse_data(data)) }
        | Err(e) { return Err(e) }
}

// Usage (lines 308-314)
sync_alloc = GPA.init()        // Sync allocator - everything blocks
async_alloc = AsyncPool.init() // Async allocator - non-blocking
```

### Pointers - Explicit, No Symbols

```zen
// Explicit pointer types (lines 364-371)
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()          // Immutable pointer
circle_mut: MutPtr<Circle> = circle.mut_ref()   // Mutable pointer

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75                           // Modify through pointer
io.println("Address: ${circle_ptr.addr}")            // Access address
```

### Metaprogramming and Reflection

```zen
// Runtime reflection (lines 244-272)
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

// Compile-time AST modification (lines 275-281)
@meta.comptime((){
    original = reflect.ast(parse_radius)
    new_body = original.body.prepend(
        AST.Call("io.println", ["Parsing radius from: ${s}"])
    )
    meta.replace(parse_radius, original.with_body(new_body))
})
```

### Concurrency Primitives

```zen
// Actors for lazy/streaming iteration (lines 228-240)
create_fibonacci = () Actor {
    outer = 100  // Captured automatically
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

// Channels (lines 397-412)
message_chan = Channel<string>(10)  // Buffered channel
@this.defer(message_chan.close())

// Mutex (lines 415-423)
counter_mutex = Mutex<u32>(0)
counter_mutex.lock() ?
    | Ok(val) {
        val = val + 1
        counter_mutex.unlock()
    }
    | Err(e) { io.println("Lock failed: ${e}") }

// Atomics (lines 426-429)
atomic_counter = AtomicU32(0)
atomic_counter.fetch_add(1)
```

### Mixed Type Vectors

```zen
// DynVec can hold multiple variant types! (lines 316-335)
entities = DynVec<GameEntity.Player, GameEntity.Enemy>(sync_alloc)
entities.push(GameEntity.Player)
entities.push(GameEntity.Enemy)

// Loop over mixed types with pattern matching
entities.loop((entity) {
    entity ?
        | Player { io.println("Player health: ${entity.get_health()}") }
        | Enemy { io.println("Enemy health: ${entity.get_health()}") }
})
```

### Module System

```zen
// Imports (lines 92-106)
{ io, maths } = @std
{ String, StringBuilder } = @std
{ Vec, DynVec } = @std

// Module exports (lines 492-502)
module.exports = {
    Shape: Shape,
    Circle: Circle,
    Rectangle: Rectangle,
    get_health: get_health,
}

// Imports in other files
Circle2D = module.import("shapes2d").Circle
shapes = module.import("shapes2d")
```

## Build System

Zen uses its own build system written in Zen (see `build.zen` in LANGUAGE_SPEC.zen lines 20-85):

```zen
Build := @std

builder = (b :: Build) void {
    // Conditional compilation
    is_release = b.args.contains("--release")
    is_release ?
        | true { b.optimization(.O3) }
        | false { b.debug_info(true) }
    
    // Multiple output targets
    b.target ?
        | C { b.emit_c("output.c") }        // Transpile to C
        | LLVM { b.emit_llvm_ir("output.ll") }  // LLVM IR
        | Native { b.emit_native() }        // Machine code
}
```

## Implementation Status

Current implementation progress: ~45% Complete

See [`docs/status/IMPLEMENTATION_STATUS_CURRENT.md`](./docs/status/IMPLEMENTATION_STATUS_CURRENT.md) for detailed status.

### Working Features
- ✅ Zero keywords design
- ✅ Pattern matching with `?`
- ✅ All 6 variable declaration forms
- ✅ Basic types (integers, floats, bool, string)
- ✅ Structs and enums
- ✅ UFC (Uniform Function Call)
- ✅ String interpolation
- ✅ Range iteration
- ✅ Basic @std imports

### In Progress
- 🚧 Option and Result types
- 🚧 Trait system (.implements/.requires)
- 🚧 Error propagation (.raise)
- 🚧 Allocator system
- 🚧 Pointer types
- 🚧 Metaprogramming

### Not Yet Implemented
- ❌ Actor model
- ❌ Channels and concurrency
- ❌ Compile-time reflection
- ❌ Module system
- ❌ Build system
- ❌ FFI
- ❌ SIMD operations

## Contributing

This project implements the specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen). All contributions must align with this specification.

## License

MIT