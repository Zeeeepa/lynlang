# Zen Compiler Architecture Audit

**Date:** January 2026
**Perspective:** Senior Rust/LLVM Compiler Engineer

---

## Senior Systems Engineer Principles

What a senior compiler engineer looks for in a codebase:

### 1. Clear Compilation Pipeline
```
Source → Lex → Parse → Sema → Lower → Codegen → Link
```
Each phase has ONE job. Data flows forward. No phase reaches back.

### 2. No Dead Code
- Every module is imported somewhere
- Every function is called
- `#[allow(dead_code)]` is a bug report, not a solution
- If it's not used, delete it. Git remembers.

### 3. Single Source of Truth
- One place defines types
- One place declares modules
- One config, not scattered constants
- DRY applies to architecture, not just code

### 4. Separation of Concerns
- Parser: syntax only (no `if name == "Option"`)
- Typechecker: semantic analysis (all type decisions here)
- Codegen: IR generation (no type inference)
- Each layer trusts the previous layer did its job

### 5. Module Size Limits
- **< 500 LOC**: Ideal
- **500-1000 LOC**: Acceptable
- **1000-2000 LOC**: Needs splitting
- **> 2000 LOC**: Architectural smell
- **> 10000 LOC**: Emergency refactor

### 6. Error Handling
- Errors bubble up, not panic
- No `.unwrap()` in library code
- Errors carry source locations
- User sees helpful messages, not stack traces

### 7. Testing Philosophy
- Unit tests for pure functions
- Integration tests for pipelines
- No `#[allow(dead_code)]` to silence test warnings
- If you can't test it, redesign it

---

## What We Want (Target Architecture)

### Ideal Pipeline
```
┌─────────┐    ┌────────┐    ┌───────────┐    ┌──────────┐    ┌─────────┐
│  Lexer  │───▶│ Parser │───▶│Typechecker│───▶│  Lower   │───▶│ Codegen │
└─────────┘    └────────┘    └───────────┘    └──────────┘    └─────────┘
     │              │              │                │               │
   Tokens         AST        Typed AST +      Monomorphized      LLVM IR
                            Diagnostics         AST
```

### Target Module Structure
```
src/
├── main.rs              CLI only, no mod declarations
├── lib.rs               Single module registry
│
├── frontend/            < 3,000 LOC total
│   ├── lexer.rs         Tokenization
│   ├── parser/          Syntax → AST
│   └── ast/             AST definitions
│
├── sema/                < 5,000 LOC total (semantic analysis)
│   ├── typechecker/     Type inference & checking
│   ├── resolver/        Name resolution
│   └── lowering/        Generic → Concrete
│
├── codegen/             < 8,000 LOC total
│   ├── llvm/            LLVM IR generation
│   └── intrinsics/      Built-in operations
│
├── driver/              < 1,000 LOC
│   ├── compiler.rs      Pipeline orchestration
│   └── diagnostics.rs   Error formatting
│
└── tools/               Separate concerns
    ├── lsp/             Language server
    └── fmt/             Formatter
```

### Target Metrics
| Metric | Current | Target |
|--------|---------|--------|
| Total LOC | 41,322 | < 35,000 |
| Dead code | ~0 | 0 |
| `#[allow(dead_code)]` | ~130 | < 20 |
| Max module LOC | 1,023 | < 2,000 ✅ |
| Typechecker integration | ✅ Integrated | Required |

### Module Size Progress
| Module | Before | After | Target |
|--------|--------|-------|--------|
| codegen/llvm/mod.rs | 992 | 702 | < 500 |
| codegen/llvm/expressions/inference.rs | 1,023 | 1,023 | Remove (dedupe) |
| typechecker/inference.rs | 1,008 | 1,008 | Keep (single source) |

---

## Executive Summary

| Metric | Before | After Cleanup |
|--------|--------|---------------|
| Total Rust files | 146 | ~135 |
| Total LOC | 43,795 | 41,292 |
| Dead code modules | 2 | 0 ✅ |
| codegen/ LOC | 12,752 | 11,691 ✅ |
| `#[allow(dead_code)]` | 165 | 133 (32 false positives removed) |

**Status:**
- Deleted ~2,500 LOC of dead/duplicate code
- Typechecker integrated into main pipeline ✅
- Collections now use stdlib Zen (not hardcoded Rust) ✅
- Still need to audit 151 `#[allow(dead_code)]` markers

---

## Current Architecture

```
src/
├── main.rs              (369 LOC)  Entry point, REPL, CLI
├── lib.rs               (16 LOC)   Module exports
├── compiler.rs          (422 LOC)  Orchestrator
├── lexer.rs             (686 LOC)  Tokenization
├── error.rs             (616 LOC)  Error types
├── well_known.rs        (345 LOC)  Built-in type registry
├── stdlib_types.rs      (314 LOC)  Stdlib type parsing
├── intrinsics.rs        (295 LOC)  Compiler intrinsics
├── formatting.rs        (482 LOC)  Code formatter
│
├── ast/                 (843 LOC)  Abstract Syntax Tree
├── parser/              (5,949 LOC) Parser + expressions
├── typechecker/         (4,226 LOC) Type checking
├── type_system/         (1,152 LOC) Monomorphization
├── codegen/             (11,691 LOC) LLVM backend ✅ reduced from 12,752
├── lsp/                 (12,338 LOC) Language Server
├── module_system/       (475 LOC)  Module resolution
├── comptime/            (660 LOC)  Compile-time evaluation
└── bin/                 (400 LOC)  Additional binaries
```

---

## Dead Code Modules (RESOLVED ✅)

### 1. `src/ffi/` - 1,455 LOC - **DELETED**

Was a comprehensive FFI builder system that was never integrated:
- Zero imports anywhere in codebase
- Had tests but code was orphaned

### 2. `src/behaviors/` - ~400 LOC - **DELETED**

Orphaned behavior system implementation, superseded by:
- `typechecker/behaviors.rs`
- `codegen/llvm/behaviors.rs`
- `parser/behaviors.rs`

**Total cleanup:** 1,855 LOC removed

---

## HIGH: Excessive Dead Code Markers

31 files contain `#[allow(dead_code)]` with 151 total instances.

### Worst Offenders

| File | Count | Notes |
|------|-------|-------|
| `ast/expressions.rs` | 20 | AST node variants |
| `error.rs` | 19 | Error variants |
| `typechecker/behaviors.rs` | 16 | Behavior system |
| `module_system/resolver.rs` | 11 | Module resolver |
| `type_system/environment.rs` | 9 | Type env |
| `compiler.rs` | 8 | Compiler methods |
| `typechecker/types.rs` | 8 | Type helpers |

**Analysis:**
- Some `#[allow(dead_code)]` is legitimate (AST variants, error types)
- Many indicate abandoned/incomplete features
- Some indicate public API not yet used internally

---

## MEDIUM: Architectural Issues

### 1. Module Declaration Duplication

`main.rs` declares modules locally AND imports from `zen::`:

```rust
// main.rs
mod ast;           // Local declaration
mod codegen;
// ...
use zen::compiler::Compiler;  // Also imports from lib
use zen::error::{CompileError, Result};
```

This creates potential for divergence between binary and library.

**Fix:** Remove local `mod` declarations from main.rs, use only `use zen::*`

### 2. Compilation Pipeline Fragmentation

Current flow:
```
Source → Lexer → Parser → [Typechecker?] → Monomorphizer → LLVM Codegen
                              ↑
                         (bypassed!)
```

The typechecker exists (4,226 LOC) but the main compilation path in `compiler.rs`
doesn't invoke it! Type checking happens ad-hoc in codegen.

**Evidence:**
```rust
// compiler.rs - NO typechecker call!
pub fn compile_llvm(&self, program: &Program) -> Result<String> {
    let processed_program = self.process_imports(program)?;
    let processed_program = self.execute_comptime(processed_program)?;
    let processed_program = self.resolve_self_types(processed_program)?;
    let monomorphized_program = monomorphizer.monomorphize_program(&processed_program)?;
    // WHERE IS TYPECHECKER?
    let mut llvm_compiler = LLVMCompiler::new(self.context);
    llvm_compiler.compile_program(&monomorphized_program)?;
}
```

### 3. Type System Module Isolation

`type_system/` (1,152 LOC) only exports `Monomorphizer`. The rest is:
- `environment.rs` - 9 `#[allow(dead_code)]`
- `instantiation.rs` - 7 `#[allow(dead_code)]`

These were designed but never fully integrated.

### 4. Comptime Module (660 LOC)

Lightly used (3 references). Contains substantial interpreter code that may be
over-engineered for current usage.

---

## Module Usage Analysis

| Module | LOC | Used By | Status |
|--------|-----|---------|--------|
| `codegen/` | 12,752 | compiler, LSP | ✅ Active (but too big) |
| `lsp/` | 12,338 | zen-lsp binary | ✅ Active (but too big) |
| `parser/` | 5,949 | compiler, LSP | ✅ Active |
| `typechecker/` | 4,226 | compiler, LSP | ✅ Now integrated! |
| `type_system/` | 1,152 | compiler (partial) | ⚠️ Much dead code |
| `ast/` | 843 | Everyone | ✅ Active |
| `comptime/` | 660 | compiler | ⚠️ Light use |
| `module_system/` | 475 | compiler, LSP | ✅ Active |

---

## What Good Architecture Looks Like

### Ideal Compiler Pipeline

```
┌─────────┐    ┌────────┐    ┌─────────────┐    ┌──────────────┐    ┌─────────┐
│  Lexer  │───▶│ Parser │───▶│ TypeChecker │───▶│ Monomorphize │───▶│ Codegen │
└─────────┘    └────────┘    └─────────────┘    └──────────────┘    └─────────┘
     │              │               │                   │                │
     ▼              ▼               ▼                   ▼                ▼
  Tokens          AST         Typed AST          Concrete AST      LLVM IR
                             + Errors            (no generics)
```

### Principles Violated

1. **Single Responsibility**: Codegen does type inference
2. **Dependency Inversion**: Hard-coded module references
3. **Interface Segregation**: Giant modules (12K LOC codegen)
4. **Dead Code Elimination**: 2,607 LOC of unused code
5. **Pipeline Clarity**: Typechecker bypassed in main flow

---

## Recommended Actions

### Immediate (Do Now)

1. **Delete `src/ffi/`** - 1,455 LOC of dead code
2. **Audit `#[allow(dead_code)]`** - Remove truly dead code, justify rest
3. **Fix main.rs module declarations** - Use library imports only

### Short-Term (This Week)

4. **Integrate typechecker into pipeline** - Call before monomorphization
5. **Audit type_system module** - Either use or remove
6. **Document why comptime is 660 LOC** - Justify or simplify

### Medium-Term (This Month)

7. **Split codegen/** - 12,752 LOC is too large
8. **Split lsp/** - 12,338 LOC is too large
9. **Create clear phase boundaries** - Parse → Check → Lower → Emit

---

## Files Deleted (This Session)

```bash
# ✅ DONE: Dead FFI module (1,455 LOC)
rm -rf src/ffi/

# ✅ DONE: Dead behaviors module (~400 LOC)
rm -rf src/behaviors/

# ✅ DONE: Removed from lib.rs
```

**Total removed:** ~1,855 LOC of dead code

## Files to Audit

Priority order for dead code audit:

1. `src/typechecker/behaviors.rs` (16 dead_code)
2. `src/module_system/resolver.rs` (11 dead_code)
3. `src/type_system/environment.rs` (9 dead_code)
4. `src/compiler.rs` (8 dead_code)
5. `src/type_system/monomorphization.rs` (7 dead_code)
6. `src/type_system/instantiation.rs` (7 dead_code)
7. `src/module_system/mod.rs` (7 dead_code)
8. `src/comptime/mod.rs` (6 dead_code)

---

## Summary

The codebase has accumulated technical debt in the form of:
- Orphaned modules (ffi, old behaviors)
- Bypassed systems (typechecker)
- Excessive dead code markers
- Module duplication patterns

Immediate cleanup of dead code would reduce codebase by ~1,500 LOC (3.4%) with
zero functionality loss.

A proper architecture would have:
- Clear pipeline: Lex → Parse → **Type** → Mono → Codegen
- No orphaned modules
- Minimal `#[allow(dead_code)]`
- Single source of truth for module declarations
