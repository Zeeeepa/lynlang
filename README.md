# Zen Programming Language

A modern systems programming language with a unique design philosophy based on minimalism and expressiveness. The language specification in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the source of truth.

## 🎯 Core Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

- **No keywords**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)  
- **Pattern matching with `?` operator**: No `match` or `switch` keywords
- **UFC (Uniform Function Call)**: Any function can be called as a method
- **Allocators determine sync/async**: No function coloring problem
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **No null/nil**: Only `Option<T>` with `.Some(T)` and `.None`
- **No unions, no tuples**: Only structs and enums
- **Assignment operators**: `=` (immutable), `::=` (mutable), `:` (type definition)
- **Error propagation**: `.raise()` for early returns, not exceptions
- **Loops**: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
- **Traits**: Via `.implements()` and `.requires()` from `@std.meta`
- **Compile-time metaprogramming**: Full AST access

## 🚀 Quick Start

### Building the Compiler
```bash
cargo build --release
```

### Running Zen Programs
```bash
# Run directly
./target/release/zen hello.zen

# Compile to executable
./target/release/zen hello.zen -o hello
./hello

# Start REPL
./target/release/zen
```

## ✨ Language Features (from LANGUAGE_SPEC.zen)

### Variables (lines 298-306)
All variable declaration forms from the spec:
```zen
x: i32              // Forward declaration  
x = 10              // Immutable assignment
y = 10              // Immutable assignment with inference
z: i32 = 20         // Immutable assignment with type
w:: i32             // Mutable forward declaration
w = 20              // Assignment to mutable
v ::= 30            // Mutable assignment with inference
u:: i32 = 40        // Mutable assignment with type
```

### Pattern Matching (lines 352-361)
Boolean pattern matching without keywords:
```zen
// Short form for single branch
is_ready ? {
    io.println("Starting game!")
}

// Full pattern match for if-else
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }
```

### No Null - Option Types (lines 109-110, 462-473)
```zen
Option<T>: Some(T) | None

maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle { radius: r }
        io.println("Created circle with area: ${circle.area()}")
    }
    | None {
        io.println("No radius provided")
    }
```

### Result Type (lines 113-114)
Error handling without exceptions:
```zen
Result<T, E>: Ok(T) | Err(E)

parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid radius") }
}
```

### Structs (lines 117-120)
With mutable fields and defaults:
```zen
Point: {
    x:: f64,        // Mutable field
    y:: f64 = 0     // With default value
}

Circle: {
    center: Point,
    radius: f64,
}
```

### Enums (line 165)
Sum types without union:
```zen
Shape: Circle | Rectangle

// Overloaded functions per variant (lines 173-180)
GameEntity: Player | Enemy | Powerup

get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }
```

### Loops and Ranges (lines 432-460)
Collection and range iteration:
```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges  
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration
dynamic_shapes.loop((shape) {
    total_area = total_area + shape.area()
})

// With index
dynamic_shapes.loop((shape, i) {
    io.println("Shape ${i}: ${shape.area()}")
})

// Infinite loop
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ?
        | true { break }
        | false { io.println("Count: ${counter}") }
})
```

### Error Propagation (lines 206-211)
Early returns with `.raise()`:
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // If Err, returns early
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Defer Statements (lines 217, 309, 314, etc.)
Resource cleanup:
```zen
sync_alloc = GPA.init()
@this.defer(sync_alloc.deinit())

file = File.open("data.txt")  
@this.defer(file.close())
// file.close() automatically called at scope end
```

### Pointer Types (lines 364-372)
Explicit pointer management:
```zen
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75
io.println("New area: ${circle_mut.val.area()}")
io.println("Address: ${circle_ptr.addr}")
```

### Dynamic Vectors (lines 317-335, 376-385)
With allocators and mixed types:
```zen
// Dynamic vector with allocator
dynamic_shapes = DynVec<Shape>(sync_alloc.allocator())
@this.defer(dynamic_shapes.deinit())

// Mixed type vector - can hold multiple variant types!
entities = DynVec<GameEntity.Player, GameEntity.Enemy>(sync_alloc)
@this.defer(entities.deinit())

entities.push(GameEntity.Player)
entities.push(GameEntity.Enemy)

// Pattern match on mixed types
entities.loop((entity) {
    entity ?
        | Player { io.println("Player health: ${entity.get_health()}") }
        | Enemy { io.println("Enemy health: ${entity.get_health()}") }
})
```

### UFC (Uniform Function Call)
Any function becomes a method:
```zen
// Traditional vs UFC
result1 = double(5)       // Traditional call
result2 = 5.double()      // UFC: becomes double(5)

// UFC with collections
shapes.map(calculate_area)      // Traditional
calculate_area.map(shapes)      // UFC style
```

### String Building (lines 386-394)
```zen
sb = StringBuilder(sync_alloc)
@this.defer(sb.deinit())
sb.append("Hello")
  .append(" ")
  .append("World")
  .append_line("!")
built_string = sb.build()
```

### Traits (lines 124-163)
With `.implements()` and `.requires()`:
```zen
// Trait definition
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

// Enforce trait requirements
Shape.requires(Geometric)
```

### Multisync Functions (lines 213-224)
Sync/async based on allocator:
```zen
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

### Actors and Channels (lines 226-240, 399-412)
Concurrency primitives:
```zen
// Actor for lazy/streaming iteration
create_fibonacci = () Actor {
    outer = 100  // Auto-captured
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

// Channels
message_chan = Channel<string>(10)  // Buffered
@this.defer(message_chan.close())

// Spawn actor to send messages
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

### Mutex and Atomics (lines 415-429)
Thread-safe primitives:
```zen
// Mutex
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
current = atomic_counter.load()
```

### Compile-time Metaprogramming (lines 243-282)
AST manipulation:
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

### Inline C/LLVM (lines 285-289)
Low-level control:
```zen
fast_memcpy = (dst: RawPtr<u8>, src: RawPtr<u8>, len: usize) void {
    inline.c("""
        memcpy(${dst.addr}, ${src.addr}, ${len});
    """)
}
```

### SIMD Operations (lines 291-294)
Vector operations:
```zen
vector_add = (a: Vec<f32, 8>, b: Vec<f32, 8>) Vec<f32, 8> {
    return simd.add(a, b)
}
```

## 📚 Standard Library

Access via `@std`:
```zen
{ io, maths } = @std                          // Modules
{ String, StringBuilder } = @std              // Types
{ requires, implements, reflect, meta } = @std // Metaprogramming
{ GPA, AsyncPool, Allocator} = @std          // Memory management
{ Vec, DynVec} = @std                         // Collections
{ Actor, Channel, Mutex, AtomicU32 } = @std  // Concurrency
```

## 🏗️ Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen  # Language specification (source of truth)
├── src/               # Rust compiler implementation
│   ├── ast/           # Abstract syntax tree
│   ├── codegen/       # LLVM code generation
│   ├── lexer.rs       # Tokenization
│   ├── parser/        # Parsing
│   └── typechecker/   # Type checking
├── stdlib/            # Standard library (Zen)
├── tests/             # Test suite (prefix: zen_)
└── examples/          # Example programs
```

## 🧪 Testing

All tests follow the `zen_` prefix convention and are in the `tests/` folder:

```bash
# Run compiler tests
cargo test

# Run Zen test files
./target/release/zen tests/zen_test_spec_aligned.zen
./target/release/zen tests/zen_test_comprehensive.zen
```

## 📈 Implementation Status

### ✅ Working Features
Based on actual testing against LANGUAGE_SPEC.zen:

- **Variable declarations** - All 8 forms from spec (lines 298-306)
- **Pattern matching** - Boolean with `?` operator (lines 352-361) 
- **Structs** - With mutable fields and defaults (lines 117-120)
- **Enums** - Basic sum types (line 165)
- **Functions** - With parameters and return types
- **Loops** - Infinite `loop()` and range `(0..10).loop()` (lines 432-460)
- **String interpolation** - `${expr}` syntax
- **Basic arithmetic** - All operators
- **Comparisons** - All comparison operators
- **@std imports** - Basic module imports

### 🚧 Partially Working
- **Option/Result types** - Defined but pattern matching needs work
- **Error propagation** - `.raise()` syntax parsed
- **Defer statements** - `@this.defer()` parsed
- **UFC** - Basic implementation

### 📋 Not Yet Implemented
From LANGUAGE_SPEC.zen:

- **Step ranges** - `(0..100).step(10)` (line 437)
- **Traits** - `.implements()` and `.requires()` (lines 136-162)
- **Pointer types** - `Ptr<>`, `MutPtr<>`, `RawPtr<>` (lines 364-372)
- **DynVec with allocators** - Mixed type vectors (lines 317-335)
- **StringBuilder** - String building API (lines 386-394)
- **Actors/Channels** - Concurrency primitives (lines 226-240, 399-412)
- **Mutex/Atomics** - Thread safety (lines 415-429)
- **Compile-time metaprogramming** - AST manipulation (lines 243-282)
- **Inline C/LLVM** - Low-level code (lines 285-289)
- **SIMD operations** - Vector operations (lines 291-294)
- **Module exports** - `module.exports` (lines 492-502)

## 🎯 Design Philosophy

Zen eliminates traditional programming keywords in favor of operators and patterns:

| Traditional | Zen Equivalent |
|------------|----------------|
| `if/else` | Pattern matching with `?` |
| `while/for` | `loop()` and `.loop()` |
| `null` | `Option.None` |
| `throw/catch` | `Result` and `.raise()` |
| `async/await` | Allocator-based |
| `class/interface` | Structs and traits |
| `*ptr` | `Ptr<T>` |
| `&ref` | `.ref()` |

## 📄 License

This project implements the Zen language specification from LANGUAGE_SPEC.zen.