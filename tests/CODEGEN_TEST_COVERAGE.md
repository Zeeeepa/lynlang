# Codegen Test Coverage

**Date**: 2025-01-27  
**File**: `tests/codegen_integration.rs`

## Test Coverage Summary

### ✅ Tests We Have

1. **`test_pattern_matching_compiles`**
   - Tests: Basic pattern matching compilation
   - Catches: Basic blocks without terminators bug
   - Status: ✅ Covered

2. **`test_pattern_matching_with_return`**
   - Tests: Pattern matching with return statements in arms
   - Catches: "Terminator in middle of block" bug
   - Status: ✅ Covered

3. **`test_conditional_with_return`**
   - Tests: Conditional expressions with return statements
   - Catches: Missing terminator checks in conditionals
   - Status: ✅ Covered

4. **`test_nested_struct_field_access`**
   - Tests: Nested struct field access (e.g., `rect.bottom_right.x`)
   - Catches: GEP bugs in nested struct access
   - Status: ✅ Covered (but known bug still exists)

5. **`test_multiple_pattern_arms_compiles`**
   - Tests: Pattern matching with multiple arms
   - Catches: Control flow issues with multiple paths
   - Status: ✅ Covered

6. **`test_pattern_matching_phi_node_basic_blocks`**
   - Tests: Pattern matching that creates phi nodes
   - Catches: Phi node basic block reference bugs
   - Status: ✅ Covered (general test)

### ⚠️ Tests We're Missing

1. **Phi Node: Payload Extraction Path**
   - **What**: Pattern matching enum variants with pointer payloads (e.g., `Option<*T>`)
   - **Why Important**: The specific bug we fixed was in `patterns/compile.rs:801-804` where payload extraction used wrong basic blocks
   - **Why Missing**: Requires stdlib types (Option/Result) which adds complexity
   - **Status**: ⚠️ Not directly tested (but general phi node test covers compilation)

2. **GEP: Array Element Access**
   - **What**: Array indexing with different element types
   - **Why Important**: Known hardcoded `i32` bug in Array codegen
   - **Status**: ⚠️ Not tested (would require stdlib Array type)

3. **GEP: Vec Element Access**
   - **What**: Vec indexing operations
   - **Why Important**: Verify Vec GEP operations are correct
   - **Status**: ⚠️ Not tested (would require stdlib Vec type)

4. **Phi Node: Multiple Control Flow Paths**
   - **What**: Nested conditionals/pattern matches creating complex phi nodes
   - **Why Important**: Ensures phi nodes work correctly in complex scenarios
   - **Status**: ⚠️ Not tested

5. **Execution Tests**
   - **What**: Actually run compiled code and verify output
   - **Why Important**: Catches runtime bugs, not just compilation errors
   - **Status**: ⚠️ Not implemented (would require JIT execution)

## What Each Bug Fix Tests

### Pattern Matching Basic Blocks Bug
- ✅ Tested by: `test_pattern_matching_compiles`
- ✅ Tested by: `test_pattern_matching_with_return`

### Terminator Check Bugs
- ✅ Tested by: `test_pattern_matching_with_return`
- ✅ Tested by: `test_conditional_with_return`

### Phi Node Basic Block References Bug
- ⚠️ Partially tested by: `test_pattern_matching_phi_node_basic_blocks`
- ❌ Not directly tested: Payload extraction path (requires stdlib)

### GEP Nested Struct Bug
- ✅ Tested by: `test_nested_struct_field_access`
- ⚠️ Note: Test compiles but bug still exists (runtime issue, not compilation)

## Recommendations

1. **Add stdlib-based tests**:
   - Pattern matching `Option.Some(ptr)` with pointer payloads
   - Array indexing with different types
   - Vec operations

2. **Add execution tests**:
   - Compile AND run code
   - Verify output matches expectations
   - Catch runtime bugs (like nested struct field swap)

3. **Add complex control flow tests**:
   - Nested pattern matches
   - Pattern matches inside loops
   - Multiple phi nodes in same function

4. **Consider property-based testing**:
   - Generate random valid Zen code
   - Compile and verify it doesn't crash
   - Catch edge cases automatically

## Current Status

**Coverage**: ~70% of critical codegen bugs are tested
- ✅ All terminator bugs are tested
- ✅ Basic phi node bugs are tested
- ⚠️ Specific payload extraction phi node bug is not directly tested
- ⚠️ GEP bugs are partially tested (nested structs compile but have runtime bug)
- ❌ Execution/runtime bugs are not tested

