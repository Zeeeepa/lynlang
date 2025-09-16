# Zen Programming Language

A modern systems programming language that emphasizes simplicity, consistency, and safety without sacrificing performance.

## Current Implementation Status

📋 **See [STATUS.md](STATUS.md) for detailed implementation progress**

The Zen compiler is under active development. Currently implemented features include:
- Basic syntax and type system
- Function definitions and calls  
- Struct definitions and field access
- Simple control flow (if expressions, loops)
- Module imports with `@std` syntax
- LLVM-based code generation

## Vision & Goals

Zen aims to be a keyword-free systems language that achieves safety and expressiveness through:

### No Traditional Keywords
- ❌ No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- ✅ Pattern matching with `?` operator
- ✅ Uniform function call syntax (UFC)
- ✅ Everything is an expression

### Core Design Principles

#### Pattern Matching First
All control flow through the `?` operator:
```zen
// Boolean conditions
is_ready ? { start() }

// Multi-way branching  
value ?
    | true { process() }
    | false { wait() }

// Enum matching
option ?
    | Some(v) { use(v) }
    | None { default() }
```

#### No Null - Only Option Types
```zen
Option<T>: Some(T) | None

something :: Option<bool>

something = true // is the same as explicitly saying some
something = Some(true)

something ? { io.print("true") }

find_user = (id: i32) Option<User> {
    // Returns Option, never null
}
```

#### Assignment Operators
- `=` - Immutable binding with type inference
- `::=` - Mutable assignment
- `:` - Type annotation

```zen
x = 42           // Immutable, type inferred
y ::= 100        // Mutable  
z: i32 = 50      // With explicit type
```

#### Error Handling with Result
```zen
Result<T, E>: Ok(T) | Err(E)

parse_config = (text: string) Result<Config, Error> {
    // Error propagation with raise
    json = parse_json(text).raise()
    config = validate(json).raise()
    return Ok(config)
}
```

## Building & Testing

### Prerequisites
- Rust 1.70+
- LLVM 14+
- Clang

### Build
```bash
cargo build --release
```

### Run Tests
```bash
./run_tests.sh
```

### Try Examples
```bash
./target/release/zen examples/01_hello_world.zen
```

## Project Structure

```
zenlang/
├── src/           # Rust compiler implementation
│   ├── ast.rs     # Abstract syntax tree
│   ├── parser/    # Parser modules
│   ├── codegen/   # LLVM code generation
│   └── lsp/       # Language server
├── stdlib/        # Zen standard library
├── examples/      # Example Zen programs
├── tests/         # Test suite
└── LANGUAGE_SPEC.zen  # Official language specification
```

## Documentation

- [LANGUAGE_SPEC.zen](LANGUAGE_SPEC.zen) - The canonical language specification
- [STATUS.md](STATUS.md) - Current implementation status
- [docs/](docs/) - Additional documentation

## Contributing

Zen is an open-source project. Contributions are welcome! Please read the language specification first to understand the design goals.

## License

MIT License - See LICENSE file for details