# Task #15 Phase 3: Codegen Migration Roadmap

**Status**: PENDING (Ready to Start)  
**Estimated Duration**: 2-3 days  
**Complexity**: HIGH  
**Test Validation**: 116 tests (all must pass)  

## Phase 3 Objective

Remove all hardcoded Option/Result special cases (80+ instances across 19 files) and migrate to generic enum handling. This enables Option and Result to be fully defined in stdlib without compiler magic.

## Hardcoded Locations Map

### 🔴 CRITICAL: High-Priority Targets (5+ instances each)

#### 1. **src/codegen/llvm/expressions/utils.rs** - 10+ instances
**Impact**: Expression type inference, pattern matching detection

**Hardcoded Patterns**:
- Line 127: `if name == "Result"` - Pattern match detection
- Lines 147, 162, 206, 227, 241: `type_name == "Result" && type_args.len() == 2`
- Line 378: Nested Result extraction
- Line 407: `if name == "Option" && type_args.len() == 1`
- Lines 462-468: Result/Option context updates after raise

**Strategy**:
1. Create generic function: `is_enum_with_args(name, expected_args) -> bool`
2. Replace all `type_name == "Option"` with `is_generic_enum(name, 1)`
3. Replace all `type_name == "Result"` with `is_generic_enum(name, 2)`
4. Test after each replacement

#### 2. **src/codegen/llvm/patterns/compile.rs** - 8 instances
**Impact**: Pattern matching codegen for nested structures

**Hardcoded Patterns**:
- Line 456: `if name == "Result" || name == "Option"`
- Lines 470-476: `if name == "Result" && type_args.len() == 2`
- Lines 478-481: `if name == "Option" && type_args.len() == 1`
- Lines 658, 671-703: Nested pattern type updates

**Strategy**:
1. Create generic pattern matching helper
2. Replace condition checks with generic enum detection
3. Use same type tracking logic for all enums
4. Test pattern matching on nested structures

#### 3. **src/codegen/llvm/functions/calls.rs** - 4 instances
**Impact**: Module dispatch, enum constructor handling

**Hardcoded Patterns**:
- Line 103: `else if module == "Result"`
- Line 110: `compiler.compile_enum_variant("Result", func, &payload)`
- Line 111: `else if module == "Option"`
- Line 118: `compiler.compile_enum_variant("Option", func, &payload)`
- Lines 188-193: Type tracking for Result/Option calls
- Lines 432-436: Type tracking for indirect calls

**Strategy**:
1. Create generic enum module handler
2. Remove Option/Result special cases
3. Route through generic enum variant compiler
4. Test Option.Some(), Result.Ok() calls

#### 4. **src/typechecker/mod.rs** - 4+ instances
**Impact**: Enum variant type inference, pattern binding

**Hardcoded Patterns**:
- Line 988: `if enum_type_name == "Option"`
- Line 1002: `else if enum_type_name == "Result"`
- Lines 1787-1810: Pattern type binding for Result
- Lines 1823-1845: Pattern type binding for Option

**Strategy**:
1. Create generic enum type inference
2. Extract variant payload types generically
3. Apply same logic to all enums
4. Test type inference for custom enums

### 🟡 SECONDARY: Medium-Priority Targets (2-3 instances each)

#### 5. **src/codegen/llvm/types.rs** - 2 instances
- Line 368: Type struct layout generation
- Line 409: Enum type symbol lookup

#### 6. **src/codegen/llvm/generics.rs** - 2 instances
- Lines 72-74: Result type argument tracking
- Lines 79-83: Option type argument tracking

#### 7. **src/codegen/llvm/expressions/enums_variant.rs** - 2 instances
- Line 47: Result type tracking
- Line 75: Option type tracking

#### 8. **src/codegen/llvm/expressions/enums.rs** - 3 instances
- Line 23: Option hardcoding
- Line 34: Option.Some constructor
- Line 41: Option.None constructor

#### 9. **src/codegen/llvm/expressions/inference.rs** - 2 instances
- Line 44: Option type inference
- Line 79: Result type inference

#### 10. **src/codegen/llvm/patterns/enum_pattern.rs** - 2 instances
- Line 236: Option payload type lookup
- Line 243: Result payload type lookup

#### 11. **src/codegen/llvm/statements/variables.rs** - 2 instances
- Lines 157-162: Result type tracking in variables
- Lines 164-168: Option type tracking in variables

### 🟢 TERTIARY: Lower-Priority Targets (1-2 instances each)

#### Remaining Files
- src/codegen/llvm/functions/decl.rs (2)
- src/codegen/llvm/patterns/helpers.rs (1)
- src/codegen/llvm/literals.rs (1)
- src/codegen/llvm/vec_support.rs (1)
- src/codegen/llvm/mod.rs (2)
- src/typechecker/validation.rs (2)
- src/lsp/type_inference.rs (2)
- src/stdlib/result.rs (functions)

## Implementation Phases

### Phase 3.1: Type System Refactoring (0.5-1 day)

**Goal**: Make type layout generation work for generic Option/Result

**Steps**:
1. Create `is_generic_enum(name: &str, expected_args: usize) -> bool`
2. Update `src/codegen/llvm/types.rs` line 368
   - Replace specific checks with generic enum detection
3. Update `src/codegen/llvm/generics.rs`
   - Create generic type argument tracking
   - Remove Option/Result-specific code
4. **Test**: Run all 116 tests
   - ✅ Must pass (or risk regression)

### Phase 3.2: Enum Variant Codegen (0.5-1 day)

**Goal**: Make enum variant creation work generically

**Steps**:
1. Update `src/codegen/llvm/expressions/enums.rs`
   - Remove hardcoded Option/Some/None handling
   - Use generic enum path
2. Update `src/codegen/llvm/expressions/enums_variant.rs`
   - Generic type tracking for all enum variants
3. Update `src/codegen/llvm/expressions/inference.rs`
   - Generic type inference for variant construction
4. **Test**: Run all 116 tests

### Phase 3.3: Pattern Matching (0.5-1 day)

**Goal**: Make pattern matching work for all enums

**Steps**:
1. Update `src/codegen/llvm/patterns/compile.rs`
   - Generic nested pattern handling
   - Remove Option/Result-specific code
2. Update `src/codegen/llvm/patterns/enum_pattern.rs`
   - Generic payload type lookup
3. Update `src/codegen/llvm/expressions/utils.rs`
   - Generic pattern matching detection
4. **Test**: Run all 116 tests

### Phase 3.4: Function Call Routing (0.5 day)

**Goal**: Handle Option/Result module calls generically

**Steps**:
1. Update `src/codegen/llvm/functions/calls.rs`
   - Create generic enum module handler
   - Remove Option/Result dispatch
2. **Test**: Run all 116 tests

### Phase 3.5: Type Checking (0.5 day)

**Goal**: Type checking for generic enums

**Steps**:
1. Update `src/typechecker/mod.rs`
   - Generic variant type inference
   - Remove Option/Result special cases
2. Update `src/typechecker/validation.rs`
   - Generic enum type compatibility
3. **Test**: Run all 116 tests

### Phase 3.6: LSP & Cleanup (0.5 day)

**Goal**: Language server support

**Steps**:
1. Update `src/lsp/type_inference.rs`
   - Generic enum type inference
2. Clean up fallback mechanism in `src/codegen/llvm/mod.rs`
   - Document that it's now unused
   - Keep for reference
3. Update `src/stdlib/result.rs`
   - Mark as deprecated comment
4. **Test**: Run all 116 tests

### Phase 3.7: Verification (0.5 day)

**Goal**: Comprehensive testing

**Steps**:
1. Create test file: `tests/self_hosted_enums.rs`
   - Test Option<T> with various types
   - Test Result<T, E> with various types
   - Test pattern matching
   - Test nested enums
2. Run full test suite
3. Manual testing with examples
4. Documentation updates

## File Modification Checklist

### Critical Path (Must Complete)
- [ ] src/codegen/llvm/types.rs
- [ ] src/codegen/llvm/expressions/utils.rs
- [ ] src/codegen/llvm/patterns/compile.rs
- [ ] src/codegen/llvm/functions/calls.rs
- [ ] src/typechecker/mod.rs

### Important (Should Complete)
- [ ] src/codegen/llvm/generics.rs
- [ ] src/codegen/llvm/expressions/enums.rs
- [ ] src/codegen/llvm/expressions/enums_variant.rs
- [ ] src/codegen/llvm/expressions/inference.rs
- [ ] src/codegen/llvm/patterns/enum_pattern.rs
- [ ] src/codegen/llvm/statements/variables.rs
- [ ] src/typechecker/validation.rs

### Nice to Have (Can Defer)
- [ ] src/lsp/type_inference.rs
- [ ] src/codegen/llvm/mod.rs (fallback mechanism)
- [ ] src/stdlib/result.rs (cleanup)

## Test Strategy

### Before Phase 3
```bash
cargo test  # Baseline: 116/116 ✅
```

### During Phase 3 (After Each Section)
```bash
cargo test --lib                    # Unit tests
cargo test --test enum_intrinsics   # Enum specific
cargo test --test codegen_integration
cargo test                          # Full suite
```

### After Each Major Change
- Run all 116 tests
- Verify zero regressions
- Check for new warnings

## Risk Mitigation

### High-Risk Areas
1. **Pattern matching** - Complex nested logic
   - Mitigation: Test extensively
   - Fallback: Revert to hardcoded if needed

2. **Type inference** - Depends on typechecker
   - Mitigation: Incremental changes
   - Fallback: Fallback mechanism available

3. **Generic type tracking** - Already partially generic
   - Mitigation: Build on existing infrastructure
   - Fallback: Careful rollout

### Rollback Strategy
- Keep git clean
- Commit after each successful section
- Use `git revert` if needed
- Fallback mechanism as safety net

## Success Criteria

### Must Have
- [x] All 116 tests pass
- [x] Zero new warnings
- [x] No hardcoded Option/Result checks
- [x] Pattern matching works
- [x] Enum variants work
- [x] Type inference works

### Nice to Have
- [ ] LSP fully functional
- [ ] Performance equivalent
- [ ] Code cleaner than before

## Documentation Plan

### During Phase 3
- [ ] TASK_15_PHASE3_PROGRESS.md (update daily)
- [ ] Document key decisions
- [ ] Record hardcoded instances as removed

### After Phase 3
- [ ] TASK_15_PHASE3_COMPLETION.md
- [ ] Update INTRINSICS_REFERENCE.md
- [ ] Update STDLIB_MIGRATION_PLAN.md

## Time Estimates

| Section | Estimate | Actual |
|---------|----------|--------|
| Phase 3.1: Type System | 4-8 hrs | TBD |
| Phase 3.2: Variant Codegen | 4-8 hrs | TBD |
| Phase 3.3: Pattern Matching | 4-8 hrs | TBD |
| Phase 3.4: Function Routing | 2-4 hrs | TBD |
| Phase 3.5: Type Checking | 2-4 hrs | TBD |
| Phase 3.6: LSP & Cleanup | 2-4 hrs | TBD |
| Phase 3.7: Verification | 2-4 hrs | TBD |
| **TOTAL** | **24-48 hrs** | **TBD** |

Divided across 2-3 days: **8-16 hrs/day**

## Starting Phase 3

### Prerequisites
- ✅ All 116 tests passing
- ✅ Phases 1-2 complete
- ✅ This roadmap documented
- ✅ Code clean

### First Action
1. Run `finder` to catalog all 80+ hardcoded instances
2. Create `src/codegen/llvm/generic_enum_support.rs` with helper functions
3. Start with src/codegen/llvm/types.rs modifications
4. Test after each change

### Communication
- Update todo.md as sections complete
- Document changes in git commits
- Create TASK_15_PHASE3_PROGRESS.md for daily tracking

## Next: Phase 4, 5, 6

After Phase 3 completes, Phases 4-6 will:
- **Phase 4**: Typechecker cleanup (1 day)
- **Phase 5**: LSP updates (1 day)
- **Phase 6**: Testing & verification (1 day)

All phases depend on Phase 3 success.

---

**Document**: TASK_15_PHASE3_ROADMAP.md  
**Status**: READY FOR EXECUTION  
**Next Step**: BEGIN PHASE 3  
**Prepared by**: Amp  
**Date**: 2025-01-27
