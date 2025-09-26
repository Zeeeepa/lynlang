# Zen Programming Language Architecture

## Overview
Zen is a statically-typed, compiled programming language that emphasizes memory safety without garbage collection. It compiles to native code through LLVM and features a sophisticated type system with generics, pattern matching, and explicit memory management.

## Core Design Principles

1. **No Garbage Collection** - All memory is explicitly managed through allocators
2. **Zero-Cost Abstractions** - High-level features compile to efficient machine code
3. **Type Safety** - Strong static typing with inference
4. **Expression-Oriented** - Everything is an expression that returns a value
5. **Explicit Over Implicit** - Clear semantics, no hidden allocations

## Compiler Architecture

### High-Level Pipeline

```
Source Code (.zen)
      │
      ▼
┌─────────────┐
│   Lexer     │ ─── Tokenization
└──────┬──────┘     (lexer.rs)
       │
       ▼
┌─────────────┐
│   Parser    │ ─── AST Construction
└──────┬──────┘     (parser/)
       │
       ▼
┌─────────────┐
│Module System│ ─── Import Resolution
└──────┬──────┘     (module_system/)
       │
       ▼
┌─────────────┐
│Type Checker │ ─── Type Validation
└──────┬──────┘     (typechecker/)
       │
       ▼
┌─────────────┐
│Monomorphizer│ ─── Generic Instantiation
└──────┬──────┘     (type_system/)
       │
       ▼
┌─────────────┐
│LLVM Codegen │ ─── IR Generation
└──────┬──────┘     (codegen/llvm/)
       │
       ▼
┌─────────────┐
│   LLVM      │ ─── Optimization & Machine Code
└─────────────┘
```

### Component Details

#### 1. Lexer (src/lexer.rs)
- **Purpose**: Converts source text into tokens
- **Key Features**:
  - Unicode support
  - String interpolation
  - Numeric literal parsing (int, float, hex, binary)
  - Comment handling
- **Output**: Stream of tokens with location information

#### 2. Parser (src/parser/)
- **Purpose**: Builds Abstract Syntax Tree (AST) from tokens
- **Architecture**: Recursive descent parser
- **Modules**:
  - `statements.rs` - Statement parsing
  - `expressions.rs` - Expression parsing
  - `types.rs` - Type annotation parsing
  - `patterns.rs` - Pattern matching parsing
- **Key Features**:
  - Error recovery
  - Precedence climbing for operators
  - Support for UFC (Universal Function Call) syntax

#### 3. AST (src/ast/)
- **Purpose**: Intermediate representation of program structure
- **Components**:
  - `declarations.rs` - Top-level declarations (functions, structs, enums)
  - `statements.rs` - Control flow and variable declarations
  - `expressions.rs` - Values and operations
  - `types.rs` - Type representations
  - `patterns.rs` - Pattern matching constructs

#### 4. Module System (src/module_system/)
- **Purpose**: Handle imports and module resolution
- **Components**:
  - `mod.rs` - Module loading and caching
  - `resolver.rs` - Symbol resolution and export validation
- **Features**:
  - File-based modules
  - Built-in standard library
  - Export/import validation
  - Module merging

#### 5. Type System (src/type_system/)
- **Purpose**: Type checking and generic resolution
- **Key Types**:
  - Primitives: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool
  - Compound: Arrays, Tuples, Structs, Enums
  - Generic: Type parameters with constraints
  - Special: Option<T>, Result<T,E>
- **Monomorphization**: Generates concrete types from generics

#### 6. Type Checker (src/typechecker/)
- **Purpose**: Validate type correctness
- **Features**:
  - Type inference
  - Generic constraint checking
  - Pattern exhaustiveness
  - Method resolution
- **Files**:
  - `mod.rs` - Main type checking logic
  - `validation.rs` - Additional validation rules

#### 7. LLVM Code Generation (src/codegen/llvm/)
- **Purpose**: Generate LLVM IR from typed AST
- **Architecture**:
  - `mod.rs` - Main code generator
  - `types.rs` - Type mapping to LLVM
  - `expressions.rs` - Expression compilation
  - `statements.rs` - Statement compilation
  - `functions.rs` - Function generation
  - `stdlib.rs` - Built-in function implementations

#### 8. Standard Library (src/stdlib/)
- **Purpose**: Core runtime functionality
- **Implementation**: Mix of Rust (compiled into compiler) and Zen
- **Key Components**:
  - Memory allocators
  - String operations
  - I/O functions
  - Math functions
  - Collection helpers

## Memory Management Architecture

### Allocator System
```zen
Allocator = struct {
    alloc: (size: u64, align: u64) Result<Ptr<void>, AllocatorError>
    free: (ptr: Ptr<void>, size: u64, align: u64) void
}
```

### Memory Safety Features
1. **Option Types** - No null pointers
2. **Result Types** - Explicit error handling
3. **RAII** - Resource cleanup through destructors
4. **Bounds Checking** - Array access validation

### Collections Memory Model
- All dynamic collections require explicit allocator
- Stack-allocated fixed arrays: `[T; N]`
- Heap-allocated dynamic arrays: `DynVec<T>`
- HashMap and HashSet with collision handling

## Type System Features

### Generics
```zen
Vec<T, size: u64> = struct {
    data: [T; size]
    len: u64
}
```

### Pattern Matching
```zen
option ?
    | Some(value) => process(value)
    | None => default_value()
```

### Universal Function Call (UFC)
```zen
"hello".to_upper().len()  // Method chaining
```

### Type Inference
```zen
x := 42        // Inferred as i32
y := 3.14      // Inferred as f64
z := [1, 2, 3] // Inferred as [i32; 3]
```

## Control Flow Architecture

### Expression-Based
Everything returns a value:
```zen
result := if condition { 10 } else { 20 }
```

### Loop Constructs
- Infinite loops with break: `loop(() { ... })`
- Range loops: `(0..10).loop((i) { ... })`
- Collection iteration: `vec.loop((item) { ... })`

### Error Propagation
```zen
value := try_operation()?  // Early return on error
```

## Compilation Phases

### Phase 1: Frontend
1. Lexical analysis
2. Syntactic parsing
3. Import resolution
4. Semantic analysis

### Phase 2: Middle-end
1. Type checking
2. Type inference
3. Generic monomorphization
4. Optimization passes

### Phase 3: Backend
1. LLVM IR generation
2. LLVM optimization
3. Machine code generation
4. Linking

## Runtime Architecture

### Entry Point
- Main function: `main = () i32 { ... }`
- Automatic initialization of runtime systems

### Memory Layout
- Stack: Local variables, function frames
- Heap: Dynamic allocations via allocators
- Static: String literals, constants
- Code: Compiled functions

### FFI (Foreign Function Interface)
- C ABI compatibility
- External function declarations
- Struct layout compatibility

## Build System Integration

### Compilation Model
- Single compilation unit per file
- Module merging before codegen
- LLVM for optimization and code generation

### Output Formats
- Native executables
- LLVM IR (for debugging)
- Object files (future)
- Libraries (future)

## Language Features Implementation

### Closures
- Capture by value
- Environment struct generation
- Function pointer with context

### String System
- Static strings: Compile-time literals
- Dynamic strings: Heap-allocated with allocator
- UTF-8 encoding
- Efficient concatenation via StringBuilder

### Comptime Evaluation
- Compile-time function execution
- Constant folding
- Type-level computation

## Error Handling Strategy

### Compile Errors
- Location tracking
- Error recovery
- Helpful diagnostics
- Type mismatch explanations

### Runtime Errors
- Panic mechanism
- Option/Result for recoverable errors
- Assert for invariants

## Optimization Strategies

### Compiler Optimizations
- Dead code elimination
- Constant propagation
- Inline expansion
- Generic specialization

### LLVM Optimizations
- Standard -O2 level optimizations
- Target-specific optimizations
- Link-time optimization (LTO)

## Future Architecture Plans

### Planned Features
1. Async/await for concurrency
2. Traits/behaviors system
3. Module packaging system
4. Incremental compilation
5. Language server protocol (LSP) full support

### Architecture Improvements
1. Parallel compilation
2. Query-based compiler architecture
3. Incremental type checking
4. Better error recovery

## Key Design Decisions

1. **LLVM Backend**: Leverages mature optimization infrastructure
2. **Monomorphization**: Simpler than trait objects, better performance
3. **Explicit Allocators**: Full control over memory, no hidden costs
4. **Expression-Oriented**: Reduces ceremony, improves composability
5. **Built-in Option/Result**: Safer error handling than exceptions

## File Structure
```
src/
├── ast/           # AST definitions
├── codegen/       # Code generation
│   └── llvm/      # LLVM backend
├── lexer.rs       # Tokenization
├── parser/        # Parsing logic
├── typechecker/   # Type validation
├── type_system/   # Type definitions
├── module_system/ # Import handling
├── stdlib/        # Built-in functions
├── compiler.rs    # Main compiler
└── main.rs        # Entry point
```