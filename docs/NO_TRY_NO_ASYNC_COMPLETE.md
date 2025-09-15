# Zenlang: No `try`, No `async` - Complete Design Philosophy

## Executive Summary

Zenlang completely eliminates the `try` and `async` keywords from the language. These words do not exist as keywords, operators, or any part of the syntax. This document explains the design rationale and implementation approach.

## Part 1: No `try` Keyword - Errors as Values

### What Doesn't Exist
- **NO** `try` keyword or blocks
- **NO** `try`/`catch`/`finally` statements  
- **NO** exceptions or exception handling
- **NO** `throw` keyword or throwing errors
- The word "try" is not special - it's just an ordinary identifier

### What We Have Instead

#### 1. Result and Option Types
```zen
// All fallible operations return Result<T, E>
read_file: (path: String) Result<String, io.Error>
parse_int: (s: String) Result<i32, ParseError>

// Optional values use Option<T>
find_user: (id: u64) Option<User>
```

#### 2. The `?` Suffix Operator for Error Propagation
```zen
// The ? operator propagates errors up the call stack
process_file: (path: String) Result<Data, Error> = {
    content := read_file(path)?      // If error, return early with that error
    parsed := parse_json(content)?   // If error, return early with that error
    validated := validate(parsed)?   // If error, return early with that error
    .Ok(validated)
}
```

#### 3. Pattern Matching for Error Handling
```zen
// Explicit error handling with pattern matching
handle_result: (r: Result<Data, Error>) void = {
    r ?
        | .Ok -> data => process(data)
        | .Err -> error => log_error(error)
}
```

### Why No `try`?

1. **Clarity**: Every potential error point is marked with `?` - no hidden control flow
2. **Composability**: Errors compose naturally through Result types
3. **Type Safety**: Can't forget to handle errors - the type system enforces it
4. **No Hidden Costs**: No exception unwinding, no runtime overhead
5. **Local Reasoning**: Error handling is explicit and local

### Migration Examples

#### Before (Other Languages)
```javascript
// JavaScript with try/catch
try {
    const data = await readFile(path);
    const parsed = JSON.parse(data);
    return process(parsed);
} catch (error) {
    console.error('Error:', error);
    throw new ProcessError(error);
}
```

#### After (Zenlang)
```zen
// Zenlang with error values
read_and_process: (path: String) Result<Processed, Error> = {
    data := read_file(path)?
    parsed := parse_json(data)?
    processed := process(parsed)?
    .Ok(processed)
}
```

## Part 2: No `async` Keyword - Colorless Concurrency

### What Doesn't Exist
- **NO** `async` keyword or function modifier
- **NO** `async fn` declarations
- **NO** `await` keyword or operator
- **NO** async blocks or async traits
- **NO** function coloring (async vs sync divide)
- The word "async" is not special - it's just an ordinary identifier

### What We Have Instead: Allocator-Based Concurrency

#### 1. Allocators Control Execution Mode
```zen
// Allocator trait combines memory + execution context
Allocator: {
    // Memory operations
    alloc: (size: usize) *void
    free: (ptr: *void) void
    
    // Execution mode
    is_concurrent: bool
    
    // Continuation support (for concurrent execution)
    suspend: () ?Continuation
    resume: (cont: Continuation) void
}
```

#### 2. Functions Take Allocator Parameters
```zen
// NO async prefix! Same function works sync or concurrent
read_file: (path: String, alloc: *Allocator) Result<String, Error> = {
    // Implementation adapts based on allocator
    alloc.is_concurrent ?
        | true => // Non-blocking I/O path
        | false => // Blocking I/O path
}

// Caller chooses execution mode
main: () void = {
    // Synchronous execution
    sync_alloc := SyncAllocator{}
    data := read_file("data.txt", &sync_alloc)
    
    // Concurrent execution - same function!
    concurrent_alloc := ConcurrentAllocator{}
    data := read_file("data.txt", &concurrent_alloc)
}
```

### Why No `async`?

1. **No Function Coloring**: Functions aren't divided into async/sync camps
2. **Progressive Enhancement**: Start sync, add concurrency without changing signatures
3. **Explicit Control**: Allocator parameter shows execution mode clearly
4. **Natural Testing**: Tests use sync allocator for determinism
5. **Composability**: Mix sync and concurrent code naturally
6. **Zero Cost**: Compiler can specialize based on allocator type

### Migration Examples

#### Before (Other Languages)
```rust
// Rust with async/await
async fn fetch_data(url: &str) -> Result<String, Error> {
    let response = client.get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

async fn process() -> Result<(), Error> {
    let data = fetch_data("http://api.example.com").await?;
    println!("{}", data);
    Ok(())
}
```

#### After (Zenlang)
```zen
// Zenlang with allocator-based concurrency
fetch_data: (url: String, alloc: *Allocator) Result<String, Error> = {
    response := client.get(url, alloc)?
    body := response.text(alloc)?
    .Ok(body)
}

process: (alloc: *Allocator) Result<void, Error> = {
    data := fetch_data("http://api.example.com", alloc)?
    io.println(data)
    .Ok(())
}

main: () void = {
    // Choose execution mode at runtime
    alloc := @debug ?
        | true => SyncAllocator{}        // Sync for debugging
        | false => ConcurrentAllocator{} // Concurrent for production
    
    process(&alloc)
}
```

## Design Philosophy

### Simplicity Through Elimination

Zenlang achieves simplicity not by adding features, but by removing unnecessary concepts:

1. **Remove Exceptions** → Errors become values
2. **Remove `try`** → Use `?` operator for propagation
3. **Remove `async`** → Use allocator parameters
4. **Remove Function Coloring** → All functions are colorless

### Explicit Over Implicit

Every design choice favors explicitness:

- Error handling sites marked with `?`
- Execution mode visible via allocator parameter
- No hidden control flow or state machines
- No implicit memory allocation

### Composition Over Keywords

Instead of special keywords, we use composable abstractions:

- `Result<T, E>` and `Option<T>` types compose naturally
- Allocators compose to create hybrid execution modes
- Pattern matching unifies all control flow

## Implementation Status

### Completed ✅
- Result and Option types implemented
- `?` operator for error propagation
- Pattern matching with `?` operator
- No `try`/`catch`/`throw` in lexer/parser
- No `async`/`await` in lexer/parser
- Allocator trait design
- Colorless function examples

### In Progress 🚧
- Runtime allocator implementations
- Continuation support for concurrent allocator
- Compiler optimizations for allocator specialization

### Future Work 📋
- Standard library allocator implementations
- Actor system using allocators
- Channel implementations with allocators
- Performance benchmarks vs async/await languages

## Benefits Summary

### For Error Handling (No `try`)
- ✅ No hidden control flow
- ✅ Errors can't be ignored
- ✅ Composable error handling
- ✅ Zero runtime overhead
- ✅ Better IDE support (types, not exceptions)

### For Concurrency (No `async`)  
- ✅ No function coloring problem
- ✅ Same code for sync/concurrent
- ✅ Progressive adoption path
- ✅ Simpler mental model
- ✅ Natural testing story
- ✅ Explicit execution control

## Conclusion

By eliminating `try` and `async` keywords entirely, Zenlang achieves:

1. **Radical Simplicity**: Fewer concepts to learn
2. **Better Composability**: Everything composes naturally
3. **Explicit Control**: No hidden behavior
4. **Performance**: Zero-cost abstractions
5. **Correctness**: Type system enforces proper error handling

The absence of these keywords is not a limitation but a strength - it forces better design patterns and clearer code.

## References

- [Error Handling Patterns](./ERROR_HANDLING_PATTERNS.md)
- [Allocator Concurrency Guide](../agent/zen-allocator-concurrent.md)
- [Language Specification](../LANGUAGE_SPEC.md)
- [Examples](../examples/)