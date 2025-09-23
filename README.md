# Zen Programming Language

**[`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) IS THE SOURCE OF TRUTH**

A revolutionary programming language with **ZERO KEYWORDS**. All control flow through pattern matching (`?`), UFC (Uniform Function Call), and structural typing via traits.

> "No keywords: `if/else/while/for/match/async/await/impl/trait/class/interface/null`"  
> — LANGUAGE_SPEC.zen, line 2

## Core Design Principles

From [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen):

1. **No keywords** - No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
2. **Only two @ symbols** - `@std` (standard library) and `@this` (current scope)
3. **Pattern matching with `?`** - Replaces all conditional keywords
4. **UFC (Uniform Function Call)** - Any function can be called as method
5. **Allocators determine sync/async** - No function coloring problem
6. **Explicit pointer types** - `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
7. **No null/nil** - Only `Option<T>` with `.Some(T)` and `.None`
8. **Assignment operators** - `=` (immutable), `::=` (mutable), `:` (type annotation)
9. **Error propagation** - `.raise()` not exceptions
10. **Traits** - Via `.implements()` and `.requires()`

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a Zen program
cargo run --bin zen examples/hello.zen

# Run comprehensive test
cargo run --bin zen tests/zen_test_language_spec_complete_validation.zen
```

## Working Example from LANGUAGE_SPEC.zen

```zen
// Imports - only @std and @this are special
{ io } = @std

// No null! Only Option types
Option<T>: Some(T) | None

// Simple struct with mutable fields
Point: {
    x:: f64,  // mutable field
    y:: f64
}

// Trait definition
Geometric: {
    area: (self) f64,
    perimeter: (self) f64,
}

// Implement trait for type
Circle: {
    center: Point,
    radius: f64,
}

Circle.implements(Geometric, {
    area = (self) f64 {
        return 3.14159 * self.radius * self.radius
    },
    perimeter = (self) f64 {
        return 2.0 * 3.14159 * self.radius
    },
})

main = () void {
    // All variable forms work
    x = 10           // Immutable
    y::= 20          // Mutable  
    z: i32 = 30      // Immutable with type
    
    // Pattern matching - no if/else!
    is_ready = true
    is_ready ? { 
        io.println("Ready!") 
    }
    
    // Full pattern match
    value = false
    value ?
        | true { io.println("Yes") }
        | false { io.println("No") }
    
    // Pointer types
    ptr: Ptr<i32> = x.ref()
    io.println("Value: ${ptr.val}, Address: ${ptr.addr}")
    
    // Range loops
    (0..5).loop((i) {
        io.println("Count: ${i}")
    })
    
    // Option handling - no null!
    maybe: Option<i32> = Option.Some(42)
    maybe ?
        | Some(val) { io.println("Value: ${val}") }
        | None { io.println("No value") }
}
```

## Implementation Status

### ✅ WORKING (Core Features ~70%)

| Feature | LANGUAGE_SPEC.zen Lines | Status |
|---------|------------------------|---------|
| **No Keywords** | 2-14 | ✅ Complete |
| **Variables** | 298-306 | ✅ All forms work |
| **Pattern Matching** | 352-361 | ✅ Full support |
| **Option/Result** | 109-113 | ✅ No nulls! |
| **Structs** | 117-120, 130-133 | ✅ With mutable fields |
| **Enums** | 165-168 | ✅ Sum types |
| **Pointer Types** | 364-372 | ✅ Ptr, MutPtr, RawPtr |
| **Loops** | 432-460 | ✅ Ranges, infinite |
| **String Interpolation** | Throughout | ✅ `"${expr}"` |
| **@std Imports** | 92-94 | ✅ Destructuring |

### ⚠️ PARTIAL (In Progress ~15%)

| Feature | LANGUAGE_SPEC.zen Lines | Issue |
|---------|------------------------|-------|
| **Traits** | 123-163 | Parsing works, execution issues |
| **UFC** | 172-181 | Basic works, overloading missing |
| **.raise()** | 205-211 | Parses, type issues |
| **@this.defer()** | 217, 313-314 | Parses, needs testing |

### ❌ NOT IMPLEMENTED (Missing ~15%)

| Feature | LANGUAGE_SPEC.zen Lines | Description |
|---------|------------------------|-------------|
| **DynVec** | 317, 377 | Mixed type vectors |
| **Allocators** | 309-314 | GPA, AsyncPool |
| **Concurrency** | 399-430 | Actor, Channel, Mutex |
| **Reflection** | 244-272 | reflect.ast() |
| **Metaprogramming** | 274-281 | @meta.comptime() |
| **Module System** | 491-502 | module.exports |
| **FFI** | 285-289 | inline.c(), inline.llvm() |
| **SIMD** | 291-294 | Vector operations |
| **StringBuilder** | 387-394 | String building |
| **Step Ranges** | 437-439 | (0..100).step(10) |

## Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen           # THE source of truth
├── IMPLEMENTATION_REALITY.md   # Honest assessment
├── src/
│   ├── main.rs                 # Compiler entry
│   ├── parser/                 # No-keyword parsing
│   ├── codegen/llvm/           # LLVM backend
│   └── typechecker/            # Type system
└── tests/
    └── zen_test_*.zen          # Feature tests
```

## Test Coverage

Run the comprehensive validation:
```bash
cargo run --bin zen tests/zen_test_language_spec_complete_validation.zen
```

This test validates every feature from LANGUAGE_SPEC.zen and reports what works and what doesn't.

## Philosophy

Zen achieves unprecedented simplicity:

- **Zero keywords** - Nothing to memorize
- **One operator** - `?` for all control flow  
- **No nulls** - Eliminates entire class of bugs
- **Explicit mutability** - Clear data flow
- **No function coloring** - Allocators handle async

## Current Limitations

Based on LANGUAGE_SPEC.zen, these features are planned but not yet implemented:

1. **Full trait dispatch** - Methods don't always resolve correctly
2. **DynVec for mixed types** - Key differentiator feature
3. **Allocator-based async** - The elegant async story
4. **Reflection & metaprogramming** - Compile-time power
5. **Module system** - For real programs

## Building

```bash
# Prerequisites
# - Rust 1.70+
# - LLVM 18
# - Clang

git clone https://github.com/zenlang/zenlang
cd zenlang
cargo build --release

# Run tests
./run_tests.sh

# Or specific test
cargo run --bin zen tests/zen_test_language_spec_main.zen
```

## Contributing

Priority areas based on LANGUAGE_SPEC.zen gaps:

1. Fix trait method dispatch (lines 123-163)
2. Implement DynVec (lines 317, 377)
3. Add allocators (lines 309-314)
4. Concurrency primitives (lines 399-430)
5. Module system (lines 491-502)

See [`IMPLEMENTATION_REALITY.md`](./IMPLEMENTATION_REALITY.md) for detailed status.

## License

MIT

---

**The dream:** Every line in [`LANGUAGE_SPEC.zen`](./LANGUAGE_SPEC.zen) working perfectly.  
**The reality:** Core philosophy works, advanced features in progress.  
**The promise:** Zero keywords, maximum elegance.