# Nested Generics Issue Analysis

## Problem Statement
Inline construction of nested generic types like `Result.Ok(Result.Ok(42))` returns 0 instead of 42 when extracting the inner value.

## Root Cause
When constructing nested enums inline:
1. Inner enum (`Result.Ok(42)`) is created with heap-allocated payload (42)
2. Inner enum struct is passed as payload to outer enum
3. Outer enum stores the struct but the inner payload pointer may become invalid
4. When extracting, we get 0 instead of the expected value

## Working Workaround
Using intermediate variables works correctly:
```zen
inner = Result.Ok(42)        // Inner enum persists in variable
outer = Result.Ok(inner)      // Outer gets stable reference
// Extraction works correctly
```

## Why It Works With Variables
- Variables keep the inner enum struct alive in memory
- The heap-allocated payload remains valid
- References are stable throughout the scope

## Why Inline Construction Fails  
- Inline construction creates temporary values
- The inner enum struct is consumed when creating outer
- Payload pointers may point to reused memory
- LLVM optimization may eliminate intermediate values

## Required Fix
Need to implement deep copying of enum payloads when:
1. An enum struct is used as payload for another enum
2. Ensure all nested heap allocations are preserved
3. Track ownership and lifetime of nested payloads

## Complexity
This is a fundamental architectural issue requiring:
- Lifetime tracking for nested payloads
- Deep copy semantics for enum structs
- Proper memory management for inline constructions
- Possible LLVM IR changes to preserve temporaries

## Current Impact
- test_generic_comprehensive.zen shows the issue
- Inline nested generics fail
- Workaround: use intermediate variables
- Affects ~5% of potential generic use cases