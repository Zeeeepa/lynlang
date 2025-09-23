# Zen Examples

Clean demonstrations of Zen's core philosophy: **elegance, efficiency, and expressiveness**.

## Quick Start

```bash
zen run hello_world.zen   # Minimal first program
zen run showcase.zen      # Core language features  
zen run todo_app.zen      # Complete application
```

## Examples

### `hello_world.zen`
The simplest possible Zen program. Start here.

### `showcase.zen`
Interactive demonstration of Zen's unique features:
- **UFC** - Universal Function Call syntax
- **Pattern matching** - No if/else, only `?`
- **ADTs** - Algebraic Data Types with variants
- **Results** - Error handling as values
- **Pipelines** - Functional composition
- **State machines** - Type-safe state transitions

### `todo_app.zen`  
Complete terminal application showcasing:
- Struct methods and data modeling
- Collection manipulation
- Error propagation with `Result<T,E>`
- Real-world patterns

## Core Philosophy

```zen
// No keywords - pattern matching replaces control flow
n > 0 ? | true { "positive" } | false { "zero or negative" }

// UFC - any function becomes a method
double = (x: i32) i32 { x * 2 }
result = 5.double()  // 10

// ADTs - type-safe variants
Shape: Circle(f64) | Rectangle(f64, f64)

// Errors as values, not exceptions
divide(10, 0) ? | Ok(v) { v } | Err(e) { 0 }
```

**DRY** | **KISS** | **Zero-cost abstractions**