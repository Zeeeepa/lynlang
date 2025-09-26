# Result<T,E> Return Type Issue Analysis
Date: 2025-09-25

## Summary
We successfully re-enabled 6 Result<T,E> tests that now work, but 5 tests remain disabled due to a specific compiler issue with Result return types in conditional branches.

## Tests Re-enabled (Now Working)
1. **simple_error_test.zen** - Uses Result types but doesn't have the conditional return issue
2. **test_debug_result_return.zen** - Simple Result returns work
3. **test_error_propagation_consolidated.zen** - .raise() operations work correctly
4. **test_pattern_match_value.zen** - Pattern matching extracting values works
5. **test_result_match_simple.zen** - Result type returns in simple cases work
6. **zen_test_result_pattern.zen** - Complex Result pattern matching works

## Tests Still Failing
1. **test_debug_block_return.zen** - Returns Result<T,E> from conditional branches
2. **test_error_propagation.zen** - Complex error propagation with custom enums
3. **test_generic_result_types.zen** - Generic Result types with multiple type parameters
4. **test_raise_float.zen** - Result<f64,E> types
5. **test_collections.zen** - Uses features like 'as' casting not yet implemented

## Root Cause
The LLVM verification error shows:
```
Function return type does not match operand type of return inst!
  ret i32 %result7
 { i64, ptr }
```

The compiler issue occurs when:
1. A function is declared with return type `Result<T,E>`
2. The function has conditional branches (pattern matching)
3. Each branch tries to return a Result value with `return Result.Ok(x)` or `return Result.Err(e)`
4. The compiler creates PHI nodes but doesn't properly convert the Result enum to the LLVM struct type

## Working Pattern
```zen
// This works - Result used in pattern matching
test = () i32 {
    r = Result.Ok(42)
    r ? 
        | .Ok(v) { return v }
        | .Err(e) { return 0 }
}
```

## Failing Pattern
```zen
// This fails - returning Result from conditional
test = () Result<i32, string> {
    x == 1 ?
        | true { return Result.Ok(42) }    // LLVM type mismatch here
        | false { return Result.Err("err") }
}
```

## Compiler Fix Needed
The compiler needs to:
1. Detect when a function returns a generic type like Result<T,E>
2. In compile_return(), convert the enum variant to the proper LLVM struct type
3. Ensure PHI nodes use the correct struct type, not the discriminant value

## Impact
- 5 tests remain disabled
- Error handling patterns limited
- Can work around by avoiding conditional returns of Result types