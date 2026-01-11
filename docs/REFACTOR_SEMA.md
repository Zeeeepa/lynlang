# Sema Integration Refactor

**Status:** MOSTLY COMPLETE
**Last Updated:** January 2026

---

## Summary

The typechecker is now integrated into the main compilation pipeline. Type checking happens before monomorphization, as it should.

---

## Completed Work

### Phase 1: Typechecker Integration

- [x] Typechecker now called in `compiler.rs` pipeline
- [x] Called after `resolve_self_types()`, before `monomorphize()`
- [x] All tests pass with typechecker in pipeline

### Phase 2: Dead Code Removal

- [x] Deleted `vec_support.rs` (326 LOC) - fixed array codegen moved to stdlib
- [x] Deleted `stdlib_codegen/collections.rs` (670 LOC) - collections now in Zen stdlib
- [x] Removed hardcoded HashMap/Set/Vec constructors from codegen
- [x] Total removed: ~1,000 LOC

### Phase 3: Module Cleanup

- [x] Deleted `src/ffi/` (1,455 LOC) - dead FFI module
- [x] Deleted `src/behaviors/` (~400 LOC) - orphaned behavior system
- [x] Fixed duplicate module declarations in main.rs

---

## Current Pipeline

```
Source
  │
  ▼
Lexer (lexer.rs)
  │
  ▼
Parser (parser/)
  │
  ▼
process_imports()      ← module_system/
  │
  ▼
execute_comptime()     ← comptime/
  │
  ▼
resolve_self_types()   ← typechecker/self_resolution.rs
  │
  ▼
typecheck()            ← typechecker/ ✅ INTEGRATED
  │
  ▼
monomorphize()         ← type_system/
  │
  ▼
compile_program()      ← codegen/llvm/
  │
  ▼
LLVM IR
```

---

## Remaining Work

### High Priority

1. **Remove codegen type inference (~400 LOC)**
   - `codegen/llvm/expressions/inference.rs` duplicates typechecker logic
   - Requires: AST type annotations so codegen can read types instead of re-inferring
   - Blocked by: AST changes

2. **Fix hardcoded generics in GenericTypeTracker**
   - `codegen/llvm/generics.rs` uses well_known.rs for Option, Result
   - ✅ DONE - Uses `wk.is_result()` and `wk.is_option()` checks

### Medium Priority

3. **Add type annotations to AST**
   - Add `resolved_type: Option<AstType>` to Expression nodes
   - Typechecker fills this in during type checking
   - Codegen reads annotations instead of re-inferring

4. **Clean up dead_code markers** ✅ PARTIAL
   - Audited `typechecker/behaviors.rs` - removed 3 unused methods
   - Removed all unnecessary `#[allow(dead_code)]` annotations from behaviors.rs

### Low Priority

5. **Split large modules**
   - `codegen/` at 11,776 LOC
   - `lsp/` at 12,338 LOC
   - Both could be more modular

---

## Key Files

| File | Purpose | LOC |
|------|---------|-----|
| `src/compiler.rs` | Pipeline orchestrator | 422 |
| `src/typechecker/mod.rs` | Main typechecker | 991 |
| `src/typechecker/inference.rs` | Type inference (keep) | 1,008 |
| `src/codegen/llvm/expressions/inference.rs` | Type inference (remove) | ~400 |
| `src/codegen/llvm/generics.rs` | Generic tracking | ~300 |

---

## Notes for Future Work

The duplicate type inference exists because:
1. AST doesn't carry type annotations after typechecking
2. Codegen needs type info to generate correct LLVM IR
3. Codegen re-infers types since it can't read from AST

Proper fix:
1. Add `resolved_type` field to AST Expression nodes
2. Typechecker fills in resolved types
3. Codegen reads types, no inference needed
4. Delete codegen inference code

This is a larger refactor tracked separately.
