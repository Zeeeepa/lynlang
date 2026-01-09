# Zen Language LLVM Code Generation Architecture

This document provides a comprehensive overview of the LLVM code generation backend for the Zen programming language. Understanding this architecture is essential for extending the compiler and making informed design decisions.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Directory Structure](#directory-structure)
3. [Core Components](#core-components)
4. [Data Flow](#data-flow)
5. [Key Design Patterns](#key-design-patterns)
6. [Extension Points](#extension-points)
7. [System Language Design Principles](#system-language-design-principles)
8. [Development Checklist](#development-checklist)

---

## Architecture Overview

The LLVM codegen backend transforms Zen's Abstract Syntax Tree (AST) into LLVM IR, which is then compiled to native machine code. The architecture follows a **visitor pattern** where the compiler traverses AST nodes and emits corresponding LLVM instructions.

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐     ┌─────────────┐
│  Zen Source │────▶│    Parser    │────▶│  Typechecker │────▶│ LLVM Codegen│
│    (.zen)   │     │              │     │              │     │             │
└─────────────┘     └──────────────┘     └──────────────┘     └──────┬──────┘
                                                                      │
                                                                      ▼
                    ┌──────────────┐     ┌──────────────┐     ┌─────────────┐
                    │   Executable │◀────│     LLVM     │◀────│   LLVM IR   │
                    │              │     │   Optimizer  │     │             │
                    └──────────────┘     └──────────────┘     └─────────────┘
```

### Key Abstractions

| Component | Purpose |
|-----------|---------|
| `LLVMCompiler` | Central compiler state, owns LLVM context/module/builder |
| `SymbolTable` | Scoped symbol resolution for types/variables/functions |
| `GenericTypeTracker` | Tracks generic type instantiations across scopes |
| `Type<'ctx>` | Zen's internal representation bridging AST types to LLVM |

---

## Directory Structure

```
src/codegen/llvm/
├── mod.rs                 # Core LLVMCompiler struct and orchestration (966 LOC)
├── types.rs               # AST→LLVM type conversion (746 LOC)
├── symbols.rs             # Symbol table with scoped lookup (209 LOC)
├── generics.rs            # Generic type tracking and instantiation
├── behaviors.rs           # Behavior/trait method dispatch (697 LOC)
│
├── expressions/           # Expression compilation
│   ├── mod.rs             # compile_expression() dispatcher
│   ├── inference.rs       # Type inference engine (1023 LOC)
│   ├── literals.rs        # Integer, float, string, bool literals
│   ├── operations.rs      # Binary ops, type casts
│   ├── calls.rs           # Function/method call compilation
│   ├── enums.rs           # Enum variant construction (384 LOC)
│   ├── structs.rs         # Struct literals and field access
│   ├── patterns.rs        # Pattern matching compilation (339 LOC)
│   ├── control.rs         # If/match expressions (372 LOC)
│   ├── collections.rs     # Collection operations
│   └── utils.rs           # Shared utilities (978 LOC)
│
├── statements/            # Statement compilation
│   ├── mod.rs             # compile_statement() dispatcher
│   ├── variables.rs       # Variable decl/assignment (650 LOC)
│   ├── control.rs         # Return, loops, break/continue (246 LOC)
│   └── deferred.rs        # Defer statement execution
│
├── functions/             # Function handling
│   ├── mod.rs             # Function compilation entry point
│   ├── calls.rs           # Call site compilation (798 LOC)
│   └── decl.rs            # Function declaration/definition (386 LOC)
│
├── stdlib_codegen/        # Compiler intrinsics
│   ├── mod.rs             # Re-exports all intrinsics
│   ├── compiler.rs        # Low-level intrinsics (827 LOC)
│   ├── collections.rs     # HashMap, HashSet, DynVec (670 LOC)
│   └── helpers.rs         # Utility functions
│
├── binary_ops.rs          # Arithmetic, logical, comparison ops (434 LOC)
├── builtins.rs            # Built-in function implementations
├── control_flow.rs        # CFG construction helpers
├── literals.rs            # Literal value compilation (412 LOC)
├── pointers.rs            # Pointer operations (ref, deref, addr)
├── strings.rs             # String handling and interpolation
├── structs.rs             # Struct type compilation (549 LOC)
└── vec_support.rs         # Vector/array support (318 LOC)
```

**Total: ~12,647 lines of Rust code**

---

## Core Components

### 1. LLVMCompiler (`mod.rs`)

The central struct holding all compilation state:

```rust
pub struct LLVMCompiler<'ctx> {
    // LLVM infrastructure
    pub context: &'ctx Context,      // Owns all LLVM types
    pub module: Module<'ctx>,         // Current compilation unit
    pub builder: Builder<'ctx>,       // IR instruction builder

    // Symbol tables
    pub variables: HashMap<String, VariableInfo<'ctx>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub function_types: HashMap<String, AstType>,
    pub struct_types: HashMap<String, StructTypeInfo<'ctx>>,
    pub symbols: SymbolTable<'ctx>,

    // Compilation context
    pub current_function: Option<FunctionValue<'ctx>>,
    pub loop_stack: Vec<(BasicBlock, BasicBlock)>,  // (continue, break)
    pub defer_stack: Vec<Expression>,
    pub current_span: Option<Span>,   // For error reporting

    // Generic/comptime support
    pub generic_type_context: HashMap<String, AstType>,
    pub generic_tracker: GenericTypeTracker,
    pub comptime_evaluator: ComptimeInterpreter,

    // Well-known types (Result, Option names)
    pub well_known: WellKnownTypes,
}
```

### 2. Type System (`types.rs`)

Bridges Zen's AST types to LLVM types:

```rust
pub fn to_llvm_type(&mut self, type_: &AstType) -> Result<Type<'ctx>, CompileError>
```

Key responsibilities:
- Primitive type mapping (i8-i64, u8-u64, f32, f64, bool)
- Pointer type handling (Ptr<T>, MutPtr<T>, RawPtr<T>)
- Struct type registration and lookup
- Generic type instantiation
- On-demand stdlib struct loading via `ensure_struct_type()`

### 3. Symbol Table (`symbols.rs`)

Hierarchical scoped symbol resolution:

```rust
pub enum Symbol<'ctx> {
    Type(BasicTypeEnum<'ctx>),
    StructType(StructType<'ctx>),
    EnumType(EnumInfo<'ctx>),
    FunctionType(FunctionType<'ctx>),
    Variable(PointerValue<'ctx>),
    Function(FunctionValue<'ctx>),
}
```

Operations:
- `enter_scope()` / `exit_scope()` - Manage lexical scopes
- `insert()` - Add symbol to current scope
- `lookup()` - Search from current scope upward

### 4. Type Inference (`expressions/inference.rs`)

Infers types for expressions without explicit annotations:

```rust
pub fn infer_expression_type(
    compiler: &LLVMCompiler,
    expr: &Expression,
) -> Result<AstType, CompileError>
```

Handles:
- Literal types
- Variable lookups
- Function return types
- Method call return types
- Generic instantiation inference
- UFC (Uniform Function Call) lookups

### 5. Behavior Dispatch (`behaviors.rs`)

Routes method calls to appropriate implementations:

```rust
pub fn compile_method_call(
    &mut self,
    object: &Expression,
    method_name: &str,
    args: &[Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError>
```

Priority order:
1. @std reference methods
2. Module import methods
3. Type-specific constructors (Range.new, HashMap.new)
4. Built-in type methods (HashMap, Range)
5. Behavior/trait dispatch
6. Qualified method lookup
7. UFC fallback

---

## Data Flow

### Expression Compilation Flow

```
Expression AST
      │
      ▼
┌─────────────────────┐
│ compile_expression()│  (expressions/mod.rs)
└─────────┬───────────┘
          │ Pattern match on Expression variant
          ▼
┌─────────────────────────────────────────────────┐
│ Dispatch to specialized module:                 │
│  • literals.rs    → Literal values              │
│  • operations.rs  → Binary ops, casts           │
│  • calls.rs       → Function/method calls       │
│  • structs.rs     → Struct literals/fields      │
│  • enums.rs       → Enum variants               │
│  • patterns.rs    → Pattern matching            │
│  • control.rs     → If/match expressions        │
└─────────────────────────────────────────────────┘
          │
          ▼
    BasicValueEnum<'ctx>
    (LLVM value result)
```

### Function Call Resolution Flow

```
FunctionCall { name, args }
          │
          ▼
┌─────────────────────────────────────────────────┐
│ 1. Check collection constructors                │
│    (HashMap.new, Range.new, etc.)               │
├─────────────────────────────────────────────────┤
│ 2. Check compiler intrinsics                    │
│    (compiler.sizeof, builtin.memcpy, etc.)      │
├─────────────────────────────────────────────────┤
│ 3. Try direct call (module.get_function)        │
├─────────────────────────────────────────────────┤
│ 4. Try indirect call (function pointer)         │
├─────────────────────────────────────────────────┤
│ 5. Error: Undeclared function                   │
└─────────────────────────────────────────────────┘
```

---

## Key Design Patterns

### 1. Visitor Pattern
The compiler traverses AST nodes via pattern matching, dispatching to specialized handlers:

```rust
match expr {
    Expression::Integer32(v) => compile_i32(*v),
    Expression::BinaryOp { .. } => compile_binary_op(..),
    Expression::FunctionCall { .. } => compile_function_call(..),
    // ...
}
```

### 2. Result Propagation
All compilation functions return `Result<T, CompileError>`, enabling clean error propagation:

```rust
let left = self.compile_expression(lhs)?;
let right = self.compile_expression(rhs)?;
let result = self.builder.build_add(left, right, "add")?;
```

### 3. On-Demand Type Registration
Stdlib types are registered lazily when first encountered:

```rust
// In to_llvm_type for struct types:
self.ensure_struct_type(name)?;  // Registers from stdlib if not found
```

### 4. Span Tracking for Errors
Source locations are tracked throughout compilation:

```rust
self.set_span(span.clone());
// ... compilation ...
CompileError::TypeError(msg, self.get_current_span())
```

### 5. PHI Nodes for Control Flow
Conditional expressions use LLVM PHI nodes to merge values:

```rust
let phi = self.builder.build_phi(result_type, "if_result")?;
phi.add_incoming(&[
    (&then_value, then_block),
    (&else_value, else_block),
]);
```

---

## Extension Points

### Adding a New Type

1. **AST Definition** (`src/ast/types.rs`):
   ```rust
   pub enum AstType {
       // Add new variant
       MyNewType { /* fields */ },
   }
   ```

2. **Type Conversion** (`types.rs`):
   ```rust
   AstType::MyNewType { .. } => {
       // Return appropriate LLVM type
   }
   ```

3. **Type Inference** (`expressions/inference.rs`):
   - Add cases in `infer_expression_type()`
   - Add cases in `infer_method_call_type()` if it has methods

### Adding a New Intrinsic

1. **Metadata** (`src/stdlib_metadata/compiler.rs`):
   ```rust
   "my_intrinsic" => Some(StdFunction { /* signature */ })
   ```

2. **Codegen** (`stdlib_codegen/compiler.rs`):
   ```rust
   pub fn compile_my_intrinsic<'ctx>(
       compiler: &mut LLVMCompiler<'ctx>,
       args: &[Expression],
   ) -> Result<BasicValueEnum<'ctx>, CompileError>
   ```

3. **Dispatch** (`functions/calls.rs`):
   ```rust
   "my_intrinsic" => Some(compile_my_intrinsic(compiler, args))
   ```

### Adding a New Stdlib Method

1. **Type Inference** (`expressions/inference.rs`):
   - Add return type in `infer_common_method_type()` or type-specific handler

2. **Codegen** (`behaviors.rs` or type-specific handler):
   - Implement in `try_compile_*_method()` function

3. **Stdlib Definition** (`stdlib/*.zen`):
   - Add the method signature for documentation

---

## System Language Design Principles

When extending the Zen compiler, keep these principles in mind:

### 1. Zero-Cost Abstractions
High-level constructs should compile to optimal machine code:
- Iterators should be as fast as hand-written loops
- Generic types should monomorphize to specialized code
- Method calls should inline when possible

### 2. Explicit Memory Management
Memory behavior should be predictable and controllable:
- Clear ownership semantics (Ptr vs MutPtr vs RawPtr)
- Explicit allocation/deallocation via allocators
- No hidden heap allocations

### 3. Compile-Time Safety
Catch errors at compile time when possible:
- Strong type inference to reduce annotation burden
- Pattern matching exhaustiveness checking
- Null safety via Option<T>

### 4. Predictable Performance
Developers should understand the cost of operations:
- No hidden virtual dispatch (explicit trait objects)
- Struct layout matches C ABI for FFI
- Stack allocation preferred over heap

### 5. Progressive Complexity
Simple things should be simple, complex things should be possible:
- Safe defaults with escape hatches (RawPtr, inline_c)
- Gradual adoption of advanced features
- Clear error messages guide users

---

## Development Checklist

When implementing new features, verify:

### Type System
- [ ] AST type variant defined
- [ ] `to_llvm_type()` handles the type
- [ ] `infer_expression_type()` can infer it
- [ ] Type can be used in generics
- [ ] Sizeof returns correct value

### Expressions
- [ ] `compile_expression()` dispatch added
- [ ] Type inference works correctly
- [ ] Pattern matching supported (if applicable)
- [ ] Method calls resolve correctly

### Functions/Methods
- [ ] Function declaration compiles
- [ ] Call sites compile correctly
- [ ] Return type inference works
- [ ] Generic instantiation works

### Error Handling
- [ ] Errors include source spans
- [ ] Error messages are actionable
- [ ] Edge cases produce sensible errors

### Testing
- [ ] Unit tests for new functionality
- [ ] Integration tests with example programs
- [ ] Edge case coverage

### Documentation
- [ ] Stdlib comments for public APIs
- [ ] Example usage in tests
- [ ] Update relevant docs

---

## Quick Reference: Common Tasks

| Task | Primary Files |
|------|---------------|
| Add new expression type | `expressions/mod.rs`, `expressions/inference.rs` |
| Add new statement type | `statements/mod.rs`, appropriate submodule |
| Add intrinsic function | `stdlib_codegen/compiler.rs`, `functions/calls.rs` |
| Add collection method | `behaviors.rs`, `stdlib_codegen/collections.rs` |
| Modify type system | `types.rs`, `expressions/inference.rs` |
| Add pattern matching | `expressions/patterns.rs` |
| Add control flow | `statements/control.rs`, `control_flow.rs` |

---

## Architecture Diagrams

### Module Dependencies

```
                    ┌──────────────┐
                    │   mod.rs     │
                    │ (LLVMCompiler)│
                    └──────┬───────┘
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │expressions│    │statements│    │ functions│
    └────┬─────┘    └────┬─────┘    └────┬─────┘
         │               │               │
         ▼               ▼               ▼
    ┌─────────────────────────────────────────┐
    │              Shared modules:             │
    │  types.rs, symbols.rs, behaviors.rs,    │
    │  generics.rs, stdlib_codegen/           │
    └─────────────────────────────────────────┘
```

### Enum Representation

```
Option<i64> in LLVM:
┌────────────────────────────────┐
│ struct {                       │
│   i64 discriminant;  // 0=Some, 1=None
│   ptr payload;       // heap-allocated i64*
│ }                              │
└────────────────────────────────┘

Some(42):  { discriminant: 0, payload: &42 }
None:      { discriminant: 1, payload: null }
```

---

*Last updated: January 2026*
*Lines of code: ~12,647*
*Files: 35*
