# Known Compiler Issues & Regression Tests

This directory contains test files that expose known bugs or limitations in the Zen compiler. These tests are currently excluded from the main test suite but should be run periodically to prevent regressions.

## Issue Tracking

### 1. HashMap Method Resolution on References
**File**: `test_hashmap_dynvec_get.zen`
**Status**: Unresolved
**Error**: "Not yet implemented" during compilation
**Description**: Cannot call methods on references returned from generic types (e.g., `HashMap.get()` returning `Option<&V>`)
**Impact**: Cannot chain method calls when working with HashMap values
**Root Cause**: Method resolution doesn't work through references for generic type parameters

---

### 2. Closure Compilation Issues
**File**: `zen_test_closures.zen`  
**Status**: Partial - Many closure tests fail
**Error**: "Unknown function: @std_io_println"
**Description**: Multiple issues with closure compilation and typechecking
**Known Skipped Tests**:
- `test_closure_raise()` - LLVM verification error with `.raise()` in closures
- `test_closure_string()` - Type error with string concatenation in closures
**Impact**: Closures with certain features (raise, string ops) cause compiler errors

---

### 3. Struct Declaration Parsing
**File**: `zen_test_structs_parse_error.zen`
**Status**: Unresolved
**Error**: "Parse error: Syntax Error: Expected ')' after closure parameters at line 154"
**Description**: Parser fails on method declarations with `self` parameter in certain contexts
**Line**: Line 154 is the `calculate_area` method definition with `(self: Rectangle) i32`
**Impact**: Struct methods with explicit `self` parameter fail to parse
**Workaround**: None - affects method definition syntax

---

## Testing Regression Issues

To test these files:

```bash
# Try to compile individual files
zen tests/regression_known_issues/test_hashmap_dynvec_get.zen
zen tests/regression_known_issues/zen_test_closures.zen
zen tests/regression_known_issues/zen_test_structs_parse_error.zen
```

Expected behavior: All should fail with the documented errors above.

## When Fixes Are Applied

1. Fix the compiler issue
2. Run the test file to confirm it compiles and executes correctly
3. Move the test back to `tests/` directory
4. Update this README to document the fix and commit hash
5. Ensure the test passes in the main test suite

## Priority

- **HIGH**: Struct method declarations (zen_test_structs_parse_error.zen)
- **MEDIUM**: HashMap method resolution (test_hashmap_dynvec_get.zen)
- **MEDIUM**: Closure edge cases (zen_test_closures.zen)
