# Generic Type System Improvements

## Summary
This document outlines the improvements made to handle nested generic types in the Zen language compiler, particularly focusing on `Result<Result<T,E>,E2>` and `Option<Option<T>>` patterns.

## Changes Made (2025-09-25)

### 1. Enhanced Generic Type Tracking
- **File**: `src/codegen/llvm/generics.rs`
- Implemented `GenericTypeTracker` with recursive type tracking
- Added support for deeply nested generics with proper context management
- Tracks type arguments for Result, Option, Array, Vec, HashMap, HashSet

### 2. Improved Heap Allocation for Nested Enums  
- **File**: `src/codegen/llvm/expressions.rs` (compile_enum_variant)
- When a Result/Option contains another Result/Option as payload:
  - The nested struct is heap-allocated to preserve the entire structure
  - Both discriminant and payload pointer are preserved
  - Prevents stack corruption of nested enum payloads

### 3. Enhanced raise() Expression Handling
- **File**: `src/codegen/llvm/expressions.rs` (compile_raise_expression)
- Added support for tracking types from:
  - Function calls returning Result<T,E>
  - Variables containing Result<T,E>
  - Generic expressions
- Properly updates generic type context when extracting nested types
- Tracks "Last_Raise_Extracted_Type" for variable type inference

### 4. Nested Generic Payload Extraction
- **File**: `src/codegen/llvm/expressions.rs` (raise payload extraction)
- When extracting nested Result/Option from outer Result:
  - Loads the entire struct from heap (not just the payload)
  - Updates generic type context for subsequent raise() calls
  - Preserves struct type information

## Current Status

### Working ✅
- Basic generic types: `Result<T,E>`, `Option<T>`
- Simple nested generics in direct expressions
- Pattern matching on nested generics
- Heap allocation of nested enum structs

### Partially Working ⚠️
- Nested generics through function returns
- Variables containing nested Results from raise()
- Type inference for deeply nested generics

### Not Working ❌  
- Variables assigned from raise() losing Result type information
- Calling raise() on variables containing nested Results
- Deep nesting beyond 2 levels

## Test Results
- **Pass Rate**: 237/248 tests (95.6%)
- **Failing Tests**: 11 (all related to nested raise operations)
- **Main Error**: "Unsupported Result type for .raise()" when calling raise() on variables

## Root Cause Analysis

The fundamental issue is in the type system's handling of variables:

1. When `inner = get_nested().raise()` executes:
   - raise() correctly extracts Result<i32, string> from Result<Result<i32, string>, string>
   - The extracted value is properly heap-allocated and loaded
   - BUT: The variable `inner` is not typed as Result<i32, string>

2. When `inner.raise()` is then called:
   - The compiler looks up `inner` in the symbol/variable table
   - It finds an i32 type instead of Result<i32, string>
   - raise() fails with "Unsupported Result type"

## Recommended Next Steps

1. **Fix Variable Type Tracking**:
   - Update variable type when assigned from raise()
   - Store generic type information in variable symbol table
   - Ensure `self.variables` map is updated with correct AstType

2. **Improve Type Inference**:
   - Make infer_expression_type() check generic_type_context
   - Add fallback to "Last_Raise_Extracted_Type" for variables
   - Consider adding a separate type tracking system for generic variables

3. **Add Debug Instrumentation**:
   - Log variable types when created/updated
   - Track generic type flow through the compiler
   - Add assertions for type consistency

## Code Examples

### Working Pattern Matching
```zen
outer ? | Result.Ok(inner) => {
    inner ? | Result.Ok(val) => val
           | Result.Err(e) => ...
}
```

### Failing raise() Pattern
```zen
inner = get_nested().raise()  // Should be Result<i32, string>
value = inner.raise()          // FAILS: inner not recognized as Result
```

## Technical Details

### Enum Struct Representation
```
Result<T,E> struct {
    discriminant: i64,     // 0 for Ok, 1 for Err
    payload: ptr           // Pointer to heap-allocated T or E
}
```

### Nested Result Storage
```
Result<Result<T,E>,E2> {
    discriminant: i64,
    payload: ptr -> Result<T,E> struct {
        discriminant: i64,
        payload: ptr -> T or E
    }
}
```

## Conclusion

The generic type system improvements have made significant progress in handling nested generics. The main remaining challenge is ensuring that variable type information is properly tracked and preserved through raise() operations. Once this is resolved, the nested generic support will be fully functional.