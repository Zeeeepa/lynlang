# Zen Language Examples

Clean, organized examples demonstrating Zen's elegance and expressiveness.

## 🚀 Quick Start

```bash
# Start here - learn Zen in 5 minutes
zen run examples/quickstart.zen

# Hello world
zen run examples/basics/hello_world.zen

# Complete showcase
zen run examples/showcase/comprehensive_demo.zen
```

## 📚 Structure

### `quickstart.zen`
Complete introduction covering all essential features in one file.

### `basics/` - Foundation
Step-by-step learning path:
1. **hello_world.zen** - Simplest program
2. **01_variables_and_types.zen** - Type system, bindings, arrays
3. **02_functions_and_control.zen** - Functions, loops, pattern matching
4. **03_structs_and_methods.zen** - User types, UFCS, generics, enums
5. **04_collections_and_iteration.zen** - Vectors, maps, functional ops
6. **05_strings_and_io.zen** - String manipulation, formatting

### `patterns/` - Idiomatic Zen
- **pattern_matching.zen** - The `?` operator for all control flow
- **error_handling.zen** - Result/Option types, `.raise()` propagation

### `advanced/` - Complex Features
- **algorithms.zen** - Sorting, searching, memoization
- **modules.zen** - Module system, exports, organization

### `concurrency/` - Parallel Programming
- **channels.zen** - Producer-consumer, pipelines, work pools
- **atomics.zen** - Lock-free counters, spinlocks, mutexes

### `showcase/` - Complete Programs
- **comprehensive_demo.zen** - Full language showcase with game engine example
- **ffi.zen** - C interop via `inline.c` with benchmarking
- **concurrency.zen** - Advanced concurrent patterns (worker pools, rate limiting, pub/sub)

## 🎯 Key Principles

### No Keywords
Zen has no `if`, `else`, `while`, `for`, `match`, `async`, `await`, `impl`, `trait`, `class`, `interface`, or `null`.

### Pattern Matching Everything
```zen
// All conditionals use ?
x < 0 ?
    | true { negative_case() }
    | false { positive_case() }

// Enum matching
status ?
    | Ok -> val { use_value(val) }
    | Err -> e { handle_error(e) }
```

### UFCS (Uniform Function Call Syntax)
```zen
// Define methods on any type
Point.distance = (self, other: Point) f64 { ... }

// Call as method or function
p1.distance(p2)  // Method style
distance(p1, p2)  // Function style
```

### Colorless Functions
```zen
// Same function works sync or async
fetch = (url: string, alloc: Ptr<Allocator>) Result<Data, Error> {
    // Allocator determines execution model
}
```

### Error Handling
```zen
// Propagate errors with .raise()
result := risky_operation().raise()

// Handle explicitly
risky_operation() ?
    | Ok -> val { process(val) }
    | Err -> e { log_error(e) }
```

## 🔧 Running Examples

```bash
# Run any example
zen run examples/<path>/<file>.zen

# Build for production
zen build examples/showcase/comprehensive.zen -O3

# Check syntax
zen check examples/basics/*.zen
```

## 📖 Learning Path

1. Start with `quickstart.zen`
2. Work through `basics/` in order
3. Study `patterns/` for idiomatic code
4. Explore `advanced/` and `concurrency/`
5. Review `showcase/` for real applications

See `LANGUAGE_SPEC.zen` for complete language specification.