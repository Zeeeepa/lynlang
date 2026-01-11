# Refactor: Add Resolved Types to AST

**Status:** IN PROGRESS (Phase 1-4 complete)
**Priority:** High
**Estimated Effort:** 210-360 edits across 40 files (reduced with incremental approach)

---

## Current Progress

### Completed
- **Phase 1:** Added `TypedExpr` wrapper struct to AST (`src/ast/expressions.rs`)
- **Phase 2:** Extended `TypeContext` with method params, behavior impls, and helper methods
- **Phase 3:** Updated codegen to use `TypeContext` for method/enum lookups
- **Phase 4:** Added constructor and variable type tracking to TypeContext (Jan 2026)

### Changes Made
1. `src/ast/expressions.rs` - Added `TypedExpr` struct with `Deref` impl
2. `src/ast/types.rs` - Added `get_type_name()` helper method
3. `src/type_context.rs` - Extended with method params, behavior impls, constructors, variables
4. `src/typechecker/mod.rs` - Updated `build_type_context()` to populate constructors
5. `src/typechecker/behaviors.rs` - Added `implementations()` accessor
6. `src/codegen/llvm/expressions/inference.rs` - Uses TypeContext for constructors, methods, enums

### Phase 4 Details (Jan 2026)
- Added `constructors` map to TypeContext for constructor return types (new, create, with_*, from_*)
- Added `variables` map to TypeContext for scoped variable types
- Added query methods: `get_constructor_type()`, `get_variable_type()`, `has_constructor()`
- Updated codegen inference to check constructor types from TypeContext first

### Remaining Work
- Full integration of `TypedExpr` in parser (optional - can defer)
- Remove remaining duplicate inference logic (incremental)
- Consider per-expression type storage for complex contexts

---

## Problem Statement

The compiler has **duplicate type inference code**:

| Location | LOC | Purpose |
|----------|-----|---------|
| `typechecker/inference.rs` | 1,026 | Type checking phase |
| `codegen/expressions/inference.rs` | 1,042 | Code generation phase |

This duplication exists because:
1. The typechecker infers types but **doesn't store them persistently**
2. The AST doesn't carry type information after type checking
3. Codegen must re-infer types to generate correct LLVM IR

**Impact:**
- ~2,000 LOC of duplicate logic
- Performance overhead (types inferred twice)
- Risk of inconsistency between phases
- Harder to maintain

---

## Solution: Hybrid Approach (Implemented)

Instead of a massive parser refactor, we've taken an incremental approach:

1. **TypedExpr struct exists** for future use
2. **TypeContext is the primary bridge** between typechecker and codegen
3. **Codegen checks TypeContext first** before falling back to local inference

This gives us 80% of the benefit with 20% of the effort.

### Option A: Wrapper Struct (Recommended)

```rust
// src/ast/expressions.rs

/// Expression with optional resolved type information
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: Expression,
    pub resolved_type: Option<AstType>,
    pub span: Option<Span>,
}

impl TypedExpr {
    pub fn new(expr: Expression) -> Self {
        Self { expr, resolved_type: None, span: None }
    }

    pub fn with_type(expr: Expression, ty: AstType) -> Self {
        Self { expr, resolved_type: Some(ty), span: None }
    }
}
```

**Pros:**
- Minimal changes to Expression enum itself
- Can add more metadata later (span, source location)
- Clear separation of concerns

**Cons:**
- Every `Expression` usage becomes `TypedExpr`
- More verbose pattern matching

### Option B: Side Table (TypeContext Extension)

```rust
// src/type_context.rs

pub struct TypeContext {
    // ... existing fields ...

    /// Expression type cache: expr_id -> resolved_type
    pub expr_types: HashMap<ExprId, AstType>,
}

// Add unique IDs to expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(u64);
```

**Pros:**
- No AST structure changes
- Expressions remain lightweight
- Easy to implement incrementally

**Cons:**
- Requires generating unique IDs
- Extra indirection for lookups
- IDs must be stable across phases

### Recommendation: Option A (Wrapper Struct)

The wrapper approach is more idiomatic for compilers and makes type information directly accessible without HashMap lookups.

---

## Scope of Changes

### Summary

| Component | Files | Match Statements | Edits Needed |
|-----------|-------|------------------|--------------|
| AST | 2 | 0 | 5-10 |
| Parser | 6 | 0-1 | 40-50 |
| Typechecker | 4 | 3 | 50-100 |
| Codegen | 14 | 8-10 | 80-120 |
| LSP | 8 | 2-3 | 10-20 |
| Type System | 3 | 2-3 | 20-40 |
| Other | 3 | 2 | 10-20 |
| **Total** | **40** | **17-20** | **215-360** |

### Expression Variants: 56

The Expression enum has 56 variants that will need handling:

**Literals (14):** Integer8-64, Unsigned8-64, Float32/64, Boolean, String, Unit

**Composite (9):** BinaryOp, FunctionCall, StructLiteral, StructField, EnumVariant, EnumLiteral, ArrayLiteral, ArrayIndex, MemberAccess

**Control Flow (7):** Loop, Break, Continue, Return, Range, CollectionLoop, Raise

**Pattern Matching (4):** QuestionMatch, Conditional, PatternMatch, Block

**Functions (3):** Closure, MethodCall, Identifier

**Pointers (7):** AddressOf, Dereference, PointerOffset, PointerDereference, PointerAddress, CreateReference, CreateMutableReference

**References (3):** StdReference, BuiltinReference, ThisReference

**Constructors (6):** Some, None, StringInterpolation, Comptime, VecConstructor, DynVecConstructor, ArrayConstructor

**Other (3):** TypeCast, Defer, StringLength

---

## Implementation Plan

### Phase 1: Add TypedExpr Wrapper (Low Risk) ✅ COMPLETE

**Goal:** Add the wrapper struct without breaking existing code.

**Files:**
- `src/ast/expressions.rs` - Add TypedExpr struct
- `src/ast/mod.rs` - Export TypedExpr

**Changes:**
```rust
// Add to src/ast/expressions.rs

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: Expression,
    pub resolved_type: Option<AstType>,
}

impl TypedExpr {
    pub fn untyped(expr: Expression) -> Self {
        Self { expr, resolved_type: None }
    }

    pub fn typed(expr: Expression, ty: AstType) -> Self {
        Self { expr, resolved_type: Some(ty) }
    }

    /// Get type, inferring if not cached
    pub fn get_type(&self) -> Option<&AstType> {
        self.resolved_type.as_ref()
    }
}

// Convenience: allow using TypedExpr where Expression is expected
impl From<Expression> for TypedExpr {
    fn from(expr: Expression) -> Self {
        Self::untyped(expr)
    }
}

impl std::ops::Deref for TypedExpr {
    type Target = Expression;
    fn deref(&self) -> &Self::Target {
        &self.expr
    }
}
```

**Test:** Compile succeeds, all existing tests pass.

---

### Phase 2: Extend TypeContext ✅ COMPLETE (Alternative Approach)

**Goal:** Instead of updating parser, extend TypeContext to store more type info.

**Actual implementation:**
- Added `method_params: HashMap<String, Vec<(String, AstType)>>` to TypeContext
- Added `behavior_impls: HashMap<String, Vec<String>>` to TypeContext
- Added query methods: `get_method_params()`, `get_function_params()`, `get_enum_variant_type()`, etc.
- Typechecker now populates these during `build_type_context()`

---

### Phase 3: Update Codegen to Use TypeContext ✅ COMPLETE

**Goal:** Codegen checks TypeContext before falling back to local inference.

**Actual implementation:**
- `infer_common_method_type()` now checks `type_ctx.get_method_return_type()` first
- `infer_custom_enum_type()` now checks `type_ctx.get_enum_variants()` first
- Added `get_type_name()` helper to `AstType`

---

### Phase 2 (Original): Update Parser to Create TypedExpr (DEFERRED)

**Status:** Deferred - TypeContext approach gives 80% benefit with 20% effort

**Goal:** Parser outputs TypedExpr instead of Expression.

**Files to modify:**
```
src/parser/
├── mod.rs                 - Update return types
├── expressions/
│   ├── mod.rs             - parse_expression returns TypedExpr
│   ├── primary.rs         - 37 Expression constructions
│   ├── operators.rs       - 10 BinaryOp constructions
│   ├── calls.rs           - FunctionCall, MethodCall
│   ├── blocks.rs          - Block, StringInterpolation
│   ├── control_flow.rs    - Loop, Break, Continue, Return
│   ├── collections.rs     - Vec/DynVec/Array constructors
│   ├── literals.rs        - Literal expressions
│   ├── patterns.rs        - Pattern expressions
│   └── structs.rs         - Struct literals
├── statements.rs          - Statement expressions
├── patterns.rs            - Pattern arms with expressions
└── program.rs             - Top-level expressions
```

**Pattern for changes:**
```rust
// Before
Ok(Expression::Integer32(value))

// After
Ok(TypedExpr::untyped(Expression::Integer32(value)))
```

**Batch update strategy:**
```bash
# Find all Expression:: constructions in parser
rg "Expression::" src/parser/ -l

# Most can be mechanically updated with sed/replace
```

**Test:** Parser tests pass, can parse all examples.

---

### Phase 3: Update AST Containers (Medium Risk)

**Goal:** Update structs that contain Expression to use TypedExpr.

**Key structures:**
```rust
// src/ast/statements.rs
pub enum Statement {
    Let { value: TypedExpr, ... },     // was Expression
    Return(Option<TypedExpr>),          // was Option<Expression>
    Expression(TypedExpr),              // was Expression
    If { condition: TypedExpr, ... },   // was Expression
    While { condition: TypedExpr, ... },
    ...
}

// src/ast/expressions.rs
pub struct MatchArm {
    pub body: TypedExpr,  // was Expression
    ...
}

// src/ast/declarations.rs
pub struct FunctionDecl {
    pub body: TypedExpr,  // was Expression
    ...
}
```

**Files:**
- `src/ast/statements.rs`
- `src/ast/declarations.rs`
- `src/ast/patterns.rs`

**Test:** All AST tests pass.

---

### Phase 4: Typechecker Populates Types (High Risk)

**Goal:** Typechecker fills `resolved_type` during inference.

**Primary file:** `src/typechecker/mod.rs`

**Key function:** `infer_expression_type()`

**Current signature:**
```rust
fn infer_expression_type(&mut self, expr: &Expression) -> Result<AstType>
```

**New signature:**
```rust
fn infer_expression_type(&mut self, expr: &mut TypedExpr) -> Result<AstType> {
    let ty = match &expr.expr {
        Expression::Integer32(_) => AstType::I32,
        Expression::BinaryOp { left, op, right } => {
            // Recursively type children
            let left_ty = self.infer_expression_type(left)?;
            let right_ty = self.infer_expression_type(right)?;
            self.infer_binary_op_type(&left_ty, op, &right_ty)?
        }
        // ... 50+ more arms
    };

    // Store the resolved type
    expr.resolved_type = Some(ty.clone());
    Ok(ty)
}
```

**Challenge:** Expression is inside TypedExpr, and sub-expressions are inside Expression variants. Need to thread TypedExpr through or use a different approach.

**Alternative:** Keep inference signature, store types in TypeContext side table:
```rust
fn infer_expression_type(&mut self, expr: &TypedExpr) -> Result<AstType> {
    let ty = match &expr.expr { ... };
    self.type_ctx.set_expr_type(expr.id, ty.clone());
    Ok(ty)
}
```

**Files:**
- `src/typechecker/mod.rs` - Main inference
- `src/typechecker/inference.rs` - Helper functions
- `src/typechecker/validation.rs` - Type validation
- `src/typechecker/type_resolution.rs` - Type resolution

**Test:** Type checking tests pass, types are populated.

---

### Phase 5: Codegen Reads Cached Types (Medium Risk)

**Goal:** Codegen uses `resolved_type` instead of re-inferring.

**Primary file:** `src/codegen/llvm/expressions/inference.rs`

**Change pattern:**
```rust
// Before
pub fn infer_expression_type(
    compiler: &LLVMCompiler,
    expr: &Expression,
) -> Result<AstType, CompileError> {
    match expr {
        Expression::Integer32(_) => Ok(AstType::I32),
        // ... 40+ arms
    }
}

// After
pub fn get_expression_type(
    compiler: &LLVMCompiler,
    expr: &TypedExpr,
) -> Result<AstType, CompileError> {
    // Fast path: use cached type
    if let Some(ty) = &expr.resolved_type {
        return Ok(ty.clone());
    }

    // Slow path: infer (should rarely happen)
    infer_expression_type_fallback(compiler, &expr.expr)
}
```

**Files:**
- `src/codegen/llvm/expressions/inference.rs` - Type lookup
- `src/codegen/llvm/expressions/mod.rs` - compile_expression
- `src/codegen/llvm/expressions/utils.rs` - Helper functions
- `src/codegen/llvm/functions/calls.rs` - Call compilation

**Test:** Codegen tests pass, generated code is correct.

---

### Phase 6: Remove Duplicate Inference (Low Risk)

**Goal:** Delete redundant inference code.

**After Phase 5 is stable:**
1. Remove fallback inference paths
2. Delete `infer_expression_type_fallback`
3. Simplify `get_expression_type` to just read cache
4. Remove unused helper functions

**Expected reduction:** ~800-1000 LOC

**Test:** Full test suite passes.

---

### Phase 7: LSP Enhancement (Low Risk)

**Goal:** LSP uses cached types for better performance.

**Files:**
- `src/lsp/hover/expressions.rs` - Read types for hover
- `src/lsp/inlay_hints.rs` - Use cached types
- `src/lsp/type_inference.rs` - Simplify or remove

**Test:** LSP features work correctly.

---

## Risk Mitigation

### Testing Strategy

1. **Unit tests** for each phase before proceeding
2. **Integration tests** with example programs
3. **Regression tests** for edge cases:
   - Generic type instantiation
   - Pattern matching exhaustiveness
   - Closure type inference
   - Method call resolution

### Rollback Plan

Each phase is independent:
- Phase 1-2: Revert TypedExpr, restore Expression
- Phase 3: Revert container changes
- Phase 4: Keep old inference, ignore resolved_type
- Phase 5-7: Keep new inference, just slower

### Performance Monitoring

Track compile times before/after:
```bash
# Benchmark
time cargo run --release -- examples/showcase.zen
```

Expected: 10-20% faster after Phase 5.

---

## Dependencies

### Before starting:
- [ ] TypeContext is integrated (DONE)
- [ ] WellKnownTypes registry is used (DONE)
- [ ] All tests pass (DONE)

### Phase gates:
- [ ] Phase 1 complete → Phase 2 can start
- [ ] Phase 2 complete → Phase 3 can start
- [ ] Phase 3 complete → Phase 4 can start
- [ ] Phase 4 complete → Phase 5 can start
- [ ] Phase 5 stable → Phase 6 can start
- [ ] Phase 6 complete → Phase 7 can start

---

## Timeline Estimate

| Phase | Effort | Parallel? |
|-------|--------|-----------|
| Phase 1: Add TypedExpr | 1 session | No |
| Phase 2: Update Parser | 2 sessions | No |
| Phase 3: Update Containers | 1 session | No |
| Phase 4: Typechecker | 3 sessions | No |
| Phase 5: Codegen | 2 sessions | No |
| Phase 6: Cleanup | 1 session | No |
| Phase 7: LSP | 1 session | Yes |
| **Total** | **~11 sessions** | |

---

## Success Criteria

1. **Functional:** All existing tests pass
2. **Performance:** Compile time ≤ current (ideally faster)
3. **Code reduction:** ~1000 LOC removed from codegen
4. **Maintainability:** Single source of type inference
5. **Extensibility:** Easy to add more metadata to expressions

---

## Related Documentation

- `docs/REFACTOR_SEMA.md` - Sema integration status
- `docs/ARCHITECTURE.md` - Compiler architecture
- `docs/design/SEPARATION_OF_CONCERNS.md` - Three-layer design
