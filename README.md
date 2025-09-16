# Zen Programming Language

A revolutionary systems programming language that eliminates traditional keywords in favor of pattern-first design and allocator-based async/sync behavior.

**Current Status**: Working Zen compiler v4 (`zenc4.c`) implementing core LANGUAGE_SPEC.zen features
- ✅ Core features working: variables, functions, structs, pattern matching, imports
- ✅ Compiles to C and executes successfully
- 🚧 Advanced features in progress: loops, ranges, Option types, traits

## 🎯 Core Philosophy

Zen follows the principles defined in `LANGUAGE_SPEC.zen`:
- **No traditional keywords**: No `if/else/while/for/match/async/await/impl/trait/class/interface/null`
- **Pattern matching everywhere**: All control flow via the `?` operator
- **UFC (Uniform Function Call)**: Any function can be called as a method
- **Allocator-determined behavior**: Sync/async determined by allocator, not function coloring
- **No null**: Only `Option<T>` with `.Some(T)` and `.None`
- **Explicit pointer types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>` (no `*` or `&`)
- **Only two @ symbols**: `@std` (standard library) and `@this` (current scope)

## 🚀 Quick Start

### Building the Compiler

```bash
# Build the Zen compiler v4
gcc -std=c99 -o zenc4 zenc4.c

# Compile a Zen program
./zenc4 myprogram.zen

# Run the generated executable
./output.c.out
```

### Hello World

```zen
// hello.zen
{ io } = @std

main = () void {
    io.println("Hello from Zen!")
}
```

## 📖 Language Features (Implemented)

### Imports and @std

```zen
// Destructuring import from @std
{ io } = @std

// Use imported module
io.println("Hello, World!")
```

### Variables

```zen
// Immutable variable (default)
x = 10

// Mutable variable with ::=
y ::= 20
y = y + 5  // Re-assignment

// With type annotation
z : i32 = 30      // Immutable with type
w :: i32 = 40     // Mutable with type
```

### Pattern Matching

All control flow uses the `?` operator:

```zen
// Simple boolean pattern
is_ready = true
is_ready ? {
    io.println("System is ready!")
}

// Pattern matching with branches
value = 42
is_match = value == 42
is_match ?
    | true { io.println("Value equals 42") }
    | false { io.println("Value does not equal 42") }
```

### Structs

```zen
// Define a struct
Point: {
    x:: f64,     // Mutable field
    y:: f64 = 0  // With default value
}

// Nested struct
Circle: {
    center: Point,
    radius: f64
}

// Create struct literals
p = Point { x: 10.5, y: 20.5 }
c = Circle {
    center: Point { x: 0, y: 0 },
    radius: 5.0
}
```

### Functions

```zen
// Function declaration
add = (a: i32, b: i32) i32 {
    return a + b
}

// Main function
main = () void {
    result = add(10, 20)
    io.println(result)
}
```

### Arithmetic and Comparisons

```zen
// Arithmetic operations
sum = a + b
diff = b - a
product = a * 2
quotient = b / 2

// Comparisons
is_equal = a == b
is_less = a < b
is_greater = a > b
```

## 🧪 Running Tests

```bash
# Build the compiler
gcc -std=c99 -o zenc4 zenc4.c

# Run comprehensive test suite
./zenc4 tests/zen_test_language_spec_working.zen && ./output.c.out

# Test specific features
./zenc4 tests/zen_test_mutable.zen && ./output.c.out        # Mutable variables
./zenc4 tests/zen_test_pattern_match.zen && ./output.c.out   # Pattern matching
./zenc4 tests/zen_test_struct.zen && ./output.c.out          # Structs
```

## 📊 Implementation Status

### ✅ Working Features
Core features fully implemented in `zenc4.c`:
- **Imports**: `{ io } = @std` destructuring syntax
- **Variables**: Immutable (`=`) and mutable (`::=`) declarations
- **Type Annotations**: `: type` and `:: type` syntax
- **Pattern Matching**: `?` operator with boolean conditions and branches
- **Structs**: Definition and instantiation with field initialization
- **Functions**: Declaration with parameters and return types
- **Arithmetic**: All basic operators (+, -, *, /, %)
- **Comparisons**: All comparison operators (==, !=, <, >, <=, >=)
- **Boolean Literals**: `true` and `false`
- **Number Literals**: Integer and floating-point
- **String Literals**: Basic string support
- **Comments**: Single-line `//` comments
- **io.println**: Console output from imported `io` module

### 🚧 Partially Implemented
Features that are parsed but need more work:
- **Option Types**: `Some(value)` and `None` recognized
- **Result Types**: `Ok(value)` and `Err(error)` recognized
- **Loops**: Basic `loop()` structure parsed
- **Ranges**: `(0..10)` syntax parsed
- **Enums**: Basic enum definitions parsed

### ❌ Not Yet Implemented
Advanced features from LANGUAGE_SPEC.zen:
- **String Interpolation**: `"Value: ${x}"`
- **Collection Methods**: `.loop()` for iteration
- **UFC**: Uniform Function Call (method chaining)
- **Traits**: `.implements()` and `.requires()`
- **Pointer Types**: `Ptr<>`, `MutPtr<>`, `RawPtr<>`
- **Defer**: `@this.defer()`
- **Error Propagation**: `.raise()` for Result types
- **Allocators**: GPA, AsyncPool for sync/async behavior
- **Metaprogramming**: `@meta.comptime()` and reflection
- **Concurrency**: Actor, Channel, Mutex, Atomic types
- **Generics**: Generic types and constraints
- **Module System**: Module exports and imports

## 🏗️ Project Structure

```
zenlang/
├── LANGUAGE_SPEC.zen       # Complete language specification (source of truth)
├── zenc4.c                 # Current working Zen compiler v4 ✅
├── zenc3.c                 # Previous working bootstrap compiler
├── tests/                  # Test suite
│   ├── zen_test_language_spec_working.zen  # Comprehensive demo
│   ├── zen_test_mutable.zen               # Mutable variables test
│   ├── zen_test_pattern_match.zen         # Pattern matching test
│   ├── zen_test_struct.zen                # Struct test
│   └── zen_test_*.zen                     # Other test files
├── output.c                # Generated C code from compilation
├── output.c.out           # Executable from compiled C code
└── .agent/                # Development tracking
    └── context.md         # Implementation notes
```

## 🤝 Contributing

Zen is actively being developed. The language specification in `LANGUAGE_SPEC.zen` is the definitive source for all language features. Current implementation in `zenc4.c` focuses on core features with a path toward self-hosting.

### Development Priorities
1. Complete core language features (loops, ranges, Option types)
2. Implement standard library modules
3. Build self-hosted compiler in Zen
4. Add advanced features (traits, generics, metaprogramming)

## 📜 License

This project is open source. See LICENSE file for details.

## 🔗 Resources

- [Language Specification](./LANGUAGE_SPEC.zen) - Complete language design and examples
- [Compiler Source](./zenc4.c) - Current compiler implementation
- [Test Suite](./tests/) - Working examples and tests