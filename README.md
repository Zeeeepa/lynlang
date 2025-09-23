# Zen Programming Language

**`LANGUAGE_SPEC.zen` IS THE SOURCE OF TRUTH**

A modern systems programming language with a radically minimal design - no keywords, only pattern matching, and uniform function calls. The complete language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) defines all features and behavior.

## Core Design Principles (from LANGUAGE_SPEC.zen lines 1-14)

- **NO KEYWORDS**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)  
- **Pattern matching with `?` operator**: Replaces ALL control flow
- **UFC (Uniform Function Call)**: Any function can be called as method
- **Allocators determine sync/async**: No function coloring
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **NO NULL/NIL**: Only `Option<T>` with `Some(T)` and `None`
- **No unions, no tuples**: Only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type)
- **Error propagation**: `.raise()` not exceptions
- **Loops**: `loop()` infinite, `.loop()` collections, `(0..10)` ranges
- **Traits**: `.implements()` and `.requires()` from `@std.meta`
- **Compile-time metaprogramming**: Full AST access

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
# Run spec alignment test
./target/release/zen tests/zen_test_spec_final_aligned.zen

# Test all working features
./target/release/zen tests/zen_test_spec_aligned_working.zen
```

## Implementation Status

### ✅ Fully Implemented (Working)

- **No keywords philosophy** - Pattern matching with `?` replaces all control flow
- **Variable declarations** - All forms: forward, immutable, mutable (lines 298-306)
- **@std imports** - Import system with destructuring (lines 92-107)
- **Pattern matching** - Boolean and full patterns (lines 352-361)
- **Structs** - Definition and instantiation (lines 117-120)
- **Enums** - Sum types with pattern matching (line 165)
- **Loops** - Range `(0..N).loop()` and infinite `loop {}` (lines 432-460)
- **@this.defer** - Cleanup at scope end (line 217)
- **String interpolation** - `"Value: ${expr}"`
- **Functions** - Full support with parameters and return types
- **Option<T>** - Some/None for no-null safety (lines 109-110)
- **Result<T, E>** - Ok/Err for error handling (lines 112-113)

### 🚧 In Development

- **UFC (Uniform Function Call)** - Call any function as method
- **Traits** - `.implements()` and `.requires()` (lines 136-168)
- **Error propagation** - `.raise()` mechanism (lines 206-211)
- **Pointer types** - `Ptr<>`, `MutPtr<>`, `RawPtr<>` (lines 363-372)
- **Dynamic vectors** - `DynVec<T>` with allocators (lines 316-350)
- **Concurrency** - Actor, Channel, Mutex (lines 397-430)
- **Metaprogramming** - `@meta.comptime` (lines 243-282)
- **Inline C/LLVM** - Low-level control (lines 285-294)
- **SIMD operations** - Vector operations (lines 291-294)

## Language Features

All examples below are from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) and are currently working.

### Variable Declarations (LANGUAGE_SPEC.zen lines 298-306)

```zen
// Forward declarations and assignments
x: i32              // forward declaration (must be in same scope)
x = 10              
y = 10              // Immutable assignment
z: i32 = 20         // Immutable assignment with type
w:: i32             // mutable forward declaration (must be in same scope)
w = 20              
v ::= 30            // Mutable assignment
u:: i32 = 40        // mutable assignment with type
```

### Imports (LANGUAGE_SPEC.zen lines 92-107)

```zen
// Only @std and @this are special
{ io, maths } = @std
{ String, StringBuilder } = @std
{ Vec, DynVec } = @std
{ Actor, Channel, Mutex, AtomicU32 } = @std

// Module imports
sdl2 = @std.import("sdl2")
ecs = @std.import("ecs")
```

### Pattern Matching with ? (LANGUAGE_SPEC.zen lines 352-361)

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

### Option Type - No Null (LANGUAGE_SPEC.zen lines 109-110, 462-473)

```zen
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

### Result Type for Error Handling (LANGUAGE_SPEC.zen lines 113-114, 199-211)

```zen
Result<T, E>: Ok(T) | Err(E)

// Parse with Result
parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid radius") }
}

// Error propagation with .raise()
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // If Err, returns early with that error
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Structs (LANGUAGE_SPEC.zen lines 117-163)

```zen
// Simple struct
Point: {
    x:: f64,  // mutable field with ::
    y:: f64 = 0  // with default value
}

// Struct with methods via trait implementation
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

### Enums (LANGUAGE_SPEC.zen lines 165-182)

```zen
// Enum type (sum type)
Shape: Circle | Rectangle
// Enforce all Shape variants must implement Geometric  
Shape.requires(Geometric)

// Enum with UFC overloading
GameEntity: Player | Enemy | Powerup

// Overload functions for each variant
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }
```

### Loops and Ranges (LANGUAGE_SPEC.zen lines 432-460)

```zen
// Range iterations
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration with UFC
shapes.loop((shape) {
    total_area = total_area + shape.area()
})

// Loop with index
shapes.loop((shape, i) {
    io.println("Shape ${i}: ${shape.area()}")
})

// Infinite loop
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { io.println("Count: ${counter}") }
})
```

### Pointer Types (LANGUAGE_SPEC.zen lines 363-372)

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

### Dynamic Vectors (LANGUAGE_SPEC.zen lines 316-350, 376-385)

```zen
// Mixed type vector - can hold multiple variant types!
entities = DynVec<GameEntity.Player, GameEntity.Enemy>(sync_alloc)
@this.defer(entities.deinit())

entities.push(GameEntity.Player)
entities.push(GameEntity.Enemy)

// Loop over mixed types with pattern matching
entities.loop((entity) {
    entity ?
        | Player { 
            io.println("Player health: ${entity.get_health()}")
        }
        | Enemy { 
            io.println("Enemy health: ${entity.get_health()}")
        }
})
```

### Concurrency (LANGUAGE_SPEC.zen lines 397-430)

```zen
// Channel communication
message_chan = Channel<string>(10)  // Buffered channel
@this.defer(message_chan.close())

// Spawn actor to send messages
sender = Actor(() {
    (0..5).loop((i) {
        message_chan.send("Message ${i}")
    })
}).spawn()

// Mutex for shared state
counter_mutex = Mutex<u32>(0)
@this.defer(counter_mutex.deinit())

counter_mutex.lock() ?
    | Ok(val) {
        val = val + 1
        counter_mutex.unlock()
    }
    | Err(e) { io.println("Lock failed: ${e}") }

// Atomic operations
atomic_counter = AtomicU32(0)
atomic_counter.fetch_add(1)
```

### Metaprogramming (LANGUAGE_SPEC.zen lines 243-282)

```zen
// AST reflection
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

### Multisync Functions (LANGUAGE_SPEC.zen lines 215-224)

```zen
// Function that's sync or async based on allocator!
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

### Inline C/LLVM (LANGUAGE_SPEC.zen lines 285-294)

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

## Implementation Status (Aligned with LANGUAGE_SPEC.zen)

✅ **Working Features**:
- **Core Philosophy (lines 1-14)**: No keywords, only `?` pattern matching
- **Variable Declarations (lines 298-306)**: All forms - immutable `=`, mutable `::=`, forward declarations
- **Pattern Matching (lines 352-361)**: Full `?` operator support for control flow
- **Imports (lines 92-107)**: `@std` and module imports
- **Loops (lines 432-460)**: Range `(0..10).loop()` and infinite `loop()`
- **@this.defer (lines 217+)**: Scope cleanup
- **String Interpolation**: `"Value: ${expr}"`
- **Basic Types**: Structs, enums, Option type (no null)
- **Functions**: Full support with parameters and returns

🚧 **Partially Implemented**:
- **UFC (line 11)**: Core mechanism exists, method syntax parsing in progress
- **Result Type (lines 113-114, 199-211)**: Type defined, `.raise()` needs work
- **Collection.loop() (lines 441-451)**: Special case handled

❌ **Not Yet Implemented** (from LANGUAGE_SPEC.zen):
- **Traits (lines 127-143, 168)**: `.implements()` and `.requires()`
- **Pointer Types (lines 363-372)**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`  
- **Dynamic Vectors (lines 316-350)**: `DynVec<T>` with allocators
- **Metaprogramming (lines 243-282)**: `@meta.comptime`
- **Concurrency (lines 397-430)**: Actor, Channel, Mutex
- **Inline C/LLVM (lines 285-294)**: `inline.c()`
- **SIMD (lines 291-294)**: `simd.add()`

## Test Suite

All tests in `tests/` folder with `zen_` prefix (per requirements):

**Core Working Tests**:
- `tests/zen_test_spec_final_aligned.zen` - Validates LANGUAGE_SPEC.zen alignment
- `tests/zen_test_hello_world.zen` - Classic hello world
- `tests/zen_test_forward_declaration.zen` - Forward declarations (lines 298-306)
- `tests/zen_test_spec_aligned_working.zen` - All implemented features

**Feature-Specific Tests**:
- Pattern matching, loops, string interpolation, Option types
- All tests derive from LANGUAGE_SPEC.zen examples

## License

MIT