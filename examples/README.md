# Zen Language Examples

Learn Zen through practical, well-organized examples.

## Quick Start

```bash
# Minimal example
zen run hello_world.zen

# 5-minute introduction  
zen run quickstart.zen
```

## Directory Structure

### `/tutorials`
Step-by-step learning path for Zen fundamentals:
1. **01_variables_and_types.zen** - Type system basics
2. **02_functions_and_control.zen** - Functions and control flow
3. **03_structs_and_methods.zen** - Data structures
4. **04_collections_and_iteration.zen** - Collections and loops
5. **05_strings_and_io.zen** - String manipulation and I/O

### `/patterns`
Advanced patterns and idioms:
- **algorithms.zen** - Algorithm implementations
- **concurrency.zen** - Async, channels, atomics
- **error_handling.zen** - Result types and error propagation
- **ffi.zen** - Foreign Function Interface
- **modules.zen** - Module system
- **pattern_matching.zen** - Advanced pattern matching

### `/showcase`
- **comprehensive_demo.zen** - Complete feature demonstration

## Running Examples

```bash
# Run any example
zen run <file>.zen

# Build to executable
zen build <file>.zen -o output
```

## Learning Path

1. Start with `hello_world.zen`
2. Review `quickstart.zen` for overview
3. Work through `/tutorials` in order
4. Explore `/patterns` based on interest
5. Study `/showcase` for real-world usage

## Key Language Features

- **Type Safety**: Strong static typing with inference
- **Pattern Matching**: Expressive `?` operator
- **Memory Safety**: No manual memory management
- **Async/Await**: First-class concurrency
- **Compile-Time Execution**: `comptime` blocks
- **FFI**: Seamless C integration with `inline.c()`

See `LANGUAGE_SPEC.zen` for complete language reference.