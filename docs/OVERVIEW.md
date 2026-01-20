# Zen Language Overview

> **The World's First AI-Native Systems Programming Language with ZERO KEYWORDS**

**Version**: 0.7.1 | **Status**: Late Alpha (90% Core Complete) | **Updated**: January 2026

---

## What is Zen?

Zen is a systems programming language that eliminates all control flow keywords, replacing them with pattern matching and Uniform Function Call (UFC). Designed through human-AI collaboration, Zen prioritizes clarity, explicit control, and type safety.

### Design Philosophy

| Traditional Languages | Zen Approach |
|-----------------------|--------------|
| `if/else/while/for/match` keywords | Pattern matching with `?` operator |
| Implicit memory allocation | Explicit Zig-style allocators |
| `null`/`nil` values | `Option<T>` with `.Some()` and `.None` |
| Method vs function distinction | UFC - any function callable as method |
| Exception-based error handling | `Result<T,E>` with `.raise()` propagation |
| Raw pointers (`*`, `&`) | Type-safe `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>` |

---

## Quick Examples

### Hello World

```zen
{ io } = @std

main = () i32 {
    io.println("Hello, World!")
    return 0
}
```

### Variable Declaration (6 Forms)

```zen
x = 10              // Immutable, type inferred
x: i32 = 10         // Immutable, explicit type
x:: i32             // Mutable, forward declaration
x ::= 10            // Mutable, type inferred
x:: i32 = 10        // Mutable, explicit type
x = x + 1           // Reassignment (after mutable declaration)
```

### Pattern Matching (Replaces All Conditionals)

```zen
// Boolean patterns
is_ready ? { start() }

// Multi-branch patterns
status ?
    | .Active { process() }
    | .Inactive { wait() }
    | .Banned { reject() }

// Option patterns
maybe_value ?
    | Some(v) { io.println("Got: ${v}") }
    | None { io.println("Empty") }
```

### Structs and Enums

```zen
// Struct with default value
Point: {
    x:: f64,
    y:: f64 = 0.0
}

// Simple enum
Status: Active, Inactive, Banned

// Generic enum with payloads
Result<T, E>: Ok: T, Err: E
Option<T>: Some: T, None
```

### UFC (Uniform Function Call)

```zen
double = (n: i32) i32 { return n * 2 }

result = double(5)     // Traditional: 10
result = 5.double()    // UFC style: 10
result = 5.double().double().double()  // Chained: 40
```

### Error Handling

```zen
load_config = (path: string) Result<Config, Error> {
    file = File.open(path).raise()       // Early return on Err
    contents = file.read_all().raise()   // Propagates errors
    return Ok(parse(contents))
}
```

### Loops

```zen
// Range iteration
(0..10).loop((i) { io.println("${i}") })

// Collection iteration
items.loop((item) { process(item) })

// Infinite loop with break
loop(() {
    condition ? { break }
})
```

### Behaviors (Traits)

```zen
Drawable: {
    draw: (self) void,
    bounds: (self) Rect,
}

Circle.implements(Drawable, {
    draw = (self) void { /* ... */ },
    bounds = (self) Rect { /* ... */ },
})
```

---

## Type System

### Primitive Types

| Category | Types |
|----------|-------|
| Signed integers | `i8`, `i16`, `i32`, `i64` |
| Unsigned integers | `u8`, `u16`, `u32`, `u64`, `usize` |
| Floating point | `f32`, `f64` |
| Other | `bool`, `void`, `string` |

### Core Generic Types

- `Option<T>` - Some(T) or None (no null)
- `Result<T, E>` - Ok(T) or Err(E) (no exceptions)
- `Vec<T>` - Growable array with allocator
- `Ptr<T>` - Immutable pointer
- `MutPtr<T>` - Mutable pointer
- `RawPtr<T>` - Unsafe raw pointer

---

## Compiler Architecture

```
Source (.zen)
      |
      v
+-----------+
|   Lexer   |  Tokenization
+-----+-----+
      | Tokens
      v
+-----------+
|  Parser   |  AST Construction
+-----+-----+
      | AST
      v
+---------------+
| Module System |  Import Resolution
+-------+-------+
        |
        v
+---------------+
| Type Checker  |  Type Inference & Validation
+-------+-------+
        | Typed AST
        v
+----------------+
| Monomorphizer  |  Generic -> Concrete Types
+--------+-------+
         |
         v
+--------------+
| LLVM Codegen |  IR Generation
+------+-------+
       |
       v
  Machine Code
```

### Codebase Metrics

| Component | Lines of Code |
|-----------|---------------|
| Rust Compiler | ~21,600 |
| Zen Standard Library | ~13,500 |
| LSP Server | ~3,700 |
| Tests | ~2,200 |

---

## Standard Library

### Module Overview

```
stdlib/
+-- std.zen              # Entry point, re-exports
+-- compiler.zen         # Compiler intrinsics API
+-- math.zen             # Math functions
+-- testing.zen          # Test framework
+-- time.zen             # Time operations
|
+-- core/                # Fundamental types
|   +-- option.zen       # Option<T>
|   +-- result.zen       # Result<T,E>
|   +-- ptr.zen          # Pointer types
|   +-- iterator.zen     # Iterator traits
|
+-- collections/         # Data structures
|   +-- string.zen       # Dynamic string
|   +-- vec.zen          # Growable vector
|   +-- hashmap.zen      # Hash map
|   +-- set.zen          # Hash set
|   +-- stack.zen        # LIFO stack
|   +-- queue.zen        # FIFO queue
|   +-- linkedlist.zen   # Doubly-linked list
|
+-- memory/              # Memory management
|   +-- allocator.zen    # Allocator behavior
|   +-- gpa.zen          # General purpose allocator
|   +-- mmap.zen         # Memory-mapped regions
|
+-- io/                  # I/O operations
|   +-- io.zen           # Basic print/read
|   +-- files/           # File operations (syscall-based)
|   +-- net/             # TCP, UDP, Unix sockets
|   +-- mux/             # epoll, poll, io_uring
|
+-- concurrency/         # Threading & sync
|   +-- primitives/      # Atomic, futex
|   +-- sync/            # Mutex, channel, thread
|   +-- async/           # Task, scheduler
|   +-- actor/           # Actor model
|
+-- sys/                 # System interface
    +-- syscall.zen      # Syscall numbers (Linux x86-64)
    +-- env.zen          # Environment variables
    +-- process/         # Process management
    +-- random/          # Random number generation
```

### Memory Model: Zig-Style Allocators

All heap allocations are explicit via allocator parameters:

```zen
{ GPA } = @std.memory.gpa

allocator = GPA.new()
v = Vec<i32>.new(allocator)
v.mut_ref().push(42)
v.mut_ref().push(100)
v.mut_ref().free()  // Explicit deallocation
```

### I/O: Syscall-Based

Core I/O uses direct Linux syscalls (no libc dependency):

```zen
{ File } = @std.io.files

file = File.open("data.txt", O_RDONLY).raise()
contents = file.read_all(allocator).raise()
file.close()
```

---

## Tooling

### CLI Commands

```bash
zen                          # Start REPL
zen <file.zen>               # Compile and run (JIT)
zen <file.zen> -o <output>   # Compile to executable (AOT)
zen --help                   # Show help
```

### Additional Tools

| Tool | Purpose |
|------|---------|
| `zen-lsp` | Language Server Protocol server |
| `zen-check` | Syntax checker (no compilation) |
| `zen-format` | Code formatter |

### LSP Features

The language server provides full IDE support:

- Semantic completion with type awareness
- Auto-import suggestions
- Hover with type information
- Go to definition / Find references
- Document symbols and highlighting
- Error diagnostics with recovery
- Inlay hints for types
- Rename across workspace
- Signature help
- Code actions and quick fixes
- Semantic tokens for highlighting

---

## Building from Source

### Prerequisites

- Rust 1.75+
- LLVM 18.1
- Linux x86-64 (primary target)

### Build Commands

```bash
# Release build
cargo build --release

# Run tests
cargo test --all

# Build LSP server
cargo build --release --bin zen-lsp

# Run example
./target/release/zen examples/showcase.zen
```

---

## Implementation Status

### Complete

- Zero-keyword syntax with `?` pattern matching
- All 6 variable declaration forms
- Structs, enums, generic types
- UFC method chaining
- Option<T> and Result<T,E>
- Error propagation with `.raise()`
- Vec<T> with allocator support
- String operations and interpolation
- Behaviors (traits) system
- 25+ compiler intrinsics
- Full LSP implementation
- Syscall-based File I/O (Linux x86-64)
- TCP/UDP networking (syscall-based)

### In Progress

- Module system cross-boundary generics
- Iterator combinators (map, filter, collect)
- First-class closures as values

### Planned

- Cross-platform support (macOS, Windows)
- Package manager
- Self-hosting compiler

---

## Project Structure

```
zenlang/
+-- src/                    # Rust compiler source
|   +-- ast/                # AST definitions
|   +-- parser/             # Syntax analysis
|   +-- typechecker/        # Type checking & inference
|   +-- codegen/llvm/       # LLVM code generation
|   +-- lsp/                # Language server
|   +-- type_system/        # Generic type handling
|   +-- module_system/      # Module resolution
|   +-- comptime/           # Compile-time evaluation
|   +-- bin/                # CLI binaries
+-- stdlib/                 # Zen standard library
+-- tests/                  # Test suite
+-- examples/               # Example programs
+-- docs/                   # Documentation
+-- vscode-extension/       # VS Code integration
```

---

## Design Principles

1. **Zero Keywords** - All control flow through pattern matching
2. **Explicit Over Implicit** - Allocators, pointers, errors all explicit
3. **UFC Everywhere** - Any function is a potential method
4. **Type Safety** - No null, no implicit conversions
5. **Syscall-First** - Direct syscalls, minimal dependencies
6. **AI-Native** - Designed for human-AI collaboration

---

## Quick Reference

```zen
// === Variables ===
x = 10                    // Immutable
x ::= 10                  // Mutable

// === Pattern Matching ===
cond ? { action() }       // Single branch
val ? | A { } | B { }     // Multi-branch

// === Functions ===
add = (a: i32, b: i32) i32 { return a + b }

// === Structs ===
Point: { x: f64, y: f64 }
p = Point { x: 1.0, y: 2.0 }

// === Enums ===
Color: Red, Green, Blue
Option<T>: Some: T, None

// === UFC ===
result = value.method().chain()

// === Loops ===
(0..n).loop((i) { })      // Range loop
items.loop((x) { })       // Collection loop
loop(() { })              // Infinite loop

// === Errors ===
result = try_op().raise() // Propagate error

// === Pointers ===
ptr = value.ref()         // Immutable Ptr<T>
mptr = value.mut_ref()    // Mutable MutPtr<T>
```

---

*Zen: Where clarity meets performance.*
