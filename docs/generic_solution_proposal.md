# Generic Type System Solution Proposal

## Problem
When we have this code:
```zen
str_result = Result.Ok("hello")  // Sets Result_Ok_Type = String globally
int_result = Result.Ok(314159)   // Still uses Result_Ok_Type = String, causing corruption
```

## Root Cause
We're tracking generic types globally with keys like "Result_Ok_Type", which gets overwritten when processing different Result instances.

## Solution: Variable-Specific Generic Tracking

Instead of:
- `Result_Ok_Type` -> used for ALL Result types in function

We need:
- `str_result_Ok_Type` -> specific to str_result variable
- `int_result_Ok_Type` -> specific to int_result variable

## Implementation Plan

1. When compiling a variable assignment with a generic type:
   - Track the generic type parameters with the variable name as prefix
   - Example: `self.track_generic_type("str_result_Ok_Type", AstType::String)`

2. When pattern matching on a variable:
   - Look up the type using the variable name
   - Example: `self.generic_type_context.get("str_result_Ok_Type")`

3. For inline expressions without variables:
   - Use a counter or expression ID to create unique keys
   - Example: `self.track_generic_type("expr_123_Ok_Type", type)`

## Benefits
- No pollution between different Result/Option instances
- Proper type tracking for nested generics
- Works with variables, function returns, and inline expressions