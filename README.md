# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and powerful metaprogramming.

> *"No keywords. Pure expression."*

## Implementation Progress: ~35% Complete

Core language features working. See [`IMPLEMENTATION_STATUS_CURRENT.md`](./IMPLEMENTATION_STATUS_CURRENT.md) for details.

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a working example
./target/release/zen tests/zen_test_working_baseline.zen

# Test spec features
./target/release/zen tests/zen_test_spec_main_from_language_spec.zen
```

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

## Language Features by Example

All examples from [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen)

### Variables - No Keywords!

```zen
// From LANGUAGE_SPEC.zen lines 299-306
x: i32               // Forward declaration
x = 10               // Assignment
y = 10               // Immutable assignment (inferred)
z : i32 = 20         // Immutable with type
w :: i32             // Mutable forward declaration
w = 20               // Assignment
v ::= 30             // Mutable assignment
u :: i32 = 40        // Mutable with type
```

### Pattern Matching - Replaces All Conditionals

```zen
// From LANGUAGE_SPEC.zen lines 352-361
is_ready = true
is_ready ? { 
    io.println("Starting game!") 
}

has_data = false
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }
```

### Loops and Ranges

```zen
// From LANGUAGE_SPEC.zen lines 431-459
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges (lines 436-439) - NOT YET IMPLEMENTED
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Infinite loop with break
counter ::= 0
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

### No Null - Only Option Types

```zen
// From LANGUAGE_SPEC.zen lines 110, 462-473
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

### Error Handling with Result

```zen
// From LANGUAGE_SPEC.zen lines 113, 199-211
Result<T, E>: Ok(T) | Err(E)

parse_radius = (s: string) Result<f64, string> {
    s.to_f64() ?
        | Some(val) { return Ok(val) }
        | None { return Err("Invalid radius") }
}

// Error propagation with .raise() - NOT YET IMPLEMENTED
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // If Err, returns early
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### Structs and Enums

```zen
// From LANGUAGE_SPEC.zen lines 117-172
Point: {
    x:: f64,  // mutable field
    y:: f64 = 0  // with default
}

Circle: {
    center: Point,
    radius: f64,
}

Rectangle: {
    top_left: Point,
    bottom_right: Point,
}

// Enum type (sum type)
Shape: Circle | Rectangle
```

### Traits - No `impl` Keyword!

```zen
// From LANGUAGE_SPEC.zen lines 125-163
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
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

// Enforce all Shape variants must implement Geometric
Shape.requires(Geometric)
```

### UFC Enum Overloading

```zen
// From LANGUAGE_SPEC.zen lines 172-182 - NOT YET IMPLEMENTED
GameEntity: Player | Enemy | Powerup

// Overload functions for each variant
get_health = (e: GameEntity.Player) u32 { return 100 }
get_health = (e: GameEntity.Enemy) u32 { return 50 }
get_health = (e: GameEntity.Powerup) u32 { return 0 }
```

### Generics with Constraints

```zen
// From LANGUAGE_SPEC.zen lines 184-196 - NOT YET IMPLEMENTED
print_area<T: Geometric>(shape: T) void {
    io.println("Area: ${shape.area()}")
}

Container<T: Geometric + Serializable>: {
    items: DynVec<T>,
    add: (item: T) void,
    total_area: () f64,
}
```

### Explicit Pointers - No `*` or `&`

```zen
// From LANGUAGE_SPEC.zen lines 364-371 - NOT YET IMPLEMENTED
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()
circle_mut: MutPtr<Circle> = circle.mut_ref()

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75
io.println("Address: ${circle_ptr.addr}")
```

### Concurrency - No `async/await`!

```zen
// From LANGUAGE_SPEC.zen lines 214-241, 396-429 - NOT YET IMPLEMENTED
// Allocators determine sync/async behavior!
sync_alloc = GPA.init()
async_alloc = AsyncPool.init()

// This blocks or doesn't based on allocator!
fetch_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)
    response = client.get(url)  // Blocks with sync_alloc, async with async_alloc
    return Ok(parse(response))
}

// Actors for streaming
create_fibonacci = () Actor {
    return Actor((receiver) {
        a ::= 0
        b ::= 1
        loop(() {
            receiver.send(a)
            temp = a + b
            a = b
            b = temp
        })
    })
}

// Channels
message_chan = Channel<string>(10)
message_chan.send("Hello")
message_chan.receive() ?
    | Some(msg) { io.println("Got: ${msg}") }
    | None { io.println("Channel closed") }
```

### Metaprogramming - Full AST Access

```zen
// From LANGUAGE_SPEC.zen lines 244-294 - NOT YET IMPLEMENTED
// Runtime reflection
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

## Current Implementation Status

### ✅ Working Features (35% Complete)
- Zero keywords design
- Pattern matching with `?`
- Variable declarations (6 of 8 forms)
- Basic loops and ranges
- String interpolation
- @std imports (io, math)
- @this.defer() for RAII
- UFC (Uniform Function Call)
- Basic arithmetic

### 🚧 Partially Working
- Structs (parse but need field access)
- Enums (parse but incomplete pattern matching)
- Traits (parse but methods not callable)

### ❌ Not Yet Implemented
- Option<T> and Result<T,E> types
- .raise() error propagation
- Step ranges
- Generics
- Pointers (Ptr<>, MutPtr<>, RawPtr<>)
- Collections (Vec, DynVec, StringBuilder)
- Concurrency (Actors, Channels, Mutex)
- Metaprogramming
- Module system
- Build system

## Contributing

See [`IMPLEMENTATION_STATUS_CURRENT.md`](./IMPLEMENTATION_STATUS_CURRENT.md) for detailed implementation status and next priorities.

## Testing

All tests should be prefixed with `zen_` and placed in the `tests/` folder:

```bash
# Run specific test
./target/release/zen tests/zen_test_working_baseline.zen

# Test pattern matching
./target/release/zen tests/zen_test_spec_feature_check.zen
```

## License

MIT