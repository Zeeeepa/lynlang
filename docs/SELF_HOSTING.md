# Zen Self-Hosting Status

## Overview
Zen is progressively becoming a self-hosted language, with major compiler components being rewritten in Zen itself. This document tracks the progress and architecture of the self-hosting effort.

## Import Syntax
As of the latest update, Zen uses a clean module-level import syntax without requiring `comptime` blocks:

```zen
// Direct module imports
core := @std.core
io := @std.io
math := @std.math

// Build imports
build := @std.build
json := build.import("json")
```

## Self-Hosted Components

### ✅ Completed Components

#### 1. Lexer (`stdlib/compiler/lexer.zen`)
- Tokenizes Zen source code
- Handles all token types including keywords, operators, literals
- Supports string interpolation and multi-line strings
- Location tracking for error reporting

#### 2. Parser (`compiler/parser.zen`)
- Parses tokens into Abstract Syntax Tree (AST)
- Supports full Zen syntax including:
  - Pattern matching
  - Generic types
  - Traits and implementations
  - Module-level imports
  - Comptime evaluation

#### 3. Type Checker (`stdlib/compiler/type_checker.zen`)
- Performs semantic analysis
- Type inference and checking
- Generic type resolution
- Trait constraint validation
- Mutability checking

#### 4. Symbol Table (`stdlib/compiler/symbol_table.zen`)
- Manages scopes and symbol resolution
- Tracks variable declarations and types
- Handles module imports and exports

#### 5. Build System (`stdlib/build.zen`)
- Project compilation orchestration
- Module dependency resolution
- Incremental compilation support
- Cross-compilation configuration

### 🚧 In Progress

#### Code Generator
- LLVM backend integration in Zen
- Native code generation
- Optimization passes

### 📋 Planned Components

#### Optimizer
- SSA-based optimizations
- Inlining and dead code elimination
- Constant folding and propagation

#### Package Manager
- Dependency management
- Package publishing and distribution
- Version resolution

## Standard Library

The Zen standard library consists of 40+ modules, all written in Zen:

### Core Modules
- `core` - Fundamental types and operations
- `io` - Input/output operations
- `fs` - File system operations
- `mem` - Memory management
- `process` - Process control

### Data Structures
- `vec` - Dynamic arrays
- `hashmap` - Hash tables
- `set` - Set data structure
- `queue` - Queue implementation
- `stack` - Stack implementation

### Utilities
- `string` - String manipulation
- `math` - Mathematical functions
- `random` - Random number generation
- `datetime` - Date and time handling
- `regex` - Regular expressions

### Network & Web
- `net` - Network operations
- `http` - HTTP client/server
- `json` - JSON parsing and serialization

### Development Tools
- `test` - Testing framework
- `assert` - Assertion utilities
- `ast` - AST manipulation

## Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run import tests specifically
cargo test --test imports

# Run integration tests
./test_integration.sh
```

### Test Coverage
- Unit tests for lexer, parser, type checker
- Integration tests for import system
- End-to-end compilation tests
- Standard library tests

## Building and Running Zen Programs

### Using the Compiler Wrapper
```bash
# Compile a Zen file
./zenc main.zen

# Compile with optimization
./zenc main.zen -O3

# Specify output file
./zenc main.zen -o myprogram
```

### Direct Compilation
```bash
# Using cargo
cargo run --bin zen -- examples/hello.zen

# Using the built binary
./target/release/zen examples/hello.zen
```

## Known Issues

1. **LLVM Physreg Copy Error**: Some conditional expression tests fail with LLVM errors. These are temporarily ignored while being investigated.

2. **Pattern Matching**: Nested conditional patterns may not branch correctly in certain edge cases.

3. **Multiple Return Values**: Type system needs improvements for functions returning multiple values.

## Contributing

To contribute to the self-hosting effort:

1. Check the planned components list above
2. Write new components in Zen when possible
3. Ensure compatibility with existing Rust components during transition
4. Add comprehensive tests for new features
5. Update this document with progress

## Roadmap

### Phase 1 (Completed) ✅
- Basic lexer and parser in Zen
- Type checker implementation
- Standard library modules

### Phase 2 (Current) 🚧
- Code generator in Zen
- Build system improvements
- Enhanced error reporting

### Phase 3 (Future) 📅
- Complete self-hosting with Zen-written compiler
- Bootstrap process documentation
- Performance optimizations

## Architecture

```
Source Code (.zen)
        ↓
    Lexer (Zen)
        ↓
    Parser (Zen)
        ↓
  Type Checker (Zen)
        ↓
  Code Generator (Rust → Zen)
        ↓
    LLVM IR
        ↓
  Native Code
```

## Bootstrap Process

Once fully self-hosted, Zen will use a three-stage bootstrap process:

1. **Stage 0**: Use existing Rust compiler to compile Zen compiler written in Zen
2. **Stage 1**: Use Stage 0 compiler to compile itself
3. **Stage 2**: Use Stage 1 compiler to compile itself (verification)

## Performance

Current benchmarks show the Zen-written components perform within 15% of their Rust counterparts, with ongoing optimization work to close this gap.

## Resources

- [Zen Language Specification](./LANGUAGE_SPEC.md)
- [Compiler Architecture](./COMPILER_ARCH.md)
- [Contributing Guide](../CONTRIBUTING.md)