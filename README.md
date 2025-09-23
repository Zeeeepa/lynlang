# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source of truth**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching, UFC (Uniform Function Call), and no function coloring.

> **Current Status:** ~40% of LANGUAGE_SPEC.zen implemented. Core features working: structs, traits, pattern matching, ranges. Major gaps: Option/Result types, error propagation, pointers, collections, concurrency.

## Core Philosophy (from LANGUAGE_SPEC.zen)

```zen
// Zen Language - Key Design Principles:
// - No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`
// - Only two @ symbols: `@std` (standard library) and `@this` (current scope)
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

## Quick Start

```zen
{ io, math } = @std

main = () void {
    // No keywords! Only pattern matching
    is_ready = true
    is_ready ? { io.println("Starting!") }
    
    // UFC - any function can be called as method
    (0..5).loop((i) {
        io.println("Count: ${i}")
    })
    
    // No null - only Option types
    maybe: Option<i32> = Some(42)
    maybe ?
        | Some(v) { io.println("Value: ${v}") }
        | None { io.println("No value") }
}
```

## Implementation Status

### ✅ Core Features Working (40% Complete)

#### No Keywords Philosophy
- **Zero keywords** - All control flow via pattern matching
- **Pattern matching with `?`** - Boolean and enum patterns
- **UFC (Uniform Function Call)** - Any function can be called as method

#### Type System
- **Structs** - With fields and defaults ✅
- **Traits** - `.implements()` with method dispatch ✅
- **Enums** - Basic sum types (partial) ⚠️
- **Option<T>** - Parsed but value extraction buggy ⚠️
- **Result<T, E>** - Parsed but not fully working ⚠️

#### Variables & Assignment
Partial support for LANGUAGE_SPEC.zen forms:
```zen
x = 10          // Immutable assignment ✅
y ::= 20        // Inferred mutable ✅
x: i32 = 30     // Typed immutable ✅
u:: i32 = 40    // Typed mutable ✅
// Forward declarations not fully working:
x: i32          // Forward declaration ⚠️
w:: i32         // Mutable forward declaration ⚠️
```

#### Control Flow & Iteration
- **Pattern matching** - Boolean patterns with `?` ✅
- **Range loops** - `(0..10).loop()` ✅
- **String interpolation** - `"Value: ${expr}"` ✅
- **Infinite loops** - `loop(() { ... })` ⚠️
- **Break/continue** - Not implemented ❌
- **Range step** - `(0..10).step(2)` ❌

#### Imports & Modules
- **@std imports** - `{ io, math } = @std` ✅
- **Module paths** - `@std.math.pi` ✅
- **@this scope** - Parsed but not fully functional ⚠️

### 🚧 Not Yet Implemented

#### Critical Missing Features from LANGUAGE_SPEC.zen

##### Memory Management
- **@this.defer()** - RAII cleanup ❌
- **Pointer types** - `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` ❌
- **Allocators** - GPA, AsyncPool ❌

##### Error Handling
- **.raise()** - Error propagation ❌

##### Collections
- **Vec<T, N>** - Fixed-size vectors ❌
- **DynVec<T>** - Dynamic vectors with allocator ❌
- **Mixed type vectors** - `DynVec<Circle, Rectangle>` ❌

##### Advanced Patterns
- **UFC overloading** - Multiple function definitions ❌
- **Enum variant matching** - `GameEntity.Player` ❌
- **Shape.requires()** - Trait constraints on enums ❌
- **Generic functions** - `<T: Trait>` ❌

##### Concurrency
- **Actor** - Message passing ❌
- **Channel<T>** - Buffered channels ❌
- **Mutex<T>** - Shared state ❌
- **AtomicU32** - Atomic operations ❌

### 📋 Priority Roadmap (To Match LANGUAGE_SPEC.zen)

#### Phase 1: Fix Core Language (Priority)
- [ ] Fix Option<T> value extraction
- [ ] Fix Result<T, E> pattern matching
- [ ] Implement .raise() error propagation
- [ ] Add forward declarations (all 6 variable forms)
- [ ] Implement pointer types (Ptr, MutPtr, RawPtr)

#### Phase 2: Essential Features
- [ ] Implement Vec<T, N> and DynVec<T>
- [ ] Add @this.defer() for RAII
- [ ] UFC overloading for enum variants
- [ ] Generic functions `<T: Trait>`
- [ ] Shape.requires() trait enforcement

#### Phase 3: Concurrency & Allocators
- [ ] Allocators (GPA, AsyncPool)
- [ ] Actor system
- [ ] Channel<T> and Mutex<T>
- [ ] Atomic operations

#### Phase 4: Metaprogramming
- [ ] **reflect.ast()** - Runtime AST inspection
- [ ] **@meta.comptime()** - Compile-time execution
- [ ] **AST manipulation** - Code generation

#### Phase 5: FFI & Performance
- [ ] **inline.c()** - Inline C code
- [ ] **inline.llvm()** - Inline LLVM IR
- [ ] **simd operations** - Vector math
- [ ] **SDL2 bindings** - Game development

#### Phase 6: Build System
- [ ] **build.zen** - Build configuration
- [ ] **Conditional compilation** - Release/debug
- [ ] **Target selection** - C/LLVM/Native

## Working Examples

### Traits & Implementations (✅ Working)
```zen
{ io, math } = @std

Geometric: {
    area: (self) f64,
}

Circle: {
    radius: f64,
}

Circle.implements(Geometric, {
    area = (self) f64 {
        return math.pi * self.radius * self.radius
    },
})

main = () void {
    circle = Circle { radius: 5.0 }
    io.println("Area: ${circle.area()}")
}
```

### Pattern Matching (✅ Working)
```zen
main = () void {
    value = 42
    is_ready = true
    
    // Boolean patterns
    is_ready ?
        | true { io.println("Ready!") }
        | false { io.println("Not ready") }
    
    // Range loops
    (0..5).loop((i) {
        io.println("Count: ${i}")
    })
}
```

## Examples from LANGUAGE_SPEC.zen (Not Yet Working)

### Error Handling with .raise() (❌ TODO)
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // Will propagate Err
    contents = file.read_all().raise()
    return Ok(config)
}
```

### Actors & Concurrency (❌ TODO)
```zen
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
```

## Building & Running

```bash
# Build the compiler
cargo build --release

# Run a simple test
./target/release/zen tests/zen_test_simple.zen

# Run working language spec features
./target/release/zen tests/zen_test_working_spec.zen

# All test files are prefixed with zen_test_ and live in tests/
ls tests/zen_test_*.zen
```

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen      # THE source of truth - all features we need
├── src/                   # Rust compiler implementation
│   ├── ast/              # AST definitions
│   ├── parser/           # Parser (no keywords!)
│   ├── codegen/llvm/     # LLVM code generation
│   ├── typechecker/      # Type system & traits
│   └── stdlib/           # Built-in modules (io, math, etc.)
├── tests/                # Test suite (all zen_test_*.zen)
│   ├── zen_test_*.zen    # All test files prefixed with zen_test_
│   └── working/          # Tests that currently pass
└── README.md             # This file - current implementation status
```

## Contributing

All contributions must align with `LANGUAGE_SPEC.zen`. The spec is the authoritative source - if something differs from the spec, the spec wins.

## Philosophy Deep Dive

### No Keywords
Zen has **ZERO** keywords. No `if`, `else`, `while`, `for`, `match`, `switch`, `class`, `interface`, `impl`, `trait`, `async`, `await`, or `null`. Everything is achieved through:
- Pattern matching with `?`
- UFC (Uniform Function Call)
- Two special symbols: `@std` and `@this`

### No Function Coloring
Functions aren't marked `async`. Instead, the allocator determines behavior:
- Sync allocator = blocking calls
- Async allocator = non-blocking calls
Same function, different behavior based on context!

### No Null
No `null`, `nil`, or `undefined`. Only `Option<T>`:
- `Some(value)` - has a value
- `None` - no value

### Explicit Memory
No hidden pointers. Choose explicitly:
- `Ptr<T>` - Immutable pointer
- `MutPtr<T>` - Mutable pointer  
- `RawPtr<T>` - Raw pointer for FFI

## License

MIT License - See LICENSE file for details