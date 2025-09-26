# Custom Enum in Generics Fix

## Problem
Custom enums lose their payload values when nested inside generics like Option<MyEnum> or Result<MyEnum, E>.

### Example:
```zen
MyResult : .Success(val: i32) | .Error(msg: string)
wrapped = Option.Some(MyResult.Success(42))
// When extracting: gets 0 instead of 42
```

## Root Cause
In `patterns.rs` line 354-358, we look up the payload type for custom enums but:
1. The type isn't properly tracked in generic_type_context
2. Custom enums are treated differently than built-in Option/Result
3. When loading payload from pointer, we default to i32 without checking actual custom enum structure

## Solution Approach

### 1. Track Custom Enum Types in Generic Contexts
When storing a custom enum in Option.Some or Result.Ok/Err, we need to:
- Store the entire enum type info in generic context
- Track the enum's variant payload types
- Preserve this information through pattern matching

### 2. Enhanced Payload Extraction
For custom enums inside generics:
- Load the entire enum struct (discriminant + payload pointer)
- Use the stored type information to correctly extract payload
- Handle recursive extraction for nested custom enums

### 3. Type Registration
Register custom enum types similar to how we register Option/Result:
- Store enum variant info in symbol table
- Track payload types for each variant
- Make this accessible during pattern matching

## Implementation Steps

1. **In expressions.rs when creating Option.Some(custom_enum):**
   - Track that Option_Some_Type is a custom enum
   - Store the custom enum's type info

2. **In patterns.rs when matching Option.Some(custom):**
   - Check if payload is a custom enum
   - Load as enum struct, not just pointer
   - Use proper type info for extraction

3. **Add custom enum support to GenericTypeTracker:**
   - Track custom enum variants
   - Store payload type mappings
   - Handle nested custom enums