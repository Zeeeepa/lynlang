# GEP (GetElementPointer) Audit Report

## Summary
Comprehensive audit of all GEP operations in the codegen module to ensure struct types match indexed values and operations are type-safe.

## GEP Usage Locations

### Critical Issues Found

#### 1. **vec_support.rs (Line 41-46, 88-98)**
**Issue**: Using `build_in_bounds_gep` with multi-level indices requires correct struct type information.

**Analysis**:
- Line 89: `vec_struct_type` extracted from `vec_value.into_struct_value().get_type()`
- Lines 81-85: Indices for nested struct access: `[0, 0, current_len]`
  - Index 0 (struct pointer)
  - Index 0 (array field at position 0)
  - current_len (element in array)
- **Status**: ✅ CORRECT - Type is properly extracted and indices are validated

#### 2. **structs.rs (Line 51-56)**
**Issue**: Basic struct field access using `build_struct_gep`.

**Analysis**:
- Line 51-56: Field pointer lookup in struct
  - `llvm_type` is the struct type
  - `field_index` comes from struct definition
  - Index properly clamped to u32
- **Status**: ✅ CORRECT - Field indices match struct definition order

#### 3. **structs.rs (Line 231-240)**
**Issue**: Field access during struct member reading.

**Analysis**:
- Similar to above, uses field indices from struct schema
- **Status**: ✅ CORRECT

#### 4. **collections_index.rs (Line 27-35)**
**Problem**: Uses `unsafe` GEP with generic `build_gep` on i8 pointers.

```rust
let gep = unsafe {
    compiler.builder.build_gep(
        i8_type,
        ptr.into_pointer_value(),
        &[offset_i64],
        "gep_tmp",
    )
};
```

**Issues**:
- Using `i8_type` as the base type for GEP on a generic pointer
- No validation that offset is in bounds
- Assumes pointer arithmetic on bytes without bounds checking

**Recommendation**: Add bounds validation or document unsafe assumptions

#### 5. **pointers.rs (Line 208-212)**
**Issue**: Pointer arithmetic GEP operation.

**Analysis**:
- Similar to collections_index.rs - uses byte-level pointer arithmetic
- **Status**: ⚠️ NEEDS BOUNDS VALIDATION

#### 6. **functions/stdlib/compiler.rs (Line 199)**
**Issue**: GEP on i8 pointer for offset arithmetic.

```rust
compiler.builder.build_gep(i8_type, ptr.into_pointer_value(), &[offset_i64], "offset_ptr")?
```

**Status**: ⚠️ SIMILAR TO ABOVE - No bounds checking

### Patterns Found

#### Safe Pattern (Used in most code)
```rust
let field_ptr = self.builder.build_struct_gep(
    struct_type,           // Source type
    struct_ptr,            // Pointer value
    field_index as u32,    // Field index from schema
    "field_ptr"
)?;
```

✅ **Good** - Type system ensures correctness

#### Unsafe Pattern (Pointer arithmetic)
```rust
unsafe {
    compiler.builder.build_gep(
        i8_type,           // Assume i8* for byte arithmetic
        ptr,
        &[offset],
        "gep_ptr"
    )
}
```

⚠️ **Problem** - No bounds validation, manual offset management

## Recommendations

### Priority 1: Add Bounds Checking
For pointer arithmetic operations (especially in collections_index.rs, pointers.rs, compiler.rs):
```rust
// Validate offset stays within allocated memory
if offset > allocated_size {
    return Err(CompileError::InternalError(
        "Pointer offset out of bounds".to_string(),
        span
    ));
}
```

### Priority 2: Create GEP Helper Function
```rust
fn safe_pointer_offset(
    &self,
    base_ptr: PointerValue,
    offset: IntValue,
    allocated_size: u64,
    name: &str,
) -> Result<PointerValue, CompileError> {
    // Validate offset
    let max_offset = self.context.i64_type().const_int(allocated_size, false);
    let exceeds_bounds = self.builder.build_int_compare(
        IntPredicate::UGT,
        offset,
        max_offset,
        "bounds_check"
    )?;
    
    // Build conditional error handling
    // ...
    
    unsafe {
        self.builder.build_gep(
            self.context.i8_type(),
            base_ptr,
            &[offset],
            name
        )
    }
}
```

### Priority 3: Type-Safe GEP in Structs
For nested struct access, ensure we always have the correct struct type:

```rust
// ✅ Good
let struct_type = self.get_struct_type(struct_name)?;
let field_ptr = self.builder.build_struct_gep(
    struct_type,
    struct_ptr,
    field_index,
    "field_ptr"
)?;

// ❌ Avoid
let field_ptr = unsafe {
    self.builder.build_gep(ptr_type, ptr, &[index], "field_ptr")?
};
```

## Test Coverage Needed

Create tests in `tests/codegen/gep_operations.rs`:

```rust
#[test]
fn test_struct_field_gep_single_level() { }

#[test]
fn test_struct_field_gep_nested() { }

#[test]
fn test_array_gep_bounds_checked() { }

#[test]
fn test_pointer_arithmetic_offset() { }

#[test]
fn test_gep_out_of_bounds_detection() { }
```

## Files to Audit More Thoroughly

1. **collections_index.rs** - Needs bounds validation
2. **pointers.rs** - Needs bounds validation
3. **functions/stdlib/compiler.rs** - Needs bounds validation
4. **expressions/utils.rs** - Has GEP operations for enum field access

## Status
- [x] Reviewed main GEP locations
- [x] Identified safe vs. unsafe patterns
- [ ] Implement bounds checking helpers
- [ ] Add comprehensive tests
- [ ] Update stdlib to use compiler GEP primitives

