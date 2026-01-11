# Zen Compiler Architecture

**Last Updated:** January 2026 (stdlib reorganized)

---

## Overview

Zen is a systems programming language with a Rust compiler targeting LLVM. The compiler follows a traditional pipeline architecture with clear phase boundaries.

For language syntax and features, see the [README](../README.md) and [LANGUAGE_SPEC.zen](../LANGUAGE_SPEC.zen).

---

## Compilation Pipeline

```
Source (.zen)
     │
     ▼
┌─────────┐
│  Lexer  │  lexer.rs (686 LOC)
└────┬────┘
     │ Tokens
     ▼
┌─────────┐
│ Parser  │  parser/ (3,578 LOC)
└────┬────┘
     │ AST
     ▼
═══════════════════════════════════
     SEMANTIC ANALYSIS (sema)
═══════════════════════════════════
     │
     ├─► process_imports()     Module resolution
     │
     ├─► execute_comptime()    Compile-time evaluation
     │
     ├─► resolve_self_types()  Self type resolution
     │
     ├─► typecheck()           Type checking & inference
     │
     └─► monomorphize()        Generic instantiation
═══════════════════════════════════
     │ Typed AST (no generics)
     ▼
┌──────────┐
│ Codegen  │  codegen/ (11,776 LOC)
└────┬─────┘
     │ LLVM IR
     ▼
┌──────────┐
│   LLVM   │  External (Inkwell bindings)
└────┬─────┘
     │
     ▼
  Machine Code
```

---

## Module Structure

```
src/
├── main.rs              (355 LOC)   CLI & REPL
├── lib.rs               (16 LOC)    Module exports
├── compiler.rs          (424 LOC)   Pipeline orchestration
├── lexer.rs             (696 LOC)   Tokenization
├── error.rs             (616 LOC)   Error types
├── well_known.rs        (345 LOC)   Built-in type registry
├── stdlib_types.rs      (314 LOC)   Stdlib type parsing
├── intrinsics.rs        (295 LOC)   Compiler intrinsics
├── formatting.rs        (482 LOC)   Code formatter
│
├── ast/                 (849 LOC)   Abstract Syntax Tree
│   ├── mod.rs                       Program, node definitions
│   ├── expressions.rs               Expression enum
│   ├── statements.rs                Statement enum
│   ├── declarations.rs              Function/struct/enum decls
│   ├── types.rs                     AstType enum
│   └── patterns.rs                  Pattern matching AST
│
├── parser/              (5,845 LOC) Syntax analysis
│   ├── mod.rs                       Parser struct, entry point
│   ├── core.rs                      Token consumption
│   ├── program.rs                   Top-level parsing
│   ├── statements.rs                Statement parsing
│   ├── patterns.rs                  Pattern matching
│   ├── types.rs                     Type annotations
│   ├── functions.rs                 Function declarations
│   ├── structs.rs                   Struct definitions
│   ├── enums.rs                     Enum definitions
│   ├── behaviors.rs                 Behavior definitions
│   └── expressions/                 Expression parsing
│       ├── primary.rs               Identifiers, literals
│       ├── operators.rs             Binary/unary ops
│       ├── calls.rs                 Function/method calls
│       ├── control_flow.rs          if/match/while exprs
│       └── collections.rs           Array/map literals
│
├── typechecker/         (4,262 LOC) Type checking
│   ├── mod.rs                       Main typechecker (~1000 LOC)
│   ├── inference.rs                 Type inference (~1000 LOC)
│   ├── behaviors.rs                 Behavior checking
│   ├── declaration_checking.rs      Validate declarations
│   ├── statement_checking.rs        Validate statements
│   ├── self_resolution.rs           Self type resolution
│   ├── type_resolution.rs           Resolve type names
│   ├── validation.rs                Type compatibility
│   └── scope.rs                     Scope management
│
├── type_system/         (1,152 LOC) Monomorphization
│   ├── mod.rs
│   ├── monomorphization.rs          Generic instantiation
│   ├── environment.rs               Type environment
│   └── instantiation.rs             Type instantiation
│
├── codegen/             (11,776 LOC) LLVM backend
│   └── llvm/
│       ├── mod.rs                   LLVMCompiler (~700 LOC)
│       ├── types.rs                 AstType → LLVM type
│       ├── symbols.rs               Symbol table
│       ├── behaviors.rs             Behavior dispatch
│       ├── generics.rs              Generic tracking
│       ├── binary_ops.rs            Arithmetic/logic ops
│       ├── literals.rs              Literal codegen
│       ├── patterns.rs              Pattern matching
│       ├── structs.rs               Struct layout
│       ├── pointers.rs              Pointer ops
│       ├── functions/
│       │   ├── decl.rs              Function declarations
│       │   └── calls.rs             Call site codegen
│       ├── expressions/
│       │   ├── inference.rs         Type inference (~1000 LOC)
│       │   ├── utils.rs             Utilities (~1000 LOC)
│       │   ├── enums.rs             Enum variants
│       │   ├── control.rs           If/match codegen
│       │   └── patterns.rs          Pattern codegen
│       ├── statements/
│       │   ├── variables.rs         Variable decl/assign
│       │   ├── control.rs           Return/loop/break
│       │   └── deferred.rs          Defer execution
│       └── stdlib_codegen/
│           ├── compiler.rs          Intrinsic implementations
│           └── helpers.rs           Codegen helpers
│
├── lsp/                 (12,338 LOC) Language Server
│   ├── server.rs                    Main server loop
│   ├── document_store.rs            Open documents
│   ├── completion.rs                Code completion
│   ├── inlay_hints.rs               Inline hints
│   ├── semantic_tokens.rs           Syntax highlighting
│   ├── signature_help.rs            Function signatures
│   ├── rename.rs                    Symbol renaming
│   ├── code_action.rs               Quick fixes
│   ├── call_hierarchy.rs            Call tree
│   ├── analyzer.rs                  Semantic analysis
│   ├── type_inference.rs            LSP type inference
│   ├── hover/                       Hover providers
│   └── navigation/                  Go-to-definition, refs
│
├── module_system/       (475 LOC)   Module resolution
│   ├── mod.rs
│   └── resolver.rs                  Import resolution
│
├── comptime/            (660 LOC)   Compile-time evaluation
│   └── mod.rs                       Comptime interpreter
│
└── bin/
    └── zen-lsp.rs                   LSP server binary
```

---

## Metrics

| Metric | Value |
|--------|-------|
| Total Rust files | 138 |
| Total LOC | ~41,300 |
| `#[allow(dead_code)]` markers | 130 |

### Module Sizes

| Module | LOC | Notes |
|--------|-----|-------|
| lsp/ | 12,338 | Full LSP implementation |
| codegen/ | 11,749 | LLVM backend |
| parser/ | 5,845 | Syntax analysis |
| typechecker/ | 4,280 | Type checking & inference |
| type_system/ | 1,111 | Monomorphization |
| ast/ | 849 | AST definitions |
| lexer.rs | 696 | Single-file tokenizer |
| comptime/ | 660 | Compile-time evaluation |

---

## Intrinsics vs Stdlib

The compiler provides minimal intrinsics. Everything else is in the Zen stdlib.

**Intrinsics** (in compiler, cannot be written in Zen):
- Memory: `raw_allocate`, `raw_deallocate`, `memcpy`, `memset`
- Pointers: `gep`, `gep_struct`, `ptr_to_int`, `int_to_ptr`
- Types: `sizeof<T>`, `alignof<T>`
- Atomics: `atomic_load`, `atomic_store`, `atomic_cas`, etc.
- Syscalls: `syscall0` - `syscall6`
- Enums: `discriminant`, `get_payload`, `set_payload`

**Stdlib** (written in Zen using intrinsics):
- All collections, memory allocators, sync primitives, I/O

See `docs/INTRINSICS_REFERENCE.md` for full intrinsics documentation.

---

## Phase Responsibilities

### Lexer (`lexer.rs`)
- Converts source text to tokens
- No semantic analysis
- Reports lexical errors

### Parser (`parser/`)
- Builds AST from tokens
- No type checking
- Reports syntax errors

### Typechecker (`typechecker/`)
- Type inference and checking
- Behavior implementation verification
- Self type resolution
- Reports type errors

### Monomorphizer (`type_system/`)
- Instantiates generic types with concrete types
- Creates specialized versions of generic functions
- No type inference (trusts typechecker)

### Codegen (`codegen/`)
- Generates LLVM IR from typed AST
- No type decisions (trusts previous phases)
- Implements intrinsics

---

## Standard Library Structure

```
stdlib/
├── std.zen             Entry point, re-exports
├── build.zen           Build system
├── compiler.zen        Compiler intrinsics
├── ffi.zen             Foreign function interface
├── math.zen            Math functions
├── testing.zen         Test framework
├── time.zen            Time operations
│
├── core/               Core types
│   ├── option.zen      Option<T>: Some, None
│   ├── result.zen      Result<T,E>: Ok, Err
│   ├── ptr.zen         Ptr<T>, MutPtr<T>, RawPtr<T>
│   ├── iterator.zen    Range, Iterator behavior
│   └── propagate.zen   Error propagation
│
├── collections/        All container types
│   ├── string.zen      Dynamic UTF-8 string
│   ├── vec.zen         Dynamic array Vec<T>
│   ├── char.zen        Character utilities
│   ├── hashmap.zen     HashMap<K,V>
│   ├── set.zen         Set<T>
│   ├── stack.zen       Stack<T>
│   ├── queue.zen       Queue<T>
│   └── linkedlist.zen  LinkedList<T>
│
├── memory/             Memory management
│   ├── allocator.zen   Allocator behavior
│   ├── gpa.zen         General purpose allocator
│   ├── mmap.zen        Memory-mapped regions
│   ├── async_allocator.zen  Async allocator behavior
│   └── async_pool.zen  io_uring-based allocator
│
├── concurrency/        ALL concurrency in one place
│   ├── primitives/     Low-level (atomic, futex)
│   ├── sync/           Thread-based (mutex, channel, thread, etc.)
│   ├── async/          Task-based (task, executor, scheduler)
│   └── actor/          Actor model (actor, supervisor, system)
│
├── io/                 I/O operations
│   ├── io.zen          Basic print/read
│   ├── files/          File ops (file, fs, dir, stat, link, copy, splice)
│   ├── net/            Networking (socket, unix_socket, pipe)
│   └── mux/            I/O multiplexing (epoll, poll, uring)
│
└── sys/                System interface
    ├── syscall.zen     Syscall numbers
    ├── process/        Process management (process, prctl, sched)
    ├── random/         Random (getrandom, prng)
    └── ...             (env, uname, resource, seccomp, memfd)
```

---

## LSP Features

The language server (`src/lsp/`) implements full LSP support:

- Hover with type info
- Go-to-definition (including nested member access)
- Find all references
- Code completion (`.`, `:`, `@`, `?` triggers)
- Signature help
- Document/workspace symbols
- Rename with prepare
- Folding ranges
- Inlay hints
- Call hierarchy
- Semantic tokens
- Document formatting

---

## Build & Test

```bash
# Build
cargo build --release

# Run all tests
cargo test

# Run compiler
./target/release/zen examples/hello.zen

# Run LSP
./target/release/zen-lsp
```

---

## Compiler Internals

### Well-Known Types

The compiler has special knowledge of these types (`src/well_known.rs`):

| Type | Special Handling |
|------|------------------|
| `Option<T>` | Pattern matching codegen, `?` operator |
| `Result<T,E>` | Pattern matching codegen, `?` operator |
| `Vec<T>` | Indexing, iteration |
| `String` | String interpolation, literals |
| `HashMap<K,V>` | Iteration |
| `Range` | Loop codegen |

### Key Data Structures

**AST Types** (`src/ast/`):
```rust
pub enum Expression {
    Integer32(i32),
    BinaryOp { left, op, right },
    FunctionCall { name, args, generics },
    StructLiteral { name, fields },
    Match { value, arms },
    // ... 50+ variants
}

pub enum Statement {
    Let { name, type_annotation, value },
    Return(Option<Expression>),
    If { condition, then_block, else_block },
    While { condition, body },
    // ...
}
```

**Compiler State** (`src/codegen/llvm/mod.rs`):
```rust
pub struct LLVMCompiler<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    variables: HashMap<String, VariableInfo>,
    functions: HashMap<String, FunctionValue>,
    struct_types: HashMap<String, StructTypeInfo>,

    generic_tracker: GenericTypeTracker,
    well_known: WellKnownTypes,
}
```

### Extension Points

**Adding a new intrinsic:**
1. Declare in `src/intrinsics.rs`
2. Add codegen in `src/codegen/llvm/stdlib_codegen/compiler.rs`
3. Document in `docs/INTRINSICS_REFERENCE.md`

**Adding a new AST node:**
1. Add variant to `src/ast/expressions.rs` or `src/ast/statements.rs`
2. Add parsing in `src/parser/`
3. Add type checking in `src/typechecker/`
4. Add codegen in `src/codegen/llvm/expressions/` or `statements/`

**Adding stdlib functionality:**
1. Write in Zen using existing intrinsics (`stdlib/*.zen`)
2. No compiler changes needed

---

## Related Documentation

- `docs/INTRINSICS_REFERENCE.md` - Compiler intrinsics reference
- `docs/ROADMAP_2026-01.md` - Development roadmap
- `docs/design/ARCHITECTURE.md` - Primitives vs features design
- `docs/design/STDLIB_DESIGN.md` - Stdlib API design
- `docs/QUICK_START.md` - Getting started guide
