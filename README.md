# Zen Programming Language

A modern systems programming language designed for clarity, performance, and safety. Zen prioritizes explicit, consistent, and elegant syntax without traditional if/else/match keywords, using a unified `?` pattern matching operator instead.

## Core Philosophy

- **NO keywords for control flow**: No `if`, `else`, `match` - only `?` operator
- **Clarity over cleverness**: Code readability is paramount
- **Explicit over implicit**: No hidden control flow or allocations
- **Errors as values**: No exceptions, use Result/Option types
- **Zero-cost abstractions**: Performance without compromise
- **Colorless async**: No function coloring with async/await

## Language Specification

Zen follows a strict [Language Specification v1.1.0](LANGUAGE_SPEC.md) that defines:
- **NO** `if`/`else`/`match` keywords - Use `?` operator exclusively
- **NO** exceptions - All errors are values
- **NO** null pointers - Use Option<T> for optional values
- **NO** implicit conversions - All type conversions must be explicit
- **NO** lifetime annotations - Smart pointers handle safety
- **NO** raw `&` or `*` - Use Ptr<T> and .value/.address
- **NO** tuples - Use structs for all product types

## Key Features

### ✅ Implemented
- **Functions**: `name = (params) ReturnType { }` syntax
- **Variables**: Immutable `:=` and mutable `::=` declarations
- **Pattern Matching**: Unified `?` operator for all conditionals
- **Types**: Full numeric types, strings, bool, void, arrays, structs, enums
- **Loops**: Single `loop` keyword with conditions and ranges
- **String Interpolation**: `$(expr)` syntax in strings
- **FFI Builder Pattern**: Safe C interop with validation
- **LSP Server**: Enhanced error messages with context
- **LLVM Backend**: Native code generation

### 🔧 Recent Enhancements
- **Enhanced FFI Builder**: Batch function addition, opaque types, C declaration parsing
- **Improved LSP Diagnostics**: Detailed error messages with suggestions and source context
- **Better Error Handling**: Context-aware error messages with fix suggestions

### 🚧 In Progress
- **UFCS**: Uniform Function Call Syntax for method-like calls
- **Bool Patterns**: Special syntax for boolean pattern matching
- **Compile-time Evaluation**: `comptime` blocks for metaprogramming
- **Behaviors**: Structural contracts that types can satisfy
- **Module System**: `@std` namespace and import system
- **Self-Hosting**: Compiler components being rewritten in Zen

### 📋 Roadmap
- **Memory Management**: Smart pointers (Ptr<T>, Ref<T>)
- **Colorless Async**: Allocator-based async without function coloring
- **Package Management**: Modern dependency management
- **Cross-compilation**: Multiple target platforms
- **Standard Library**: Comprehensive stdlib modules

## Self-Hosting Progress

Zen is progressively becoming self-hosted! Major compiler components are being rewritten in Zen:

### ✅ Self-Hosted Components
- **Lexer** - Complete tokenizer written in Zen
- **Parser** - Full AST generation in Zen
- **Type Checker** - Semantic analysis in Zen
- **Build System** - Project compilation orchestration
- **Standard Library** - 40+ modules written in Zen

### 🚧 Transitioning
- **Code Generator** - Moving from Rust to Zen

See [Self-Hosting Documentation](docs/SELF_HOSTING.md) for details.

## Unique Syntax

Zen has a distinctive, keyword-minimal syntax:

```zen
// No 'if' or 'else' - unified ? operator for all conditionals
score ? | 90..=100 => "A"
       | 80..=89  => "B"
       | _        => "C"

// Pattern matching with destructuring using ->
result ? | .Ok -> value => process(value)
        | .Err -> msg => handle_error(msg)

// Single 'loop' keyword for iteration
loop condition { }             // Conditional loop
loop { }                       // Infinite loop

// Clean function syntax
add = (a: i32, b: i32) i32 { a + b }

// Variable declarations
PI := 3.14159                  // Immutable
counter ::= 0                  // Mutable
```

## Building

```bash
# Build the compiler
cargo build --release

# Run tests (all should pass)
cargo test

# Compile a Zen file
cargo run --bin zen examples/hello.zen
```

## Quick Start

New to Zen? Start here:
1. **[`examples/01_basics_working.zen`](examples/01_basics_working.zen)** - Simplest working example
2. **[`examples/02_functions_working.zen`](examples/02_functions_working.zen)** - Functions and calls
3. **[`examples/WORKING_FEATURES.md`](examples/WORKING_FEATURES.md)** - What currently works
4. **[`lang.md`](lang.md)** - Full language specification (v1.0)

## Examples

The `examples/` directory contains two categories:

### Working Examples (Current Implementation)
- **`01_basics_working.zen`** - Variables and arithmetic
- **`02_functions_working.zen`** - Function definitions and calls
- **`working_hello.zen`** - Minimal working program
- **`working_variables.zen`** - Variable declarations
- **`working_loops.zen`** - Basic loops
- **`WORKING_FEATURES.md`** - Complete list of working features

### Specification Examples (Future Features) 
- **`zen_spec_showcase.zen`** - Complete language specification demonstration (NEW)
- **`zen_master_showcase.zen`** - Comprehensive feature showcase
- **`01_hello_world.zen`** - Hello world per spec
- **`02_variables_and_types.zen`** - Full variable system
- **`03_pattern_matching.zen`** - Pattern matching with `?` operator
- **`04_loops.zen`** - All loop patterns per spec
- **`05_structs_and_methods.zen`** - Structs with UFCS
- Additional examples demonstrating planned features

## Project Structure

```
zen/
├── src/
│   ├── ast.rs              # Abstract syntax tree
│   ├── parser/             # Parser implementation
│   ├── codegen/            # LLVM code generation
│   ├── typechecker/        # Type checking (WIP)
│   ├── compiler.rs         # Main compiler logic
│   └── main.rs             # CLI entry point
├── examples/               # Example Zen programs
├── tests/                  # Integration tests
├── lang.md                 # Language specification
└── .agent/                 # Project metadata
```

## Development Status

- **Parser**: ✅ Core features implemented, including `?` pattern matching syntax
- **Code Generation**: ✅ LLVM backend working for basic features
- **Type System**: 🚧 Basic type checking, improvements in progress
- **Pattern Matching**: ✅ Parser complete (src/parser/expressions.rs:373-429), 🚧 Codegen WIP
- **Module System**: 📋 Specified with `@std` namespace, not yet implemented
- **Documentation**: ✅ Complete specification in lang.md
- **Examples**: ✅ 30+ example files demonstrating current and future features
- **Naming**: ✅ Consistently "zen" throughout (no "zena" references)

## Contributing

We welcome contributions! Check out:
- [GitHub Issues](https://github.com/anthropics/zen/issues) for bug reports and features
- `ROADMAP.md` for development priorities
- `STYLE_GUIDE.md` for code style guidelines

## Language Specification

See [`lang.md`](lang.md) for the complete language specification including:
- Detailed syntax rules
- Type system
- Memory model
- Standard library design

## License

[To be determined]

## Contact

Report issues at: https://github.com/anthropics/zen/issues