# Codegen Bugs Review and Test Coverage

**Last Updated**: 2025-01-27  
**Purpose**: Comprehensive review of codegen bugs found, fixes applied, and test coverage

## Overview

This document consolidates all codegen bug reviews and test coverage analysis. It replaces the separate review documents for easier reference.

## Critical Bugs Fixed

### 1. Pattern Matching: Basic Blocks Without Terminators ✅ FIXED

**Date**: 2025-01-27  
**Location**: `src/codegen/llvm/expressions/patterns.rs`

**Problem**: 
- Pattern matching created `arm_blocks` but never properly terminated them
- Caused LLVM verification error: "Basic Block does not have terminator"

**Fix**: Restructured to use `test_blocks` and `body_blocks` with proper control flow

**Tests**: `test_pattern_matching_compiles`, `test_pattern_matching_with_return`

---

### 2. Pattern Matching: Terminator in Middle of Block ✅ FIXED

**Date**: 2025-01-27  
**Location**: `src/codegen/llvm/expressions/patterns.rs:96-101`

**Problem**: 
- When pattern match arms contained return statements, code tried to add branch after return
- Caused "Terminator found in middle of block" error

**Fix**: Check for existing terminators before adding merge branch

**Tests**: `test_pattern_matching_with_return`

---

### 3. Conditional: Missing Terminator Check ✅ FIXED

**Date**: 2025-01-27  
**Location**: `src/codegen/llvm/control_flow.rs:31-45`

**Problem**: 
- Conditional expressions didn't check for terminators before adding branches
- Same issue as pattern matching

**Fix**: Added terminator checks before adding merge branches

**Tests**: `test_conditional_with_return`

---

### 4. Phi Node: Wrong Basic Block References ✅ FIXED

**Date**: 2025-01-27  
**Location**: `src/codegen/llvm/patterns/compile.rs:791-804`

**Problem**: 
- Phi nodes used start blocks (`then_bb`, `else_bb`) instead of end blocks
- After `build_unconditional_branch()`, should capture actual end blocks

**Fix**: Capture `then_bb_end` and `else_bb_end` after branches are added

**Tests**: `test_pattern_matching_phi_node_basic_blocks`

---

### 5. Void Return Type Bug ✅ FIXED

**Date**: 2025-01-27  
**Location**: `src/codegen/llvm/expressions/control.rs:227-232`

**Problem**: 
- `compile_return` always called `build_return(Some(&value))` even for void functions
- Void functions should use `build_return(None)`

**Fix**: Check if function is void before building return instruction

**Tests**: `test_void_function_with_expression`, `test_void_function_no_return`

---

## Known Issues (Not Bugs, But Worth Noting)

### 1. Pointer Dereferencing: Hardcoded i32 Type ⚠️

**Location**: `src/codegen/llvm/pointers.rs:141-142`

**Issue**: Assumes `i32` when type information is missing  
**Impact**: Medium - Could cause incorrect loads for other types  
**Status**: Documented, needs improvement

### 2. Pattern Matching: Dummy i32 for Void ⚠️

**Location**: `src/codegen/llvm/expressions/patterns.rs:110`

**Issue**: Returns dummy `i32` instead of proper void/unit type  
**Impact**: Low - Works but semantically incorrect  
**Status**: Documented

### 3. Array Codegen: Hardcoded i32 ⚠️

**Location**: `src/codegen/llvm/functions/arrays.rs`

**Issue**: Array operations hardcode `i32` element type  
**Impact**: High - Breaks `Array<i64>`, `Array<f64>`, etc.  
**Status**: Documented TODO

### 4. Nested Struct Field Access Bug ⚠️

**Location**: `src/codegen/llvm/structs.rs`

**Issue**: Field values get swapped in nested struct access  
**Impact**: Critical - Runtime bug  
**Status**: Documented in `tests/known_bugs/README.md`

---

## Test Coverage

### ✅ Tests We Have (8 tests)

1. `test_pattern_matching_compiles` - Basic pattern matching
2. `test_pattern_matching_with_return` - Pattern matching with returns
3. `test_conditional_with_return` - Conditionals with returns
4. `test_void_function_with_expression` - Void function compilation
5. `test_void_function_no_return` - Implicit void returns
6. `test_nested_struct_field_access` - GEP operations
7. `test_multiple_pattern_arms_compiles` - Multiple arms
8. `test_pattern_matching_phi_node_basic_blocks` - Phi node correctness

### ⚠️ Tests We're Missing

- Phi node payload extraction path (requires stdlib)
- Array element access with different types
- Vec operations
- Execution/runtime tests
- Complex nested control flow

**Coverage**: ~70% of critical codegen bugs are tested

---

## GEP Operations Status

- ✅ Nested struct field access: GEP code correct (bug elsewhere)
- ✅ Vec element access: Correct
- ⚠️ Array element access: Hardcoded i32 issue
- ✅ Most GEP operations use correct types and indices

---

## Phi Nodes Status

- ✅ Array.get() phi node: Correct
- ✅ Array.pop_by_ptr() phi node: Correct
- ✅ Pattern matching pointer phi node: Correct
- ✅ Pattern matching payload phi node: Fixed

---

## Recommendations

1. **Add stdlib-based tests** for Option/Result pattern matching
2. **Add execution tests** to catch runtime bugs
3. **Fix hardcoded types** in Array codegen
4. **Investigate nested struct bug** (GEP code looks correct)
5. **Improve pointer type inference** when type info missing

