# Quick Start: Task #15 Phase 3 - Codegen Migration

**For**: Anyone starting Phase 3 of Task #15  
**Status**: Ready to execute  
**Duration**: 2-3 days  
**Complexity**: HIGH  

## TL;DR

1. **Problem**: Compiler has 80+ hardcoded references to Option/Result
2. **Goal**: Replace with generic enum handling
3. **Impact**: Enables fully self-hosted enums
4. **Risk**: Medium (but comprehensive tests catch regressions)

## Current State (After Phases 1-2)

✅ Parser loads Option/Result from stdlib files  
✅ Module system resolves paths correctly  
✅ Enum definitions parse without errors  
✅ All 116 tests passing  
⚠️ Codegen still uses hardcoded types (Phase 3 job)  

## Starting Phase 3

### 1. Get Oriented (30 minutes)

Read these in order:
1. **TASK_15_PHASE3_ROADMAP.md** - Detailed plan
2. **SESSION_STATUS_TASK15.md** - Current architecture
3. **TASK_15_ANALYSIS.md** - Why this task exists

### 2. Understand the Code (1 hour)

Key files to understand:
```
src/codegen/llvm/
├── types.rs                    # Type layout generation
├── expressions/
│   ├── utils.rs               # Expression utilities (10+ hardcoded)
│   ├── enums.rs               # Enum codegen
│   └── enums_variant.rs       # Variant creation
├── patterns/
│   ├── compile.rs             # Pattern matching (8+ hardcoded)
│   └── enum_pattern.rs        # Enum-specific patterns
├── functions/
│   └── calls.rs               # Function call routing (4 hardcoded)
└── ...

src/typechecker/
├── mod.rs                      # Type checking (4+ hardcoded)
└── validation.rs              # Type validation
```

### 3. Verify Setup (10 minutes)

```bash
# Run baseline tests
cargo test

# Should see: 116/116 PASSED ✅
```

### 4. Create Generic Helpers (1-2 hours)

Create `src/codegen/llvm/generic_enum_support.rs`:

```rust
/// Check if a type is a generic enum with expected argument count
pub fn is_generic_enum(name: &str, expected_args: usize, type_args: &[AstType]) -> bool {
    type_args.len() == expected_args
}

/// Get the Ok/Some type argument from a generic enum
pub fn get_generic_enum_some_type(type_args: &[AstType]) -> Option<&AstType> {
    type_args.first()
}

/// Get the Err/None type argument from a generic enum (for Result)
pub fn get_generic_enum_err_type(type_args: &[AstType]) -> Option<&AstType> {
    type_args.get(1)
}
```

### 5. Start Migration (Section by Section)

**Section 1: Type System** (src/codegen/llvm/types.rs)
```bash
# Edit src/codegen/llvm/types.rs line 368
# Replace: if name == "Result" || name == "Option"
# With: if is_generic_enum(name, expected_args, type_args)

# Test after changes
cargo test
```

**Section 2: Enum Variants** (src/codegen/llvm/expressions/)
```bash
# Edit src/codegen/llvm/expressions/enums.rs
# Remove hardcoded Option/Some/None handling
# Use generic enum path

# Test after changes
cargo test
```

**Section 3: Pattern Matching** (src/codegen/llvm/patterns/)
```bash
# Edit src/codegen/llvm/patterns/compile.rs
# Replace Option/Result-specific code with generic enum handling

# Test after changes
cargo test
```

Continue for sections 4-6 as outlined in TASK_15_PHASE3_ROADMAP.md

## Test Checklist

After each major change:

```bash
# Quick test
cargo test --lib

# Full test
cargo test

# Specific tests
cargo test --test enum_intrinsics
cargo test --test codegen_integration
```

**Must See**: 116/116 PASSED ✅

## Quick Reference: Hardcoded Patterns

### Pattern 1: Option/Result Type Check
```rust
// OLD
if name == "Option" || name == "Result" { ... }

// NEW
if compiler.is_generic_enum(name, 1, &type_args) || 
   compiler.is_generic_enum(name, 2, &type_args) { ... }
```

### Pattern 2: Generic Type Arguments
```rust
// OLD
if name == "Result" && type_args.len() == 2 {
    let ok_type = &type_args[0];
    let err_type = &type_args[1];
}

// NEW
if is_generic_enum("Result", 2, type_args) {
    let ok_type = get_generic_enum_some_type(type_args);
    let err_type = get_generic_enum_err_type(type_args);
}
```

### Pattern 3: Module Dispatch
```rust
// OLD
else if module == "Result" {
    compiler.compile_enum_variant("Result", func, &payload)
} else if module == "Option" {
    compiler.compile_enum_variant("Option", func, &payload)
}

// NEW
else {
    // Let generic enum handler route it
    compiler.compile_generic_enum_variant(module, func, &payload)
}
```

## Key Files Summary

| File | Hardcoded Count | Priority | Effort |
|------|-----------------|----------|--------|
| expressions/utils.rs | 10+ | HIGH | 8+ hrs |
| patterns/compile.rs | 8 | HIGH | 8 hrs |
| functions/calls.rs | 4 | MEDIUM | 4 hrs |
| typechecker/mod.rs | 4+ | MEDIUM | 4 hrs |
| generics.rs | 2 | MEDIUM | 2 hrs |
| expressions/enums*.rs | 5 | MEDIUM | 4 hrs |
| Other files | 40+ | LOW | 6 hrs |

## Common Pitfalls

### 1. Breaking Pattern Matching
**Issue**: Pattern matching for nested Option/Result breaks  
**Solution**: Test extensively, use generic pattern code

### 2. Type Inference Issues
**Issue**: Type parameters lost during monomorphization  
**Solution**: Track types generically, don't special-case Option/Result

### 3. Generic Type Arguments
**Issue**: Can't find type arguments for Option<T>  
**Solution**: Use generic enum type tracking

## Rollback Instructions

If things break badly:

```bash
# See what changed
git diff src/codegen/llvm/

# Revert the current file
git checkout -- src/codegen/llvm/expressions/utils.rs

# Or revert entire phase 3
git reset --hard HEAD~N
```

## Communication/Documentation

While working on Phase 3:

1. **Commit after each section** with message:
   ```
   Task #15 Phase 3: Remove hardcoded Option/Result from [filename]
   
   - Replaced 5 hardcoded checks with generic enum handling
   - Tests: 116/116 PASSED ✅
   ```

2. **Create daily progress file**:
   ```
   TASK_15_PHASE3_PROGRESS.md (update daily)
   - [ ] Type System refactoring
   - [x] Enum Variant Codegen (completed)
   - [ ] Pattern Matching
   - etc.
   ```

3. **Update this file** when Phase 3 is complete:
   - Change "Ready to execute" to "COMPLETED ✅"

## Success Criteria

- [x] All 116 tests passing
- [x] Zero new compiler warnings
- [x] No hardcoded Option/Result references remain
- [x] Generic enum handling works for all cases
- [x] Pattern matching works
- [x] Type inference works

## Estimated Timeline

| Section | Time | Status |
|---------|------|--------|
| Generic helpers | 1-2 hrs | ⏳ |
| Type System | 4-8 hrs | ⏳ |
| Variants | 4-8 hrs | ⏳ |
| Patterns | 4-8 hrs | ⏳ |
| Function routing | 2-4 hrs | ⏳ |
| Type checking | 2-4 hrs | ⏳ |
| LSP/Cleanup | 2-4 hrs | ⏳ |
| Verification | 2-4 hrs | ⏳ |
| **TOTAL** | **24-48 hrs** | **2-3 days** |

## Need Help?

1. **Check TASK_15_PHASE3_ROADMAP.md** - Detailed roadmap
2. **Search for patterns** - Use finder tool to locate similar code
3. **Run tests** - Tests will tell you what's broken
4. **Look at generic enums** - See how Vec/HashMap handle generics
5. **Check git history** - See how Tasks #16-18 were structured

## Next Phase

After Phase 3 completes:

- **Phase 4**: Typechecker cleanup (1 day)
- **Phase 5**: LSP updates (1 day)  
- **Phase 6**: Testing & verification (1 day)

Then Task #15 is complete!

---

**Document**: QUICK_START_TASK15_PHASE3.md  
**Status**: Ready for execution  
**Updated**: 2025-01-27
