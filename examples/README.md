# Zen Language Examples

Clean, focused examples demonstrating Zen's elegant design.

## Quick Start

```bash
# Hello world
zen run hello_world.zen

# Language tour (3 minutes)
zen run quickstart.zen  

# Feature showcase
zen run showcase/full_demo.zen

# Complete application
cd full_example && zen run main.zen
```

## Structure

```
examples/
├── hello_world.zen      # Minimal first program
├── quickstart.zen       # Core language tour
├── common.zen          # Shared utilities
│
├── tutorials/          # Step-by-step learning
│   ├── 01_variables_and_types.zen
│   ├── 02_functions_and_control.zen
│   ├── 03_structs_and_methods.zen
│   ├── 04_collections_and_iteration.zen
│   └── 05_strings_and_io.zen
│
├── patterns/           # Idiomatic code patterns
│   ├── algorithms.zen
│   ├── concurrency.zen
│   ├── error_handling.zen
│   ├── ffi.zen
│   ├── modules.zen
│   └── pattern_matching.zen
│
├── showcase/           # Feature demonstrations
│   └── full_demo.zen   # All features integrated
│
└── full_example/       # Complete application
    ├── main.zen        # Application entry
    └── math_utils.zen  # Utility module
```

## Core Features

### UFC (Universal Function Call)
```zen
// Any function becomes a method
double = (x: i32) i32 { x * 2 }
result = 5.double().double()  // 20
```

### Pattern Matching Only
```zen
// No if/else - just patterns
x > 0 ? 
    | true { "positive" } 
    | false { "negative" }
```

### Explicit Error Handling
```zen
// Results, not exceptions
divide(10, 2) ? 
    | Ok(v) { io.println("${v}") } 
    | Err(e) { io.println("Error: ${e}") }
```

### Zero-Cost FFI
```zen
// Inline C for performance
inline.c("
    int add(int a, int b) { 
        return a + b; 
    }
")
```

## Learning Path

**Beginner** → `hello_world.zen` → `quickstart.zen` → `tutorials/`  
**Intermediate** → `patterns/` → `common.zen` → `showcase/`  
**Advanced** → `full_example/` → Build your own

## Philosophy

✓ **Simplicity** - Minimal, consistent syntax  
✓ **Elegance** - Beautiful, expressive code  
✓ **Performance** - Zero-cost abstractions  
✓ **Safety** - No null, explicit errors  

→ Full specification: `../LANGUAGE_SPEC.zen`