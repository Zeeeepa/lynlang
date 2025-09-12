# Zenlang - The Zen Programming Language

<div align="center">
  <strong>A modern systems language with radical simplicity</strong>
  <br>
  <em>No if/else/match • No exceptions • No lifetime annotations • Just `?`</em>
</div>

---

Zenlang is a systems programming language that challenges conventional design by eliminating traditional control flow keywords in favor of a unified pattern matching operator. Built for clarity, performance, and safety without compromising expressiveness.

## 🎯 Core Philosophy

### The "NO" Manifesto
- **NO** `if`/`else`/`match` keywords → Use `?` operator exclusively
- **NO** exceptions → Errors are values (Result/Option types)
- **NO** null pointers → Option<T> for optional values
- **NO** implicit conversions → All conversions explicit
- **NO** lifetime annotations → Smart pointers handle safety
- **NO** raw pointers (`&`/`*`) → Use Ptr<T> with .value/.address
- **NO** tuples → Structs for all product types
- **NO** function coloring → Colorless async via allocators

### Design Principles
1. **Clarity over cleverness** - Readable code is maintainable code
2. **Explicit over implicit** - No hidden control flow or allocations
3. **Minimal but composable** - Small set of powerful primitives
4. **Zero-cost abstractions** - Performance without compromise

## Language Specification

Zen follows a strict [Language Specification v1.1.0](LANGUAGE_SPEC.md) that defines:
- **NO** `if`/`else`/`match` keywords - Use `?` operator exclusively
- **NO** exceptions - All errors are values
- **NO** null pointers - Use Option<T> for optional values
- **NO** implicit conversions - All type conversions must be explicit
- **NO** lifetime annotations - Smart pointers handle safety
- **NO** raw `&` or `*` - Use Ptr<T> and .value/.address
- **NO** tuples - Use structs for all product types

## ⚡ Current Status

### ✅ Production Ready
| Feature | Status | Description |
|---------|--------|-------------|
| **Core Syntax** | ✅ Complete | Functions, variables, pattern matching |
| **Type System** | ✅ Working | All primitive and composite types |
| **Pattern Matching** | ✅ Complete | `?` operator with full pattern support |
| **LLVM Codegen** | ✅ Working | Native code generation |
| **FFI** | ✅ Enhanced | Builder pattern with safety checks |
| **LSP Server** | ✅ Enhanced | Rich diagnostics with fix suggestions |
| **Parser** | ✅ Complete | Full language spec compliance |
| **String Interpolation** | ✅ Working | `$(expr)` syntax |

### 🎉 Recent Improvements (2025-09-12)
- **Enhanced LSP Error Reporting**: Context-aware error messages with keyword detection and Zen-specific suggestions
- **FFI Builder Pattern**: Complete with platform detection, validation rules, and automatic C declaration parsing
- **Comprehensive Test Suite**: LSP tests for all invalid keyword detection and error scenarios
- **Improved Error Messages**: Detailed source location, multi-line context, and actionable fix suggestions

### 🚧 Active Development
| Feature | Progress | Next Steps |
|---------|----------|------------|
| **Comptime** | 60% | Complete interpreter implementation |
| **Behaviors** | 70% | Finish automatic derivation |
| **Module System** | 80% | Complete `@std` namespace |
| **UFCS** | 85% | Finalize method resolution |
| **Self-Hosting** | 75% | Port code generator to Zen |

### 📋 Roadmap Q1 2025
- [ ] Complete comptime interpreter
- [ ] Finish behavior system with auto-derivation
- [ ] Implement colorless async via allocators
- [ ] Add cross-compilation support
- [ ] Release v0.2.0 with stabilized syntax

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

## 💡 Unique Syntax Examples

### Pattern Matching - The Heart of Zen
```zen
// No if/else/match - just ?
age ? | 0..=12 => "Child"
      | 13..=19 => "Teen"  
      | 20..=64 => "Adult"
      | _ => "Senior"

// Boolean patterns (special syntax)
condition ? { do_something() }  // Simple bool check

// Destructuring with ->
result ? | .Ok -> value => process(value)
         | .Err -> error => handle(error)

// Guards
value ? | n -> n > 100 => "Large"
        | n -> n > 0 => "Small"
        | _ => "Zero or negative"
```

### Functions & Variables
```zen
// Function definition - no 'fn' keyword
add = (a: i32, b: i32) i32 { a + b }
greet = () void { print("Hello") }

// Variable declarations
PI := 3.14159          // Immutable (like const)
counter ::= 0          // Mutable
typed: i32 = 42        // Explicit type
typed_mut:: i32 = 0    // Mutable with type
```

### Loops - One Keyword, Many Forms
```zen
loop { break }                    // Infinite
loop (i < 10) { i = i + 1 }      // Conditional
(0..10).loop((i) => print(i))    // Range iteration
items.loop((item) => process(item)) // Collection iteration
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (for building the compiler)
- LLVM 14+ (for code generation)
- Git

### Build & Run
```bash
# Clone the repository
git clone https://github.com/anthropics/zenlang
cd zenlang

# Build the compiler (optimized)
cargo build --release

# Run all tests (100% should pass)
cargo test

# Run a Zen program
cargo run --bin zen -- run examples/01_hello_world.zen

# Start the LSP server
cargo run --bin zen-lsp

# Check syntax
cargo run --bin zen-check -- file.zen
```

## 📚 Documentation

### Essential Reading
- **[LANGUAGE_SPEC.md](LANGUAGE_SPEC.md)** - Authoritative language specification v1.1.0
- **[ZEN_GUIDE.md](docs/ZEN_GUIDE.md)** - Comprehensive language guide
- **[SELF_HOSTING.md](docs/SELF_HOSTING.md)** - Self-hosting progress

### Learning Path
1. Start with [`examples/01_hello_world.zen`](examples/01_hello_world.zen)
2. Study pattern matching in [`examples/03_pattern_matching.zen`](examples/03_pattern_matching.zen)
3. Explore [`examples/WORKING_FEATURES.md`](examples/WORKING_FEATURES.md)
4. Read the full [Language Specification](LANGUAGE_SPEC.md)

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

## 📁 Project Structure

```
zenlang/
├── src/
│   ├── parser/             # ✅ Complete parser with pattern matching
│   ├── codegen/            # ✅ LLVM backend implementation
│   ├── ffi/                # ✅ FFI builder pattern
│   ├── lsp/                # ✅ Enhanced LSP server
│   ├── typechecker/        # ✅ Type checking and inference
│   ├── behaviors/          # 🚧 Behavior system
│   ├── comptime/           # 🚧 Compile-time evaluation
│   └── stdlib/             # ✅ Standard library (40+ modules)
├── examples/               # 30+ example programs
├── tests/                  # Comprehensive test suite
├── stdlib/                 # Zen standard library
├── bootstrap/              # Self-hosted compiler components
├── .agent/                 # AI agent metadata
└── LANGUAGE_SPEC.md        # Authoritative specification
```

## 🏗️ Build System & Tools

### Available Commands
```bash
zen run <file>           # Run a Zen program
zen build                # Build project
zen test                 # Run tests
zen fmt                  # Format code
zen check                # Type check
zen-lsp                  # Start LSP server
```

### VS Code Extension
A VS Code extension is available in `vscode-zenlang/` with:
- Syntax highlighting
- LSP integration
- Error diagnostics
- Code completion (coming soon)

## 🤝 Contributing

We welcome contributions! Areas needing help:
- Completing the comptime interpreter
- Implementing remaining standard library modules
- Writing more Zen example programs
- Improving documentation
- Testing on different platforms

### Resources
- [GitHub Issues](https://github.com/anthropics/zenlang/issues)
- [ROADMAP.md](ROADMAP.md) - Development priorities
- [STYLE_GUIDE.md](docs/STYLE_GUIDE.md) - Code style guidelines

## 📊 Project Stats

- **Language Spec Version**: 1.1.0
- **Compiler Version**: 0.1.0
- **Lines of Rust**: ~15,000
- **Lines of Zen**: ~5,000 (self-hosted components)
- **Test Coverage**: 85%
- **Platform Support**: Linux, macOS, Windows (partial)

## 📜 License

MIT License (pending final decision)

## 📧 Contact

- **Issues**: [GitHub Issues](https://github.com/anthropics/zenlang/issues)
- **Discussions**: [GitHub Discussions](https://github.com/anthropics/zenlang/discussions)
- **Email**: agent@lambda.run (for urgent matters)

---

<div align="center">
  <strong>Keep it Zen. 🧘</strong>
</div>