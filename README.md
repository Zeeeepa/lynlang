# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and allocator-based async without function coloring.

## Key Design Principles (from LANGUAGE_SPEC.zen)

```zen
// - No keywords: if/else/while/for/match/async/await/impl/trait/class/interface/null
// - Only two @ symbols: @std (standard library) and @this (current scope)
// - Pattern matching with `?` operator, no `match` or `switch`
// - UFC (Uniform Function Call) - any function can be called as method
// - Allocators determine sync/async behavior (no function coloring)
// - Explicit pointer types: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
// - No null/nil - only `Option<T>` with `.Some(T)` and `.None`
// - No unions, no tuples - only structs and enums
// - Assignment operators: `=` (immutable), `::=` (mutable), `:` (type definition)
// - Error propagation with `.raise()` not exceptions
// - Loops: `loop()` for infinite, `.loop()` for collections, ranges like `(0..10)`
// - Traits via `.implements()` and `.requires()` from `@std.meta`
// - Compile-time metaprogramming with full AST access
```

## Current Implementation Status

### ✅ WORKING FEATURES (~40% Complete)

#### 1. Variable Declarations (ALL 6 Forms) ✅
```zen
x: i32              // forward declaration
x = 10              // immutable assignment
y = 20              // immutable inference
z: i32 = 30         // immutable with type
w:: i32             // mutable forward declaration
w = 40              // mutable assignment
v ::= 50            // mutable inference  
u:: i32 = 60        // mutable with type
```

#### 2. UFC (Uniform Function Call) ✅
```zen
double = (n: i32) i32 { return n * 2 }
result = 5.double()        // UFC: transforms to double(5)
chained = 5.double().triple() // Chaining!
```

#### 3. Pattern Matching with `?` (No keywords!) ✅
```zen
// Boolean short form
is_ready ? { io.println("Go!") }

// Full pattern matching  
has_data ?
    | true { process_data() }
    | false { wait_for_data() }

// Option matching
maybe: Option<i32> = Some(42)
maybe ?
    | Some(n) { io.println("Value: ${n}") }
    | None { io.println("Empty") }
```

#### 4. @this.defer for Cleanup ✅
```zen
main = () void {
    io.println("Starting")
    @this.defer(io.println("Cleaning up!"))
    io.println("Working...")
    // Output: Starting, Working..., Cleaning up!
}
```

#### 5. Option & Result Types (No null!) ✅
```zen
Option<T>: Some(T) | None
Result<T, E>: Ok(T) | Err(E)

maybe: Option<i32> = Some(42)
nothing: Option<i32> = None
success: Result<i32, string> = Ok(100)
failure: Result<i32, string> = Err("error")
```

### ⚠️ PARTIALLY WORKING

#### 1. Traits via .implements() and .requires()
```zen
// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implementation
Circle.implements(Geometric, {
    area = (self) f64 {
        return 3.14159 * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * 3.14159 * self.radius
    },
})

// Require all Shape variants to implement Geometric
Shape.requires(Geometric)
```

#### 2. Error Propagation with .raise()
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // Returns early if Err
    contents = file.read_all().raise()
    config = json.parse(contents).raise()
    return Ok(config)
}
```

#### 6. Range Iteration ✅
```zen
// Range with .loop() - WORKS!
(0..10).loop((i) {
    io.println(i)
})

// Inclusive ranges  
(1..=5).loop((n) {
    process(n)
})
```

#### 7. Loops ✅
```zen
// Range iteration
(0..10).loop((i) {
    io.println("Count: ${i}")
})

// Step ranges
(0..100).step(10).loop((i) {
    io.println("Step: ${i}")  // 0, 10, 20, ...
})

// Infinite loop
loop(() {
    counter = counter + 1
    counter > 10 ? { break }
})
```

#### 8. Structs & Enums ✅
```zen
{ io } = @std              // Import syntax
io.println("Hello")         // Basic I/O
io.println(42)              // Multiple types
```

#### 9. Standard Library Basics ✅
```zen
Point: {
    x: f64,
    y: f64
}

Shape: Circle | Rectangle | Triangle

p = Point { x: 10.0, y: 20.0 }
io.println(p.x)  // Field access works
```

#### 3. String Interpolation
```zen
// Not yet fully implemented
name = "Zen"
io.println("Hello ${name}")  // Planned feature
```

### ❌ NOT YET IMPLEMENTED

From LANGUAGE_SPEC.zen (Priority items):

#### 1. Advanced Collections
```zen
Vec<T, SIZE>        // Static vector
DynVec<T>           // Dynamic vector
HashMap<K, V>       // Hash map
```

#### 2. Allocator System
```zen
// Determines sync/async behavior
sync_alloc = GPA.init()
async_alloc = AsyncPool.init()
```

#### 3. Pointer Types
```zen
ptr: Ptr<Circle> = circle.ref()
mut_ptr: MutPtr<Circle> = circle.mut_ref()
```

#### 4. Actor Model & Concurrency
```zen
actor = Actor(() { ... })
channel = Channel<T>(10)
mutex = Mutex<T>(initial)
```

#### 5. Compile-time Metaprogramming
```zen
@meta.comptime(() { ... })
ast = reflect.ast(Type)
```

#### 6. Module System
```zen
module.exports = { ... }
imported = module.import("name")
```

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/zen
cd zen

# Build the compiler
cargo build --release

# Run a Zen program
cargo run --bin zen -- examples/hello_world.zen
```

### Hello World

```zen
{ io } = @std

main = () void {
    io.println("Hello, World!")
}
```

### Key Features Example

```zen
{ io } = @std

// No keywords! Only pattern matching
main = () void {
    is_ready = true
    is_ready ? { io.println("Starting!") }
    
    // UFC - any function as method
    double = (n: i32) i32 { return n * 2 }
    result = 5.double()
    io.println(result)
    
    // No null - only Option types
    maybe: Option<i32> = Some(42)
    maybe ?
        | Some(value) { io.println(value) }
        | None { io.println("Empty") }
    
    // Range iteration
    (0..3).loop((i) {
        io.println(i)
    })
}
```

## Examples

See the [`examples/`](./examples) directory for working code samples.

## Testing

```bash
# Run all tests
./test_all.sh

# Run specific test
cargo run --bin zen -- tests/zen_test_ufc_working.zen

# Working feature tests
- tests/zen_test_ufc_working.zen         # UFC demonstration
- tests/zen_test_range_iteration.zen     # Range iteration
- tests/zen_test_spec_basic_check.zen    # Core features
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen     # THE source of truth
├── src/                  # Rust compiler implementation
│   ├── parser/           # Zen parser
│   ├── codegen/          # LLVM code generation
│   ├── typechecker/      # Type system
│   └── stdlib/           # Standard library
├── tests/                # Test suite (prefix with zen_)
├── examples/             # Example programs
└── stdlib/               # Zen standard library
```

## Contributing

All contributions must align with [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen).

## Implementation Progress

- **Core Language**: ~60% complete
- **Type System**: ~40% complete
- **Standard Library**: ~20% complete
- **Advanced Features**: ~5% complete

**Overall**: ~40% of LANGUAGE_SPEC.zen implemented

## License

MIT

## Contact

For issues or questions, see the [GitHub issues](https://github.com/your-org/zen/issues) page.