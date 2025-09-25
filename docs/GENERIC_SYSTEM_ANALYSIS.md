# Generic Type System Improvements and Limitations

## Current Status (2025-09-25)

### Working Features
- Basic generic types (Result<T,E>, Option<T>) work correctly
- Single-level generic payload extraction works
- Type tracking for generic instantiation is functional
- HashMap<K,V>, HashSet<T>, Array<T>, DynVec<T> basic support

### Known Limitations and Issues

#### 1. Nested Generic Type Extraction
**Problem**: When nesting generic types like `Result<Result<T,E>, E2>` or `Option<Result<T,E>>`, payload extraction fails.

**Symptoms**:
- Extracted values return 0 or None instead of actual values
- PHI node type mismatches in LLVM when pattern matching nested generics

**Root Cause**: 
- Heap-allocated nested enum structs lose payload information during extraction
- Pattern matching type inference doesn't properly handle nested return types
- PHI nodes require consistent types across all branches, but nested patterns return inner types

**Example of failing code**:
```zen
inner = Result.Ok(42)
outer = Result.Ok(inner)
outer ? | Result.Ok(extracted) => {
    extracted ? | Result.Ok(val) => val  // Returns i32, but outer expects Result struct
                | Result.Err(_) => -1
}
```

#### 2. Deep Generic Type Tracking
The current `GenericTypeTracker` and `generic_type_context` provide basic tracking but don't fully support:
- Recursive type parameter resolution
- Complex nested type inference
- Type parameter constraints

### Required Improvements

1. **Nested Enum Payload Handling**
   - Fix heap allocation and extraction of nested enum structs
   - Ensure payload pointers are correctly dereferenced for nested types
   - Maintain type information through multiple levels of nesting

2. **Pattern Matching Type System**
   - Improve type inference for nested pattern matching
   - Handle type coercion or wrapping for consistent PHI node types
   - Consider introducing intermediate values to maintain type consistency

3. **Generic Monomorphization**
   - Implement proper generic function specialization
   - Generate specialized code for each concrete type instantiation
   - Cache monomorphized versions to avoid duplication

4. **Type Parameter Constraints**
   - Add support for trait/behavior bounds on generic types
   - Implement type checking for generic constraints
   - Enable better error messages for generic type mismatches

### Workaround Strategies

Until these improvements are implemented, users can work around the limitations by:

1. **Avoiding deeply nested generics** - Use separate variables for each level:
```zen
inner = Result.Ok(42)
outer = Result.Ok(inner)

// Instead of nested pattern matching, extract step by step
inner_extracted = outer ? | Result.Ok(v) => v
                          | Result.Err(_) => Result.Err("outer error")

final_value = inner_extracted ? | Result.Ok(v) => v
                                | Result.Err(_) => -1
```

2. **Using wrapper structs** for complex nested types
3. **Limiting generic usage** to single-level types where possible

### Implementation Priority

1. **High**: Fix nested payload extraction (blocks test_raise_nested_result.zen)
2. **Medium**: Improve pattern matching type inference
3. **Low**: Add generic monomorphization and constraints

### Tests Affected
- test_raise_nested_result.zen.disabled
- test_raise_simple_nested.zen.disabled  
- zen_test_comprehensive_working.zen.disabled (partially)
- zen_test_raise_consolidated.zen.disabled_still_broken (partially)