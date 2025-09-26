# Zen Generic Type System

## Overview
The Zen language supports a comprehensive generic type system that enables type-safe, reusable code with compile-time type checking and LLVM-based code generation.

## Supported Generic Types

### Option<T>
Represents an optional value that can be `Some(value)` or `None`.

```zen
opt1: Option<i32> = Option.Some(42)
opt2: Option<string> = Option.None

opt1 ?
    | Option.Some(v) { io.println("Value: ${v}") }
    | Option.None { io.println("No value") }
```

### Result<T, E>
Represents a result that can be `Ok(value)` or `Err(error)`.

```zen
res1: Result<i32, string> = Result.Ok(100)
res2: Result<f64, string> = Result.Err("Math error")

res1 ?
    | Result.Ok(v) { io.println("Success: ${v}") }
    | Result.Err(e) { io.println("Error: ${e}") }
```

### Array<T>
Dynamic array with type-safe operations.

```zen
arr: Array<i32> = Array.new(10, 0)
arr.push(42)
arr.push(100)
io.println("Length: ${arr.len()}")
io.println("First: ${arr.get(0)}")
```

### HashMap<K, V>
Hash map with generic key and value types.

```zen
map: HashMap<string, i32> = HashMap<string, i32>.new()
map.insert("key", 42, hash_fn, eq_fn)
val = map.get("key", hash_fn, eq_fn)
```

## Generic Functions

### Functions with Generic Parameters
Functions can accept generic types as parameters:

```zen
process_option = (opt: Option<i32>) void {
    opt ?
        | Option.Some(v) { io.println("Processing: ${v}") }
        | Option.None { io.println("Nothing to process") }
}

process_result = (res: Result<i32, string>) void {
    res ?
        | Result.Ok(v) { io.println("Result: ${v}") }
        | Result.Err(e) { io.println("Error: ${e}") }
}
```

### Functions Returning Generic Types
Functions can return generic types:

```zen
create_option = (val: i32) Option<i32> {
    // Important: Store in variable before returning (workaround for known issue)
    result = val > 0 ?
        | true { Option.Some(val) }
        | false { Option.None }
    return result
}
```

## Pattern Matching with Generics
Pattern matching works seamlessly with generic types:

```zen
value: Option<i32> = Option.Some(42)

// Direct pattern matching
value ?
    | Option.Some(v) { io.println("Got ${v}") }
    | Option.None { io.println("Got nothing") }

// Pattern matching with expressions
result = value ?
    | Option.Some(v) { v * 2 }
    | Option.None { 0 }
```

## Implementation Details

### GenericTypeTracker
The compiler uses a `GenericTypeTracker` to manage generic type instantiations:
- Tracks type parameters at different scopes
- Supports nested generics (e.g., `Result<Option<T>, E>`)
- Manages type monomorphization for LLVM codegen

### LLVM Representation
Generic types are compiled to LLVM struct types:
- `Option<T>` and `Result<T,E>`: Represented as `{i64 discriminant, ptr payload}`
- `Array<T>`: Represented as `{ptr data, i64 length, i64 capacity}`
- `HashMap<K,V>`: Represented as `{ptr buckets, i64 size, i64 capacity}`

### Memory Management
- Enum payloads are heap-allocated to ensure persistence
- Uses malloc for dynamic allocation
- Proper handling of nested generic types

## Known Limitations

### Direct Return from Closures
**Issue**: Directly returning an enum variant from a closure loses the payload value.

```zen
// PROBLEMATIC - payload gets lost
make_option = () Option<i32> {
    Option.Some(42)  // Returns Some(0) instead of Some(42)
}

// WORKAROUND - store in variable first
make_option = () Option<i32> {
    result = Option.Some(42)
    result  // Returns Some(42) correctly
}
```

### Nested Generic Instantiation
Complex nested generics (e.g., `Result<Result<Result<T,E>,E>,E>`) work but require careful handling.

## Best Practices

1. **Always use type annotations** for generic variables to ensure correct type inference
2. **Store generic returns in variables** when returning from functions (workaround for known issue)
3. **Use pattern matching** for safe extraction of values from Option and Result types
4. **Provide hash and equality functions** when using HashMap with custom types

## Future Improvements

1. Fix direct return of enum variants from functions
2. Improve type inference for deeply nested generics
3. Add more generic collection types (Vec<T>, Set<T>, etc.)
4. Support generic function definitions with type parameters
5. Implement trait bounds for generic constraints

## Testing
See `tests/test_generics_comprehensive_showcase.zen` for a complete demonstration of all generic features.