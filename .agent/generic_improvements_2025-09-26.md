# Generic Type System Improvements - September 26, 2025

## Summary
Fixed critical issues with generic types and Array<T> pattern matching that were causing segfaults.

## Key Problems Identified

1. **Inconsistent Option Struct Layouts**
   - Array.pop() was creating Option structs with direct payload storage: `struct { i64 discriminant, i32 payload }`
   - Pattern matching expected Option structs with pointer payloads: `struct { i64 discriminant, ptr payload }`
   - This mismatch caused segfaults when pattern matching on Option values returned from Array methods

2. **Incorrect Discriminant Values**
   - Array.pop() had discriminants reversed (Some=1, None=0 instead of Some=0, None=1)
   - This caused pattern matching to fail even when struct layouts matched

3. **Lack of Generic Type Tracking for Arrays**
   - Array<T> methods weren't properly tracking the element type T
   - Pattern matching couldn't determine the correct payload type for Option<T> when T came from Array<i32>

## Solutions Implemented

### 1. Fixed Option Discriminant Values
- Changed Array.pop() to use correct discriminants:
  - Some = 0
  - None = 1
- This aligns with the convention used throughout the compiler

### 2. Unified Option Struct Layout  
- Modified Array.pop() to use pointer-based payload storage
- Now allocates payload values on heap and stores pointers
- Creates consistent struct layout: `struct { i64 discriminant, ptr payload }`
- This matches what pattern matching expects for generic enums

### 3. Improved Memory Management
- Added proper malloc calls for payload allocation in Array.pop()
- Uses pointer casting to store i32 values in allocated memory
- None variant uses null pointer for payload

## Test Results

### Fixed Tests
- `test_array_methods.zen` - Now passes completely
- `test_array_methods_simple.zen` - Works correctly  
- `test_array_pop_debug.zen` - Pattern matching successful
- All basic Array<i32> operations with Option pattern matching now work

### Still Failing  
- `test_array_with_generics.zen` - Nested generics (Array<Option<i32>>) still need work
- `test_collections.zen` - Complex collection types still have issues

## Technical Details

### Code Changes in expressions.rs:

1. **Array.pop() Method (lines 6921-7060)**:
   - Fixed discriminant values (Some=0, None=1)
   - Added malloc for payload allocation
   - Changed to pointer-based payload storage
   - Unified Option struct type for both Some and None branches

2. **Key Insights**:
   - Generic enums in this compiler use pointer payloads for type erasure
   - This allows a single struct layout to handle different payload types
   - Pattern matching code dereferences these pointers based on tracked type info

## Remaining Work

1. **Full Generic Monomorphization**
   - Need to properly instantiate generic types with concrete type arguments
   - Array<Option<i32>> requires nested generic support

2. **Generic Type Registry**
   - Option and Result should be registered as proper generic enums
   - Need consistent handling across all code paths

3. **Type Tracking Enhancement**  
   - Better tracking of generic type parameters through method calls
   - Proper type inference for nested generics

## Impact

- Eliminates segfaults in Array pattern matching scenarios
- Makes Array<T> methods usable with Option<T> return types
- Improves overall stability of generic type system
- Sets foundation for more comprehensive generic support

## Next Steps

1. Implement proper generic type instantiation for nested types
2. Add comprehensive generic type registry
3. Enable disabled generic-related tests
4. Extend solution to other collection types (Vec, HashMap, etc.)

## Files Modified
- `src/codegen/llvm/expressions.rs` - Fixed Array.pop() Option struct generation