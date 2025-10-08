# Known Compiler Bugs

This directory contains test files that expose known bugs in the Zen compiler. These tests are excluded from the main test suite until the bugs are fixed.

## Active Bugs

### 1. Nested Struct Field Access Bug (CRITICAL)
**File**: `nested_struct_field_bug.zen`
**Status**: Unresolved
**Severity**: High

**Description**:
When accessing fields from nested structs (struct containing another struct), the compiler incorrectly swaps or misidentifies field values, particularly for the second field (y) of nested Point structs.

**Example**:
```zen
Point: { x: f64, y: f64 }
Rectangle: { top_left: Point, bottom_right: Point }

rect = Rectangle {
    top_left: Point { x: 0.0, y: 0.0 },
    bottom_right: Point { x: 10.0, y: 5.0 }
}

// BUG: These access wrong values
r.bottom_right.y  // Returns 0.0 instead of 5.0
r.top_left.y      // Returns 10.0 instead of 0.0
```

**Expected**:
- `bottom_right.y` should return 5.0
- `top_left.y` should return 0.0

**Actual**:
- `bottom_right.y` returns 0.0
- `top_left.y` returns 10.0

**Impact**: Any code using nested structs with multiple fields will produce incorrect results.

**Workaround**: None currently. Avoid nested struct patterns until fixed.

**Root Cause**: Likely in struct field offset calculation or GEP (GetElementPtr) generation in LLVM codegen for nested struct access.

**Fix Required**: Investigate `src/compiler/expressions.rs` struct field access codegen, particularly for nested field access chains.

---

### 2. Method Resolution on References
**File**: `test_hashmap_dynvec_get.zen`
**Status**: Unresolved
**Severity**: Medium

**Description**:
Cannot call methods on references returned from generic types. When `HashMap.get()` returns `Option<&V>`, we cannot call methods on the `&V` reference inside the pattern match.

**Example**:
```zen
list = map.get(1)  // Returns Option<&DynVec<String>>
list ?
    | Some(vec_ref) {
        first = vec_ref.get(0)  // ERROR: Undeclared function: 'get'
    }
```

**Expected**: Should resolve `get` method on `DynVec<String>` through the reference.

**Actual**: Error: "Undeclared function: 'get'"

**Impact**: Cannot chain method calls when working with HashMap values.

**Workaround**: None currently. Would need to dereference or copy the value.

**Root Cause**: Method resolution (UFC - Uniform Function Call) doesn't work through references for generic type parameters.

**Fix Required**: Enhance method resolution in typechecker to handle references to generic types.

---

## Bug Resolution Process

1. When a bug is fixed, move the test file from `tests/known_bugs/` back to `tests/`
2. Update this README to mark the bug as resolved with the commit hash
3. Ensure the test passes before moving it
