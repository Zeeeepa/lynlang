# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source of truth**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching, UFC (Uniform Function Call), and no function coloring.

> **Current Status:** ~30% of LANGUAGE_SPEC.zen implemented. Core philosophy realized! Pattern matching, UFC, all variable forms working. Option<i32> payloads extract correctly! Major gaps: Option<string> payloads, pointers, collections, error propagation, generics, concurrency, metaprogramming.

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

### ✅ Core Features Working (30% Complete)

#### No Keywords Philosophy
- **Zero keywords** - All control flow via pattern matching
- **Pattern matching with `?`** - Boolean and enum patterns
- **UFC (Uniform Function Call)** - Any function can be called as method

#### Type System
- **Structs** - Basic definition and creation ✅
- **Traits** - `.implements()` parsed ⚠️ (methods can't access struct fields)
- **Enums** - Option/Result work with pattern matching ✅
- **Option<i32>** - Pattern matching AND value extraction working! ✅ (FIXED!)
- **Option<string>** - Pattern matching works, string payloads show as addresses ⚠️
- **Result<T, E>** - Definition works, integer payloads should work ⚠️

#### Variables & Assignment
All 8 forms from LANGUAGE_SPEC.zen (including mutable variants):
```zen
x: i32          // Forward declaration ✅
x = 10          // Immutable assignment ✅
y = 20          // Inferred immutable ✅
z: i32 = 30     // Typed immutable ✅
w:: i32         // Mutable forward declaration ✅
w = 40          // Mutable assignment ✅
v ::= 50        // Inferred mutable ✅
u:: i32 = 60    // Typed mutable ✅
```

#### Control Flow & Iteration
- **Pattern matching** - Boolean patterns with `?` ✅
- **Range loops** - `(0..10).loop()` ✅
- **Infinite loops** - `loop(() { ... })` ✅
- **Break statement** - Works in loops ✅
- **String interpolation** - Basic `"Value: ${expr}"` ✅
- **Continue statement** - Not working ❌
- **Range step** - `(0..10).step(2)` ❌

#### Imports & Modules
- **@std imports** - `{ io, math } = @std` ✅
- **Module paths** - `math.pi` works ✅
- **@this scope** - Not implemented ❌
- **module.exports/import** - Not implemented ❌

#### Memory Management
- **@this.defer()** - Basic RAII works ✅
- **Allocators** - Not implemented ❌

### 🚧 Not Yet Implemented

#### Critical Missing Features from LANGUAGE_SPEC.zen

##### Memory Management
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

### 📋 Implementation Roadmap (To Complete LANGUAGE_SPEC.zen)

#### Phase 1: Fix Core Type System (Priority)
- [x] Fix Option<i32> value extraction ✅ DONE!
- [ ] Fix Option<string> payload handling
- [ ] Test Result<T, E> pattern matching and value extraction
- [ ] Fix struct field access in trait methods
- [ ] Implement pointer types (Ptr, MutPtr, RawPtr)
- [ ] Fix boolean single pattern execution

#### Phase 2: Error Handling & Collections
- [ ] Implement .raise() error propagation
- [ ] Implement Vec<T, N> fixed-size vectors
- [ ] Implement DynVec<T> dynamic vectors
- [ ] Implement mixed type vectors (DynVec<T1, T2>)
- [ ] Implement StringBuilder

#### Phase 3: Generics & UFC
- [ ] Generic functions `<T: Trait>`
- [ ] Multiple trait constraints (T: A + B)
- [ ] UFC overloading for enum variants
- [ ] Enum variant constructors (GameEntity.Player)
- [ ] .requires() trait enforcement on enums

#### Phase 4: Concurrency & Allocators
- [ ] Allocators (GPA, AsyncPool)
- [ ] Actor system for message passing
- [ ] Channel<T> buffered channels
- [ ] Mutex<T> for shared state
- [ ] AtomicU32 atomic operations

#### Phase 5: Metaprogramming
- [ ] reflect.ast() - Runtime AST inspection
- [ ] @meta.comptime() - Compile-time execution
- [ ] AST manipulation - Code generation
- [ ] Type introspection at runtime

#### Phase 6: FFI & Build System
- [ ] inline.c() - Inline C code
- [ ] inline.llvm() - Inline LLVM IR
- [ ] simd operations - Vector math
- [ ] build.zen - Build configuration
- [ ] Target selection - C/LLVM/Native
- [ ] SDL2 FFI bindings

## Working Examples

> **Best Demo:** Run `./target/release/zen tests/zen_test_language_spec_working_2025.zen` to see all working features!

### Variable Declarations (✅ ALL 8 FORMS WORKING)
```zen
{ io } = @std

main = () void {
    // All 8 forms from LANGUAGE_SPEC.zen work!
    x: i32          // Forward declaration
    x = 10
    y = 20          // Immutable inferred
    z: i32 = 30     // Typed immutable
    w:: i32         // Mutable forward declaration  
    w = 40
    v ::= 50        // Mutable inferred
    u:: i32 = 60    // Typed mutable
    
    v = 70          // Can reassign mutable
    io.println("All variable forms work!")
}
```

### Pattern Matching & Loops (✅ Working)
```zen
{ io } = @std

main = () void {
    // Boolean pattern matching - no if/else keywords!
    is_ready = false
    is_ready ?
        | true { io.println("Ready!") }
        | false { io.println("Not ready") }
    
    // Range loops - no for keyword!
    (0..3).loop((i) {
        io.println("Count: ${i}")
    })
    
    // Infinite loop with break - no while keyword!
    counter ::= 0
    loop(() {
        counter = counter + 1
        counter > 2 ? {
            break
        }
        io.println("Loop: ${counter}")
    })
}
```

## Recent Fixes & Improvements

### Option<i32> Value Extraction (✅ NOW WORKING!)
```zen
// This now works correctly - major milestone achieved!
maybe: Option<i32> = Some(42)
maybe ?
    | Some(v) { io.println("Value: ${v}") }  // Prints: "Value: 42" ✅
    | None { io.println("None") }
```

### Option<string> (⚠️ Partially Working)
```zen
// String payloads extract but show as memory addresses
maybe: Option<string> = Some("Hello")
maybe ?
    | Some(s) { io.println("Value: ${s}") }  // Shows address, not "Hello"
    | None { io.println("None") }
```

### Error Propagation (❌ TODO)
```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()  // .raise() not implemented
    contents = file.read_all().raise()
    return Ok(config)
}
```

### Pointers (❌ TODO)
```zen
// No pointer types yet - spec requires Ptr<T>, MutPtr<T>, RawPtr<T>
circle = Circle { radius: 5.0 }
ptr: Ptr<Circle> = circle.ref()        // Not implemented
mut_ptr: MutPtr<Circle> = circle.mut_ref()  // Not implemented
val = ptr.val                          // Dereference not implemented
addr = ptr.addr                        // Address not implemented
```

### Collections (❌ TODO)
```zen
// No Vec or DynVec yet
static_vec = Vec<i32, 100>()          // Fixed-size vector
dynamic_vec = DynVec<Shape>(alloc)    // Dynamic vector
mixed = DynVec<Circle, Rectangle>(alloc)  // Mixed types
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

## Summary

**Implementation Status: ~30% Complete**

✨ **Major Milestone:** Option<i32> payload extraction now works correctly! The Zen compiler successfully implements the core philosophy of zero keywords with pattern matching and UFC. All 8 variable declaration forms work perfectly.

**Recent Achievement:** Fixed enum payload storage to properly preserve integer types, solving the long-standing Option<T> extraction issue for numeric types.

**Next Priority:** Fix Option<string> payload handling, then implement .raise() error propagation and pointer types from LANGUAGE_SPEC.zen.

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