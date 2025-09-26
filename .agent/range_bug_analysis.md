# Range Variable Bug Analysis

## Problem
When creating a range with a variable end value (e.g., `0..x` where `x` is a variable), LLVM verification fails with:
```
LLVM verification error: Use of instruction is not an instruction!
  %end_cast = sext i32 %x1 to i64
```

## Root Cause
1. When we declare `x = 3`, it creates an alloca named `%x`
2. When we load `x` for the range expression, LLVM wants to name the loaded value `%x` but that's taken
3. LLVM auto-numbers it to `%x1`
4. But something in our code generation is breaking the SSA chain, making `%x1` not recognized as a valid instruction

## Test Cases
- Works: `0..3` (literal range)
- Works: Loading and using variable in expressions
- Fails: `0..x` where x is a variable
- Fails: Both i32 and i64 variables

## Investigation Path
1. compile_identifier loads the variable correctly
2. compile_range_expression receives the loaded value
3. When trying to cast to i64, the build_int_cast fails because the input value is not recognized

## Potential Solutions
1. Ensure loaded values are properly tracked in SSA form
2. Use different naming scheme for loaded values vs allocas
3. Check if there's a basic block issue (values from different blocks)