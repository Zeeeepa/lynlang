# Zen Programming Language

A revolutionary systems programming language based on the principles from LANGUAGE_SPEC.zen: zero keywords, pattern-first design, and colorless async through allocators.

## Key Design Principles

### 🚫 No Keywords
Zen has **zero keywords**. No `if/else/while/for/match/async/await/impl/trait/class/interface/null`. Everything is achieved through:
- Pattern matching with `?` operator
- Only two special symbols: `@std` (standard library) and `@this` (current scope)
- Method calls and UFC (Uniform Function Call)
- Loops via `loop()` function and `.loop()` method on collections

### 📋 Pattern Matching First
All control flow uses the `?` operator:
```zen
// Boolean patterns
is_ready ? { start_game() }

// Multi-branch patterns
has_data ?
    | true { process_data() }
    | false { io.println("Waiting for data...") }

// Option type matching
maybe_value ?
    | Some(val) { io.println("Value: ${val}") }
    | None { io.println("No value") }

// Enum matching
shape ?
    | Circle { io.println("Circle area: ${shape.area()}") }
    | Rectangle { io.println("Rectangle area: ${shape.area()}") }
```

### 🔄 Assignment Operators
Three distinct operators for clarity:
- `=` - Immutable binding
- `::=` - Mutable assignment with type inference
- `:` - Type annotation
- `::` - Mutable type annotation

```zen
// From LANGUAGE_SPEC.zen lines 298-306
x: i32              // Forward declaration (immutable)
x = 10              // Assignment
y = 10              // Immutable assignment
z: i32 = 20         // Immutable with explicit type
w :: i32            // Mutable forward declaration
w = 20              // Assignment to mutable
v ::= 30            // Mutable assignment
u :: i32 = 40       // Mutable with explicit type
```

### 🎯 Explicit Pointer Types
No `*` or `&` operators. Instead, explicit types (Ptr<>, MutPtr<>, RawPtr<>):
```zen
// From LANGUAGE_SPEC.zen lines 364-372
circle = Circle { center: Point { x: 100, y: 100 }, radius: 50 }
circle_ptr: Ptr<Circle> = circle.ref()          // Immutable pointer
circle_mut: MutPtr<Circle> = circle.mut_ref()   // Mutable pointer

io.println("Circle area: ${circle_ptr.val.area()}")  // .val to dereference
circle_mut.val.radius = 75                           // Modify through pointer
io.println("New area: ${circle_mut.val.area()}")
io.println("Address: ${circle_ptr.addr}")            // Get pointer address
```

### ❌ No Null - Only Option Types
```zen
// From LANGUAGE_SPEC.zen lines 110, 462-473
Option<T>: Some(T) | None

maybe_radius: Option<f64> = Some(5.5)
maybe_radius ?
    | Some(r) {
        circle = Circle {
            center: Point { x: 100.0, y: 100.0 },
            radius: r
        }
        io.println("Created circle with area: ${circle.area()}")
    }
    | None {
        io.println("No radius provided")
    }
```

### 🚀 Colorless Async via Allocators
Functions aren't colored async/sync. The allocator determines behavior:
```zen
// From LANGUAGE_SPEC.zen lines 213-224
fetch_game_data = (url: string, alloc: Allocator) Result<Data, Error> {
    client = HttpClient(alloc)
    @this.defer(client.deinit())
    
    // This blocks or doesn't based on allocator!
    response = client.get(url)
    response ?
        | Ok(data) { return Ok(parse_data(data)) }
        | Err(e) { return Err(e) }
}

// Synchronous execution
sync_alloc = GPA.init()
data = fetch_game_data("api.com", sync_alloc)  // Blocks

// Asynchronous execution  
async_alloc = AsyncPool.init()
data = fetch_game_data("api.com", async_alloc)  // Non-blocking
```

### 📦 Container Types
```zen
// From LANGUAGE_SPEC.zen lines 374-385
// Static sized vector
shapes = Vec<Shape, 100>()
shapes.push(Circle { center: Point { x: 0, y: 0 }, radius: 10 })

// Dynamic vector with allocator
dynamic_shapes = DynVec<Shape>(sync_alloc.allocator())
@this.defer(dynamic_shapes.deinit())

// Mixed type vector - can hold multiple variant types!
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

### 🔧 UFC (Uniform Function Call)
Any function can be called as a method on its first parameter:
```zen
// Traditional function call
distance = calculate_distance(point1, point2)

// UFC style - same function called as method
distance = point1.calculate_distance(point2)

// Works with any function
doubled = multiply(value, 2)     // Traditional
doubled = value.multiply(2)      // UFC style
```

### 🔄 Loops
No `while` or `for` keywords. Use `loop()` and range operators:
```zen
// From LANGUAGE_SPEC.zen lines 431-459
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Collection iteration with UFC
shapes.loop((shape) {
    io.println("Area: ${shape.area()}")
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

### 💫 String Interpolation
Built-in string interpolation with `${}` syntax:
```zen
name = "World"
count = 42
message = "Hello ${name}! Count is ${count}"
io.println(message)  // "Hello World! Count is 42"

// With expressions
x = 10
y = 20
result = "The sum of ${x} and ${y} is ${x + y}"
io.println(result)  // "The sum of 10 and 20 is 30"
```

### 🔀 Error Propagation
Use `.raise()` instead of exceptions or `?` operator from Rust:
```zen
// From LANGUAGE_SPEC.zen lines 205-211
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()      // If Err, returns early with that error
    contents = file.read_all().raise()  // Automatic error propagation
    config = json.parse(contents).raise()
    return Ok(config)
}
```

### 🧹 Defer Mechanism
Cleanup with `@this.defer()`:
```zen
// From LANGUAGE_SPEC.zen line 217, 309-314
main = () void {
    sync_alloc = GPA.init()
    @this.defer(sync_alloc.deinit())  // Cleanup at scope exit
    
    file = File.open("data.txt")
    @this.defer(file.close())         // Ensures file is closed
    
    // ... use file and allocator ...
}  // Deferred operations execute here in reverse order
```

## Implementation Status

### ✅ Working Features
- Variable declarations (all forms: `=`, `::=`, `:`, `::`)
- Pattern matching with `?` operator
- Enums and Option types
- UFC (Uniform Function Call)
- Loops (`loop()`, ranges with `.loop()`)
- Structs with mutable fields
- String interpolation
- Error propagation with `.raise()`
- Defer mechanism with `@this.defer()`
- Basic pointer operations (`.ref()`, `.mut_ref()`, `.val`, `.addr`)
- Container types (Vec, DynVec) - partial

### 🚧 In Progress
- Trait system (`.implements()`, `.requires()`)
- Allocators (GPA, AsyncPool)
- Colorless async/sync
- Metaprogramming and comptime
- Full generic type system

### ❌ Not Yet Implemented
- Actor model and channels
- Mutex and atomic types
- AST reflection
- Inline C/LLVM
- SIMD operations
- Module system (`module.exports`, `module.import()`)
- Build system integration
- FFI (Foreign Function Interface)

## Quick Start

### Building
```bash
cargo build --release
```

### Running Examples
```bash
# Run comprehensive language spec showcase
./target/release/zen tests/zen_test_language_spec_showcase.zen

# Test string interpolation
./target/release/zen tests/zen_test_string_interp_demo.zen

# Test all working features
./target/release/zen tests/zen_test_spec_complete.zen
```

## Language Specification

The complete language specification is in `LANGUAGE_SPEC.zen`, which serves as both documentation and a compilable example of the language's features.

## Project Structure

```
zenlang/
├── src/
│   ├── parser/         # Parser implementation
│   ├── codegen/        # LLVM code generation
│   │   └── llvm/       # LLVM backend
│   ├── typechecker/    # Type system
│   ├── ast/            # Abstract syntax tree
│   └── lexer.rs        # Lexical analysis
├── tests/              # Test files (zen_test_*.zen)
├── LANGUAGE_SPEC.zen   # Complete language specification
└── IMPLEMENTATION_STATUS.md  # Detailed implementation progress
```

## Philosophy

Zen represents a radical rethinking of systems programming languages:

1. **No Keywords**: Everything is data and functions. Control flow emerges from pattern matching.

2. **Pattern-First**: The `?` operator is the universal control flow mechanism.

3. **No Function Coloring**: Async/sync is determined by allocators, not function signatures.

4. **Explicit Over Implicit**: Pointer types are explicit. No hidden allocations.

5. **No Null**: Option types eliminate null pointer exceptions at compile time.

6. **Simplicity**: Only two special symbols (`@std`, `@this`). Everything else is regular syntax.

## Contributing

This is an experimental language exploring new paradigms in systems programming. Contributions are welcome! Check `IMPLEMENTATION_STATUS.md` for areas that need work.

## License

[License information to be added]