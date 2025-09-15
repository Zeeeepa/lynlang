# Zenlang: NO try, NO async - Design Summary

## Core Language Principles

Zenlang completely eliminates two major language constructs found in most modern languages:

1. **NO `try` keyword** - Not present in any form
2. **NO `async` keyword** - Completely absent from the language

## Why NO `try`?

The `try` keyword and exception handling have been completely removed from Zenlang because:

- **Errors are values** - Use `Result<T, E>` and `Option<T>` types
- **Explicit control flow** - The `?` operator shows exactly where errors can propagate
- **No hidden jumps** - No invisible exception propagation through the stack
- **Composable** - Error handling composes naturally with function composition
- **Type-safe** - Compiler ensures all Results are handled

### Error Handling in Zenlang

```zen
// Use ? suffix operator for error propagation (NOT try!)
process: (input: String) Result<Output, Error> = {
    parsed := parse(input)?        // Bubbles parse errors
    validated := validate(parsed)? // Bubbles validation errors
    .Ok(transform(validated))
}

// Pattern match for custom error handling
handle_errors: (data: Data) void = {
    result := process(data)
    result ?
        | .Ok -> value => io.println("Success: $(value)")
        | .Err -> error => io.println("Error: $(error)")
}
```

## Why NO `async`?

The `async`/`await` keywords have been completely eliminated because:

- **No function coloring** - Functions aren't marked as sync or async
- **Allocator-based concurrency** - Pass allocator to control execution mode
- **Same code, different modes** - Debug with sync, deploy with concurrent
- **Progressive adoption** - Change allocator, not code structure
- **Simpler testing** - Tests use sync allocator for determinism

### Concurrency in Zenlang

```zen
// NO 'async fn' prefix! Functions are colorless
read_file: (path: String, alloc: *Allocator) Result<[]u8, Error> = {
    buffer := alloc.alloc(4096)
    defer alloc.free(buffer)
    
    // Same code works sync or concurrent based on allocator
    bytes := alloc.is_concurrent ?
        | true => io.read_concurrent(path, buffer, alloc)?
        | false => io.read_sync(path, buffer)?
    
    .Ok(bytes)
}

// Choose execution mode via allocator
main: () void = {
    // Synchronous execution
    sync_alloc := SyncAllocator{}
    data := read_file("data.txt", &sync_alloc)
    
    // Concurrent execution - same function!
    concurrent_alloc := ConcurrentAllocator{ runtime: Runtime.init() }
    data := read_file("data.txt", &concurrent_alloc)
}
```

## Key Differences from Other Languages

| Feature | Other Languages | Zenlang |
|---------|----------------|---------|
| Error Handling | `try`/`catch`/`throw` | `?` operator + pattern matching |
| Exceptions | Hidden control flow | Explicit `Result<T, E>` values |
| Async Functions | `async fn foo()` | `foo(alloc: *Allocator)` |
| Awaiting | `await foo()` | `foo(alloc)?` |
| Function Coloring | async infects callers | No coloring - allocator parameter |
| Concurrency Mode | Fixed at compile time | Runtime choice via allocator |

## Implementation Status

✅ **Completed:**
- No `try` keyword in lexer/parser
- No `async` keyword in lexer/parser  
- Error handling via `?` operator and Result types
- Allocator-based concurrency model
- Comprehensive documentation
- Example code using these patterns

## Benefits

1. **Simplicity** - Fewer language constructs to learn
2. **Explicitness** - All error paths and execution modes visible
3. **Composability** - Errors and concurrency compose naturally
4. **Type Safety** - Compiler enforces proper error handling
5. **Testing** - Simpler, deterministic tests
6. **Performance** - No exception overhead, controlled allocation

## Migration Guide

### From `try`/`catch`:
```javascript
// Other languages
try {
    const data = readFile(path);
    return process(data);
} catch (e) {
    console.error(e);
}
```

```zen
// Zenlang
data := read_file(path)?
process(data)
```

### From `async`/`await`:
```javascript
// Other languages
async function fetchData() {
    const response = await fetch(url);
    return await response.json();
}
```

```zen
// Zenlang
fetch_data: (url: String, alloc: *Allocator) Result<Data, Error> = {
    response := http.get(url, alloc)?
    response.parse_json(alloc)
}
```

## Conclusion

By eliminating `try` and `async` keywords, Zenlang achieves:
- Simpler language specification
- More explicit and predictable code
- Better composability
- Easier testing and debugging
- No function coloring problem
- Clear execution semantics

These design decisions make Zenlang code more maintainable and easier to reason about while maintaining full expressiveness for error handling and concurrent programming.