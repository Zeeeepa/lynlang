# Zenlang Codebase Cleanup Plan

**Generated**: January 2026
**Total Lines Analyzed**: 42,807 lines across 80+ Rust files

---

## Executive Summary

| Metric | Count | Severity |
|--------|-------|----------|
| Files over 700 lines | 10 | CRITICAL |
| Total `.clone()` calls | 1,115 | HIGH |
| `.to_string()` calls | 963 | HIGH |
| TODO/FIXME comments | 21+ | MEDIUM |
| Redundant code blocks | 8+ | MEDIUM |
| Magic numbers/strings | 15+ | LOW-MEDIUM |

---

## Tier 1: Critical (Do First)

### 1.1 Split `parser/statements.rs` (1,358 lines)

**Problem**: Monolithic statement parser violates Single Responsibility Principle.

**Action**: Split into focused modules:
- `parser/statements/mod.rs` - Exports and common utilities
- `parser/statements/declarations.rs` - Variable, const, type declarations
- `parser/statements/control_flow.rs` - If, match, loops, break, continue
- `parser/statements/functions.rs` - Function/method declarations
- `parser/statements/imports.rs` - Import/export statements

**Target**: Each file under 400 lines.

---

### 1.2 Consolidate Type Inference Logic

**Problem**: Duplicate type inference in two locations:
- `src/typechecker/inference.rs` (1,041 lines)
- `src/codegen/llvm/expressions/inference.rs` (1,041 lines)

The codegen file even documents this as technical debt (lines 3-14).

**Action**:
1. Make typechecker inference the canonical source
2. Have codegen query TypeContext instead of re-inferring
3. Remove redundant fallback checks in codegen (lines 151-154 duplicate 103-108)

**Files to modify**:
- `src/codegen/llvm/expressions/inference.rs` - Remove duplicate logic
- `src/typechecker/inference.rs` - Ensure complete coverage

---

### 1.3 Extract LSP Helper Functions

**Problem**: Same patterns repeated 50+ times across LSP files:

```rust
// Pattern A: Lock handling
match store.lock() {
    Ok(s) => s,
    Err(_) => return null_response(&req),
}

// Pattern B: Range creation
fn make_range(...) -> lsp_types::Range { ... }
fn dummy_range() -> lsp_types::Range { ... }
```

**Action**: Create `src/lsp/utils.rs`:
```rust
pub fn with_document_store<F, R>(store: &Arc<Mutex<DocumentStore>>, req: &Request, f: F) -> Response
pub fn make_range(start: Position, end: Position) -> lsp_types::Range
pub fn dummy_range() -> lsp_types::Range
pub fn position_to_byte_offset(content: &str, pos: Position) -> Option<usize>
pub fn success_response<T: Serialize>(req: &Request, result: T) -> Response
pub fn null_response(req: &Request) -> Response
```

**Files affected**:
- `src/lsp/document_store.rs` lines 20-78
- `src/lsp/server.rs` lines 40-113
- `src/lsp/hover/mod.rs`
- `src/lsp/completion.rs`
- Multiple navigation files

---

## Tier 2: High Impact (Next Sprint)

### 2.1 Refactor Document Store (989 lines)

**Problem**: `src/lsp/document_store.rs` has too many responsibilities:
- Document management
- Symbol indexing
- AST caching
- Compilation integration
- Stdlib resolution
- Formatting

**Action**: Split into focused components:
- `DocumentManager` - Document lifecycle, content tracking
- `SymbolIndexer` - Symbol extraction and indexing
- `AnalysisCache` - AST and type caching
- `CompilerBridge` - Compilation integration

**Target**: Each component under 300 lines.

---

### 2.2 Reduce Clone Count in Codegen

**Problem**: 224 `.clone()` calls in `src/codegen/llvm/` alone. `AstType` is a large enum with String fields—cloning entire type trees is wasteful.

**Hot spots**:
- `behaviors.rs` - Type cloning on every behavior codegen
- `types.rs` - Conversion functions clone types in loops
- `expressions/inference.rs` - Type lookups clone results

**Action**:
1. Audit each `.clone()` call—most can use `&AstType`
2. Cache type lookups instead of re-inferring
3. Consider `Rc<AstType>` for widely-shared types
4. Replace `type_args[i].clone()` loops with references

**Expected improvement**: 10-20% performance in type-heavy code paths.

---

### 2.3 Split Large Codegen Files

**Files exceeding healthy limits**:

| File | Lines | Action |
|------|-------|--------|
| `expressions/utils.rs` | 913 | Split: type conversion, generic tracking, AST utilities |
| `stdlib_codegen/compiler.rs` | 908 | Split by category: arithmetic, bitwise, atomic, syscalls |
| `functions/calls.rs` | 788 | Extract: type building, argument conversion, generic tracking |

---

### 2.4 Consolidate Error Handling Patterns

**Problem**: Inconsistent error handling across LSP:
- Some use `try_parse_params(&req)`
- Some use `serde_json::from_value()` directly
- Some have custom match patterns

**Action**: Standardize on helper functions:
```rust
fn parse_params<T: DeserializeOwned>(req: &Request) -> Result<T, Response>
fn handle_lsp_error(req: &Request, error: impl std::error::Error) -> Response
```

---

## Tier 3: Important (Polish)

### 3.1 Address Critical TODOs

| Location | Issue | Priority |
|----------|-------|----------|
| `typechecker/behaviors.rs:316` | Missing requirement storage for enum variants | HIGH |
| `typechecker/inference.rs:238` | Need proper stdlib module registry | HIGH |
| `typechecker/mod.rs:921` | Field type extraction incomplete | HIGH |
| `codegen/llvm/pointers.rs:185` | Hardcoded i64 for usize (non-portable) | MEDIUM |
| `parser/behaviors.rs:448` | Generic type parameters unsupported | MEDIUM |

---

### 3.2 Fix Magic Numbers

**Syscall dispatch table** (`stdlib_codegen/compiler.rs:759-765`):
```rust
// BEFORE: Magic constraint strings
match arg_count {
    0 => ("syscall", "={rax},{rax},~{rcx},~{r11},~{memory}", 1),
    // ...
}

// AFTER: Named constants
const SYSCALL_CONSTRAINTS: &[SyscallConstraint] = &[
    SyscallConstraint { asm: "syscall", constraint: "...", arg_count: 1 },
    // ...
];
```

**Bit width matching** (`patterns.rs:306-307`):
```rust
// BEFORE: Magic numbers
match bit_width { 1 => AstType::Bool, 32 => AstType::I32, ... }

// AFTER: Named constants
const BOOL_BIT_WIDTH: u32 = 1;
const I32_BIT_WIDTH: u32 = 32;
```

---

### 3.3 Simplify Complex Conditionals

**Parser lookahead** (`expressions/primary.rs:58-96`):

39 lines with 5 saved state variables for Vec<T> constructor detection.

**Action**: Extract helper function:
```rust
fn detect_collection_constructor_type(
    parser: &mut Parser,
    name: &str,
) -> CollectionConstructorType {
    // Encapsulate lookahead logic
}
```

**Document store symbol extraction** (`document_store.rs:813-983`):

170 lines of nested conditionals. Extract smaller functions:
```rust
fn extract_variable_symbols(stmt: &Statement, ...) -> Vec<Symbol>
fn extract_function_symbols(stmt: &Statement, ...) -> Vec<Symbol>
fn extract_struct_symbols(stmt: &Statement, ...) -> Vec<Symbol>
```

---

### 3.4 Remove Dead Code

Files with `#[allow(dead_code)]` at module level:
- `src/typechecker/mod.rs:28`
- `src/typechecker/inference.rs:29`
- `src/codegen/llvm/mod.rs:1` (module-wide!)

**Action**:
1. Remove dead_code allowances
2. Run `cargo build` with warnings enabled
3. Delete or mark unused items as `pub(crate)` if needed internally

---

### 3.5 Improve Module Organization

**Current LSP structure** (inconsistent):
```
lsp/
├── hover/           # Submodule
├── navigation/      # Submodule
├── completion.rs    # Top-level (why not submodule?)
├── code_action.rs   # Top-level
└── ...
```

**Proposed structure**:
```
lsp/
├── core/
│   ├── server.rs
│   ├── document_store.rs
│   └── utils.rs
├── features/
│   ├── hover/
│   ├── completion/
│   ├── navigation/
│   ├── semantic_tokens/
│   ├── actions/
│   └── formatting/
└── types/
```

---

## Code Duplication Hotspots

### Pattern 1: Type Lookup with Multiple Fallbacks

**Location**: `codegen/llvm/expressions/inference.rs:101-169`

Currently tries 8+ sources with redundant checks. Consolidate into:
```rust
fn lookup_function_return_type(
    compiler: &Compiler,
    name: &str,
) -> Result<AstType, CompileError> {
    // Single priority-ordered lookup chain
    // 1. TypeContext
    // 2. function_types map
    // 3. StdlibTypeRegistry
    // 4. Intrinsics
    // 5. Error (not silent I32 fallback)
}
```

### Pattern 2: Generic Type Tracking

Three different mechanisms:
- `generic_type_context` HashMap in `mod.rs`
- `track_result_types()` in `expressions/utils.rs`
- `track_generic_return_type()` in `functions/calls.rs`

**Action**: Unify into single `GenericTypeTracker` with consistent API.

---

## Performance Improvements

### 1. Cache StdlibTypeRegistry

**Location**: `codegen/llvm/expressions/inference.rs:114-117`

```rust
// BEFORE: Creates new registry on every call
let registry = crate::stdlib_types::stdlib_types();

// AFTER: Use lazy_static or once_cell
static STDLIB_REGISTRY: Lazy<StdlibTypeRegistry> = Lazy::new(|| {
    crate::stdlib_types::stdlib_types()
});
```

### 2. Reduce String Operations in Hot Paths

**Location**: `codegen/llvm/expressions/inference.rs:136-148`

```rust
// BEFORE: Multiple string ops per function call
if name.contains('<') && name.contains('>') && !name.starts_with("compiler.") ...

// AFTER: Parse once into structured data
enum FunctionName {
    Generic { base: String, type_args: Vec<AstType> },
    Intrinsic { module: &'static str, method: String },
    Plain(String),
}
```

---

## Validation Checklist

Before marking each item complete:

- [ ] All tests pass (`cargo test`)
- [ ] No new warnings (`cargo build`)
- [ ] No increase in clone/to_string count (use `grep -c` to verify)
- [ ] File line counts reduced (target: <500 lines per file)
- [ ] Public API unchanged (or migration documented)

---

## Progress Tracking

| Item | Status | Date | Notes |
|------|--------|------|-------|
| 1.1 Split statements.rs | TODO | | |
| 1.2 Consolidate inference | TODO | | |
| 1.3 LSP helpers | TODO | | |
| 2.1 Refactor DocumentStore | TODO | | |
| 2.2 Reduce clones | TODO | | |
| 2.3 Split codegen files | TODO | | |
| 2.4 Error handling | TODO | | |
| 3.1 Address TODOs | TODO | | |
| 3.2 Fix magic numbers | TODO | | |
| 3.3 Simplify conditionals | TODO | | |
| 3.4 Remove dead code | TODO | | |
| 3.5 Module organization | TODO | | |
