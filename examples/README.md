# Zen Examples

Clean, elegant demonstrations of Zen's design principles.

## Quick Start

```bash
zen run hello_world.zen          # Minimal example
zen run quickstart.zen           # Core features tour  
zen run showcase/full_demo.zen   # Complete showcase
```

## Core Principles

- **No keywords** - Pattern matching with `?`, no `if/else/match/async/await`
- **UFC** - Any function becomes a method: `5.double().square()`
- **Explicit pointers** - `Ptr<T>`, `MutPtr<T>`, `RawPtr<T>`
- **No nulls** - Only `Option<T>` with `.Some(T)` and `.None`
- **Two imports** - `@std` (stdlib) and `@this` (current scope)

## Structure

```
├── hello_world.zen      # Minimal program
├── quickstart.zen       # Essential syntax  
├── common.zen          # Shared utilities
├── tutorials/          # Step-by-step learning (01-05)
├── patterns/           # Idiomatic code patterns
│   ├── algorithms.zen  # Sort, search, dynamic programming
│   ├── concurrency.zen # Channels and spawn
│   ├── error_handling.zen  # Result<T,E> patterns
│   ├── ffi.zen        # C integration
│   ├── modules.zen    # Code organization  
│   └── pattern_matching.zen # Algebraic types
└── showcase/
    └── full_demo.zen  # Complete feature tour
```

## Learning Path

1. **Start**: `hello_world.zen` → `quickstart.zen`
2. **Learn**: Work through `tutorials/` (01-05)
3. **Practice**: Study `patterns/` for idiomatic code
4. **Master**: See `showcase/full_demo.zen` for everything

## Quick Syntax

```zen
// Variables
name = "immutable"          // Immutable by default
count ::= 0                  // Mutable with ::=

// UFC - Any function as method
5.double().square()          // Method chaining

// Pattern matching
x > 0 ? | true { "pos" } | false { "neg" }

// Explicit errors
result ? | Ok(v) { use(v) } | Err(e) { handle(e) }

// FFI
inline.c(""" int add(int a, int b) { return a + b; } """)
```

→ See `../LANGUAGE_SPEC.zen` for complete specification