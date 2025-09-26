# Nested Generics Fix Analysis

## Problem
When we have nested generic types like `Result<Result<i32, string>, string>`, the inner generic is not being properly extracted.

### Test Case
```zen
// Function returns Result<Result<i32, string>, string>
nested_result = () Result<Result<i32, string>, string> {
    inner = Result.Ok(42)
    Result.Ok(inner)
}
// When extracting, inner Result.Ok(42) returns garbage value instead of 42
```

## Root Cause

1. When creating Result.Ok(Result.Ok(42)):
   - Inner Result.Ok(42) is created as struct {tag=0, payload=ptr_to_42}
   - This struct is heap-allocated (malloc'd) to preserve it
   - Outer Result.Ok stores pointer to heap-allocated inner Result

2. When pattern matching on outer Result:
   - Correctly extracts inner Result struct
   - BUT when pattern matching on inner Result:
     - The struct is loaded correctly
     - But the payload pointer is not being dereferenced with the correct type
     - Generic type context is lost between outer and inner pattern match

## Solution Approach

1. **Enhanced Type Tracking**: When extracting a nested generic payload, preserve and update the generic type context
2. **Recursive Type Resolution**: When loading payload from nested generics, recursively resolve the actual payload type
3. **Context Propagation**: Pass generic type context through pattern match branches

## Implementation Strategy

### Step 1: Track Nested Generic Context
- When extracting Result<Result<T,E>,E2>, track:
  - Result_Ok_Type = Result<T,E>
  - Result_Ok_Ok_Type = T
  - Result_Ok_Err_Type = E

### Step 2: Update Pattern Extraction
- In patterns.rs, when extracting payload that is itself a generic:
  - Store the extracted struct in a variable with proper type tracking
  - Update generic_type_context for the nested type

### Step 3: Fix Payload Dereferencing
- When loading payload from nested Result/Option:
  - Check if payload type is Generic
  - If so, properly handle as struct with its own discriminant and payload