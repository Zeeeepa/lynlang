# Nested Generics Issue Analysis

## Problem Statement
Nested generic types like `Result<Result<i32, string>, string>` are not extracting payloads correctly. When extracting the inner value (42) from nested Results, we get 0 instead.

## Test Results

### Working Case
```zen
inner = Result.Ok(42)
outer = Result.Ok(inner)
// Pattern matching on outer -> inner -> 42 works correctly
```

### Broken Cases
```zen
// Case 1: Direct nesting
outer = Result.Ok(Result.Ok(42))
// Pattern matching gets 0 instead of 42

// Case 2: Function return
get_inner = () Result<i32, string> { Result.Ok(42) }
outer = Result.Ok(get_inner())
// Pattern matching gets 0 instead of 42
```

## Root Cause Analysis

The issue is related to memory management and pointer validity:

1. When `Result.Ok(42)` is created:
   - 42 is heap-allocated at address A
   - Result struct {tag=0, ptr=A} is created on stack
   - The struct is loaded and returned as a VALUE (not pointer)

2. When this struct becomes a payload for outer `Result.Ok(inner)`:
   - The struct value is heap-allocated at address B
   - This copies {tag=0, ptr=A} to heap location B
   - The outer Result stores ptr=B

3. During extraction:
   - Outer Result extracts ptr=B correctly
   - Loading from B gives {tag=0, ptr=A}
   - BUT: When we try to load from ptr=A, we get 0 instead of 42

## Hypothesis

The pointer to the heap-allocated 42 (ptr=A) is somehow becoming invalid when the Result struct is used as a temporary value. Possible reasons:

1. **Memory corruption**: The heap memory at A is being overwritten
2. **Pointer context**: The pointer was created in a different context and isn't valid when accessed later
3. **Double heap allocation**: The struct is being heap-allocated twice, and the pointers aren't being updated correctly
4. **LLVM optimization**: LLVM might be optimizing away the temporary values

## Current Implementation

- Result/Option enums are stack-allocated initially
- Payloads are always heap-allocated
- When a Result/Option struct is used as a payload in another enum, it gets heap-allocated
- The struct copying preserves the pointer values but not necessarily their validity

## Solutions Attempted

1. **Always heap-allocate Result/Option** - Didn't fix the issue
2. **Return pointers instead of values** - Didn't fix the issue
3. **Various debug outputs** - Confirmed heap allocation is happening but values are lost

## Next Steps

1. Check if this is an LLVM optimization issue
2. Consider using a different memory allocation strategy
3. Investigate if we need to "deep copy" nested structures
4. Look into using reference counting or GC for nested heap allocations