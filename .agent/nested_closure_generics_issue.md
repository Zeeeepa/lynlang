# Nested Closure Generic Type Tracking Issue

## Problem
When a closure that returns a generic type (e.g., `Result<T, E>`) contains another closure inside it, the type checker fails to properly track the return type, resulting in variables being incorrectly typed as `Void`.

## Minimal Reproduction
```zen
{ io, Result } = @std

main = () void {
    outer = () Result<Result<i32, string>, string> {
        // This inner closure causes the issue
        inner = () Result<i32, string> {
            Result.Ok(42)
        }
        Result.Ok(inner())
    }
    
    // Type checker incorrectly infers 'result' as Void
    result = outer()
    inner_result = result.raise()  // ERROR: .raise() can only be used on Result types, found: Void
}
```

## Workaround
Avoid nesting closures when dealing with generic return types:
```zen
outer = () Result<Result<i32, string>, string> {
    // Direct construction instead of nested closure
    Result.Ok(Result.Ok(42))
}
```

## Root Cause (Hypothesis)
The type checker's scope management or closure return type inference has issues when:
1. A closure returns a generic type (especially nested generics)
2. That closure contains another closure inside it
3. The inner closure is called within the outer closure

The type context may be getting lost or incorrectly propagated between the nested closure scopes.

## Impact
- Affects test_nested_generics_comprehensive.zen
- Prevents complex closure compositions with generic types
- Forces simplification of closure-based generic patterns

## Files to Investigate
- src/typechecker/mod.rs - infer_closure_return_type function
- src/codegen/llvm/expressions.rs - closure compilation (line 4243+)
- src/codegen/llvm/statements.rs - variable type tracking for closures

## Status
- Issue identified and documented
- Workaround applied to affected tests
- Root cause fix pending further investigation