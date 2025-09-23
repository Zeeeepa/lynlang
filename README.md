# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) is the authoritative source of truth**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching, UFC (Uniform Function Call), and no function coloring.

> **Current Status:** ~15% of LANGUAGE_SPEC.zen implemented. Basic foundation working - immutable variables, io.println, arithmetic, direct boolean literals, string literals. Major gaps: mutable variables (::=), pattern matching (?), structs, enums, Option<T>, Result<T,E>, loops, ranges, UFC, pointers, traits, error propagation (.raise()), generics, concurrency, metaprogramming.

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

### ✅ Core Features Working (15% Complete)

#### No Keywords Philosophy
- **Zero keywords** - Philosophy established in parser ✅
- **Pattern matching with `?`** - Parsed but not fully working ❌
- **UFC (Uniform Function Call)** - Parsed but not implemented ❌

#### Type System
- **Basic types** - i32, i64, f32, f64, bool, string ✅
- **Type inference** - Basic inference for literals ✅
- **Structs** - Parsed but not fully working ❌
- **Traits** - Parsed but not working ❌
- **Enums** - Parsed but not working ❌
- **Option<T>** - Not working ❌
- **Result<T, E>** - Not working ❌

#### Variables & Assignment
Basic forms from LANGUAGE_SPEC.zen:
```zen
x = 10          // Immutable assignment ✅
y = 20          // Inferred immutable ✅
// Other forms parsed but not fully working:
x: i32          // Forward declaration ❌
z: i32 = 30     // Typed immutable ⚠️
w:: i32         // Mutable forward declaration ❌
w ::= 40        // Mutable assignment ❌
v ::= 50        // Inferred mutable ❌
u:: i32 = 60    // Typed mutable ❌
```

#### Control Flow & Iteration
- **Direct boolean literals** - `true`/`false` work in expressions ✅
- **Boolean variables** - Storage/loading has issues ❌
- **Pattern matching** - Parsed but not working ❌
- **Range loops** - Not working ❌
- **Infinite loops** - Not working ❌
- **Break/Continue** - Not working ❌
- **String interpolation** - Not working ❌

#### Imports & Modules
- **@std imports** - `{ io } = @std` basic form works ✅
- **io.println()** - Works for strings, integers, floats, direct booleans ✅
- **Other @std modules** - Not working ❌
- **@this scope** - Not implemented ❌
- **module.exports/import** - Not implemented ❌

#### Core Working Features
- **Comments** - Single-line `//` comments ✅
- **Arithmetic** - Basic +, -, *, / operations ✅
- **String literals** - Basic string support ✅
- **Integer literals** - i32/i64 support ✅
- **Float literals** - f32/f64 support ✅
- **Function definitions** - `main = () void { }` ✅

### 🚧 Not Yet Implemented (85% of LANGUAGE_SPEC.zen)

#### Core Language Features
- **Mutable variables** - `::=` operator and mutable declarations ❌
- **Pattern matching** - `?` operator for control flow ❌
- **Structs** - Definition and field access ❌
- **Enums** - Sum types with variants ❌
- **Option<T>** - No null, only Option types ❌
- **Result<T, E>** - Error handling type ❌
- **Loops** - `loop()` and collection `.loop()` ❌
- **Ranges** - `(0..10)` syntax and iteration ❌
- **String interpolation** - `"Value: ${expr}"` ❌
- **UFC** - Method-style function calls ❌
- **@this** - Current scope reference ❌
- **Defer** - `@this.defer()` cleanup ❌

#### Type System
- **Pointer types** - `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` ❌
- **Traits** - `.implements()` and `.requires()` ❌
- **Generics** - Type parameters and constraints ❌

#### Memory & Concurrency
- **Allocators** - GPA, AsyncPool, sync/async behavior ❌
- **Vec<T, N>** - Fixed-size vectors ❌
- **DynVec<T>** - Dynamic vectors with allocator ❌
- **Actor** - Message passing concurrency ❌
- **Channel<T>** - Buffered channels ❌
- **Mutex<T>** - Shared state ❌
- **AtomicU32** - Atomic operations ❌

#### Advanced Features
- **.raise()** - Error propagation ❌
- **Metaprogramming** - `@meta.comptime()` and AST manipulation ❌
- **Reflection** - `reflect.ast()` runtime inspection ❌
- **FFI** - Foreign function interface ❌
- **SIMD** - Vector operations ❌
- **Module system** - `module.exports` and `module.import` ❌

### 📋 Implementation Roadmap

#### Phase 1: Core Language (Next Priority)
- [ ] Fix boolean variable storage/loading
- [ ] Implement mutable variables (`::=`)
- [ ] Implement pattern matching with `?`
- [ ] Implement basic structs
- [ ] Implement basic enums
- [ ] Implement Option<T> type
- [ ] Implement Result<T, E> type

#### Phase 2: Control Flow & Iteration
- [ ] Implement loops (`loop()` and `.loop()`)
- [ ] Implement ranges `(0..10)`
- [ ] Implement break/continue
- [ ] Implement UFC (Uniform Function Call)
- [ ] Implement string interpolation

#### Phase 3: Type System & Memory
- [ ] Implement pointer types (Ptr, MutPtr, RawPtr)
- [ ] Implement traits (`.implements()` and `.requires()`)
- [ ] Implement generics
- [ ] Implement allocators
- [ ] Implement Vec and DynVec

#### Phase 4: Advanced Features
- [ ] Implement concurrency (Actor, Channel, Mutex)
- [ ] Implement error propagation (`.raise()`)
- [ ] Implement metaprogramming
- [ ] Implement module system

## Working Examples

> **Best Demo:** Run `./target/release/zen tests/zen_test_spec_working_now.zen` to see all working features!

### Basic Working Example
```zen
{ io } = @std

main = () void {
    // Basic immutable variables work
    x = 42
    y = 3.14
    message = "Hello, Zen!"
    
    // Arithmetic works
    sum = x + 10
    
    // io.println works for various types
    io.println(message)
    io.println(x)
    io.println(y)
    io.println(sum)
    
    // Direct boolean literals work
    io.println(true)
    io.println(false)
}
```

## Key Features from LANGUAGE_SPEC.zen (Goals)

The language aims to have:

### No Keywords Philosophy
```zen
// No if/else/while/for/match keywords!
// Everything is pattern matching and function calls
is_ready ? { io.println("Go!") }  // Instead of if
(0..10).loop((i) { })              // Instead of for
loop(() { })                       // Instead of while
```

### Pattern Matching with ?
```zen
// All control flow via pattern matching
value ?
    | Some(x) { io.println(x) }
    | None { io.println("empty") }
```

### UFC (Uniform Function Call)
```zen
// Any function can be called as a method
list.map(double)    // Same as map(list, double)
5.times(() { })     // Same as times(5, () { })
```

### No Null - Only Option Types
```zen
// No null/nil/undefined
maybe: Option<i32> = Some(42)
empty: Option<i32> = None  // Not null!
```

## Building & Running

```bash
# Build the compiler
cargo build --release

# Run the working features demo
./target/release/zen tests/zen_test_spec_working_now.zen

# Run a simple test
./target/release/zen tests/zen_test_simple.zen

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

**Implementation Status: ~15% Complete**

Current state: Basic foundation is working - immutable variables, io.println for various types, arithmetic operations, string/integer/float literals, and direct boolean literals. The parser recognizes most of the language syntax but code generation is incomplete.

**Known Issues:**
- Boolean variables don't load/store correctly (direct literals work)
- Most advanced features are parsed but not compiled

**Next Priority:** Fix boolean variables, implement mutable variables (`::=`), pattern matching (`?`), basic structs, and enums.

## Contributing

All contributions must align with [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen). The spec is the authoritative source - if something differs from the spec, the spec wins.

Focus areas:
1. Implement missing features from LANGUAGE_SPEC.zen
2. Fix boolean variable storage
3. Complete pattern matching implementation
4. Add struct and enum support

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