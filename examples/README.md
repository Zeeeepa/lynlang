# Zen Examples

Practical examples showcasing Zen's elegance, expressiveness, and power.

## Quick Start

```bash
zen run hello_world.zen      # Minimal example
zen run quickstart.zen        # Essential features in 5 minutes
```

## Structure

### `quickstart.zen`
Essential Zen syntax - variables, UFC, pattern matching, error handling

### `tutorials/`
Progressive learning path:
1. **Variables & Types** - Type system, assignments (=, ::=, :)
2. **Functions & Control** - UFC, pattern matching with ?
3. **Structs & Methods** - Data structures and behaviors
4. **Collections** - Arrays, maps, iterators
5. **Strings & I/O** - Text processing and file operations

### `patterns/`
Idiomatic Zen patterns:
- **algorithms** - Sorting, searching, graph algorithms
- **concurrency** - Channels, spawning, worker pools
- **error_handling** - Result<T,E>, error propagation
- **ffi** - C integration with inline.c()
- **modules** - Code organization and namespaces
- **pattern_matching** - Sum types and destructuring

### `showcase/`
**full_demo.zen** - Complete feature demonstration with all major Zen capabilities

### `common.zen`
Shared utilities and types (Point, Shape, helpers) to maintain DRY principles

## Key Features

- **No keywords** - No if/else/match/async/null
- **Pattern matching** - Single ? operator
- **UFC** - Any function as method
- **Error handling** - Result<T,E> and Option<T>
- **FFI** - Inline C with inline.c()

## Learning Path

1. `hello_world.zen` - First program
2. `quickstart.zen` - Core concepts
3. `tutorials/` - Systematic learning
4. `patterns/` - Advanced techniques
5. `showcase/` - Full applications

See `LANGUAGE_SPEC.zen` for complete reference.