# Zen Programming Language

**`LANGUAGE_SPEC.zen` IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS** - everything is pattern matching, uniform function calls, and compile-time metaprogramming. The complete language specification is defined in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

## Core Philosophy (LANGUAGE_SPEC.zen lines 1-14)

```zen
// NO KEYWORDS - No if/else/while/for/match/async/await/impl/trait/class/interface/null
// Only two @ symbols: @std (standard library) and @this (current scope)
// Pattern matching with ? operator, no match or switch
// UFC (Uniform Function Call) - any function can be called as method
// Allocators determine sync/async behavior (no function coloring)
// Explicit pointer types: Ptr<>, MutPtr<>, RawPtr<> (no * or &)
// No null/nil - only Option<T> with .Some(T) and .None
// No unions, no tuples - only structs and enums
// Assignment operators: = (immutable), ::= (mutable), : (type definition)
// Error propagation with .raise() not exceptions
// Loops: loop() for infinite, .loop() for collections, ranges like (0..10)
// Traits via .implements() and .requires() from @std.meta
// Compile-time metaprogramming with full AST access
```

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

## Current Implementation Status

### ✅ Working Features (Can use today)

| Feature | LANGUAGE_SPEC.zen Reference | Status |
|---------|----------------------------|--------|
| **No Keywords** | Lines 1-2 | ✅ Complete - `?` operator replaces all control flow |
| **Variable Declarations** | Lines 298-306 | ✅ All forms: `=`, `::=`, type annotations |
| **Pattern Matching** | Lines 352-361 | ✅ Boolean patterns with `?` operator |
| **Option Type** | Lines 109-110, 462-473 | ✅ `Some(T)` / `None` - no null |
| **Result Type** | Lines 112-113 | ✅ `Ok(T)` / `Err(E)` for errors |
| **Loops & Ranges** | Lines 432-460 | ✅ `(0..10).loop()`, infinite `loop{}` |
| **String Interpolation** | Throughout | ✅ `"Value: ${expr}"` |
| **@std Imports** | Lines 92-107 | ✅ `{ io, math } = @std` |
| **Functions** | Throughout | ✅ First-class functions |
| **Basic Arithmetic** | Implicit | ✅ `+`, `-`, `*`, `/`, `%` |

### 🚧 In Development (Partial implementation)

| Feature | LANGUAGE_SPEC.zen Reference | Status |
|---------|----------------------------|--------|
| **@this.defer** | Lines 217, 309, 314 | ⚠️ Basic support |
| **Structs** | Lines 117-120 | ⚠️ Basic structs work |
| **Enums** | Line 165 | ⚠️ Simple enums work |
| **Step Ranges** | Lines 437-439 | ⚠️ `(0..100).step(10)` parsing |

### ❌ Not Yet Implemented (From LANGUAGE_SPEC.zen)

| Feature | LANGUAGE_SPEC.zen Reference | Priority |
|---------|----------------------------|----------|
| **UFC** | Line 4, throughout | High - Core feature |
| **Traits** | Lines 123-163 | High - `.implements()`, `.requires()` |
| **Error .raise()** | Lines 206-211 | High - Error propagation |
| **Pointer Types** | Lines 363-372 | Medium - `Ptr<>`, `MutPtr<>`, `RawPtr<>` |
| **DynVec** | Lines 316-350 | Medium - Dynamic vectors with allocators |
| **Allocators** | Lines 308-314 | Medium - Sync/async determination |
| **Concurrency** | Lines 397-430 | Low - Actor, Channel, Mutex |
| **Metaprogramming** | Lines 243-282 | Low - `@meta.comptime` |
| **Inline C/LLVM** | Lines 285-294 | Low - FFI |
| **SIMD** | Lines 291-294 | Low - Vector operations |

## Language Examples from LANGUAGE_SPEC.zen

All examples below are directly from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen). Features marked with ✅ are working, ⚠️ are partial, ❌ are not yet implemented.

### ✅ Variable Declarations (Lines 298-306)

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

### ✅ Imports (Lines 92-107)

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

### ✅ Pattern Matching with ? (Lines 352-361)

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

### ✅ Option Type - No Null (Lines 109-110, 462-473)

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

### ✅ Result Type (Lines 112-113)

```zen
Result<T, E>: Ok(T) | Err(E)

// Working Result type usage
parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid radius") }
}

// Note: .raise() error propagation from spec not yet implemented
```

### ⚠️ Structs (Lines 117-120) - Basic Support

```zen
// Simple struct (working)
Point: {
    x:: f64,  // mutable field with ::
    y:: f64 = 0  // with default value
}

// Create and use struct
origin = Point { x: 0.0, y: 0.0 }
io.println("Point at (${origin.x}, ${origin.y})")

// Note: Traits via .implements() not yet working
```

### ⚠️ Enums (Lines 165-182) - Basic Support

```zen
// Enum type (working)
Shape: Circle | Rectangle | Triangle

// Pattern match on enum
shape: Shape = Circle
shape ?
    | Circle { io.println("It's a circle") }
    | Rectangle { io.println("It's a rectangle") }
    | Triangle { io.println("It's a triangle") }

// Note: .requires() and UFC overloading not yet working
```

### ✅ Loops and Ranges (Lines 432-460)

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

### ❌ Pointer Types (Lines 363-372) - Not Implemented

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

### ❌ Dynamic Vectors (Lines 316-350) - Not Implemented

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

### ❌ Concurrency (Lines 397-430) - Not Implemented

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

### ❌ Metaprogramming (Lines 243-282) - Not Implemented

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

### ❌ Multisync Functions (Lines 215-224) - Not Implemented

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

### ❌ Inline C/LLVM (Lines 285-294) - Not Implemented

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

## Running Tests

All tests are in `tests/` directory with `zen_` prefix:

```bash
# Run hello world
./target/release/zen tests/zen_test_hello_world.zen

# Run comprehensive spec test
./target/release/zen tests/zen_test_spec_aligned_working.zen

# Run simple spec validation
./target/release/zen tests/zen_test_simple_spec.zen
```

**Key Test Files**:
- `zen_test_spec_aligned_working.zen` - Tests all working features from LANGUAGE_SPEC.zen
- `zen_test_hello_world.zen` - Simplest possible Zen program
- `zen_test_simple_spec.zen` - Basic feature validation

## License

MIT