# Error Bubbling Best Practices in Zenlang

## Core Principle: Errors as Values

Zenlang has **NO** `try` keyword, **NO** exceptions, and **NO** `async` keywords. The word `try` does not exist in Zenlang - it's not a keyword, not an operator, not part of any syntax. Similarly, `async` is not a language keyword - there's no `async fn`, no `async` blocks, nothing. All errors are values using `Result<T, E>` and `Option<T>` types, and concurrency is achieved through allocators.

## Error Propagation Patterns

### 1. The `?` Operator - Quick Propagation

The `?` operator is the primary way to bubble errors up:

```zen
// Propagate errors with minimal boilerplate
process_file: (path: String) Result<Data, Error> = {
    content := read_file(path)?      // Bubbles up read errors
    parsed := parse(content)?        // Bubbles up parse errors
    validated := validate(parsed)?   // Bubbles up validation errors
    .Ok(validated)
}
```

### 2. Pattern Matching - Explicit Control

Use pattern matching when you need to transform or handle errors:

```zen
// Add context to errors
process_with_context: (path: String) Result<Data, Error> = {
    read_file(path) ?
        | .Ok -> content => parse(content) ?
            | .Ok -> parsed => .Ok(validate(parsed)?)
            | .Err -> e => .Err(Error.ParseFailed{ source: e, file: path })
        | .Err -> e => .Err(Error.ReadFailed{ source: e, file: path })
}
```

### 3. Error Recovery

Handle specific errors while propagating others:

```zen
// Try primary, fall back to secondary
resilient_load: (primary: String, fallback: String) Result<Config, Error> = {
    load_config(primary) ?
        | .Ok -> config => .Ok(config)
        | .Err -> e => e ?
            | .FileNotFound => load_config(fallback)?  // Try fallback
            | _ => .Err(e)  // Propagate other errors
}
```

### 4. Collect Multiple Errors

When you need to validate multiple things:

```zen
// Collect all validation errors
validate_user: (user: User) Result<User, []Error> = {
    errors := []Error{}
    
    // Check each field
    user.name.is_empty() ?
        | true => errors.push(Error.InvalidName)
        | false => {}
    
    user.email.contains("@") ?
        | false => errors.push(Error.InvalidEmail)
        | true => {}
    
    user.age < 0 || user.age > 150 ?
        | true => errors.push(Error.InvalidAge)
        | false => {}
    
    // Return errors or success
    errors.is_empty() ?
        | true => .Ok(user)
        | false => .Err(errors)
}
```

### 5. Early Returns

Use early returns for cleaner error handling:

```zen
process_data: (input: Input) Result<Output, Error> = {
    // Validate input first
    input.is_valid() ?
        | false => return .Err(Error.InvalidInput)
        | true => {}
    
    // Check permissions
    has_permission(input.user) ?
        | false => return .Err(Error.Unauthorized)
        | true => {}
    
    // Process if all checks pass
    result := transform(input)
    .Ok(result)
}
```

## Error Types Best Practices

### Define Clear Error Types

```zen
// Domain-specific errors
DatabaseError:
    | ConnectionFailed(message: String)
    | QueryFailed(query: String, error: String)
    | TransactionRollback(reason: String)

// Application errors
AppError:
    | Database(err: DatabaseError)
    | Validation(field: String, reason: String)
    | Network(code: u32, message: String)
    | Business(rule: String, violation: String)
```

### Error Conversion

```zen
// Convert between error types
from_db_error: (err: DatabaseError) AppError = {
    .Database(err)
}

// Use ? with conversion
fetch_user: (id: u64) Result<User, AppError> = {
    query := "SELECT * FROM users WHERE id = ?"
    db.execute(query, [id])
        .map_err(from_db_error)?  // Convert and propagate
}
```

## Common Patterns

### Chain Operations

```zen
// Chain multiple fallible operations
pipeline: (input: String) Result<Output, Error> = {
    parse(input)?
        |> validate?
        |> transform?
        |> serialize?
}
```

### Wrap External Errors

```zen
// Wrap third-party errors
call_external_api: (url: String) Result<Response, AppError> = {
    http.get(url) ?
        | .Ok -> response => .Ok(response)
        | .Err -> http_err => .Err(AppError.Network(
            http_err.code,
            "External API failed: " + http_err.message
        ))
}
```

### Default Values on Error

```zen
// Provide defaults for non-critical errors
load_config: () Config = {
    read_config_file() ?
        | .Ok -> config => config
        | .Err -> _ => Config.default()  // Use defaults if config missing
}
```

## Testing Error Paths

```zen
test "error propagation" {
    // Test error cases
    result := process_file("nonexistent.txt")
    result ?
        | .Err -> e => assert(e == Error.FileNotFound)
        | .Ok -> _ => @panic("Should have failed")
}

test "error recovery" {
    // Test fallback behavior
    result := resilient_load("missing.cfg", "default.cfg")
    assert(result.is_ok())
}
```

## Anti-Patterns to Avoid

### ❌ Don't Ignore Errors

```zen
// BAD - ignoring errors
bad_example: () void = {
    read_file("data.txt")  // Error: unused Result
}

// GOOD - handle or propagate
good_example: () Result<void, Error> = {
    data := read_file("data.txt")?
    process(data)
    .Ok(())
}
```

### ❌ Don't Panic on Recoverable Errors

```zen
// BAD - panicking on user input errors
bad_validate: (input: String) Data = {
    input.is_valid() ?
        | false => @panic("Invalid input!")  // Don't do this!
        | true => parse(input)
}

// GOOD - return error
good_validate: (input: String) Result<Data, Error> = {
    input.is_valid() ?
        | false => .Err(Error.InvalidInput)
        | true => .Ok(parse(input))
}
```

### ❌ Don't Lose Error Context

```zen
// BAD - losing error information
bad_wrapper: () Result<Data, String> = {
    fetch_data() ?
        | .Ok -> data => .Ok(data)
        | .Err -> _ => .Err("Something went wrong")  // Lost context!
}

// GOOD - preserve error details
good_wrapper: () Result<Data, Error> = {
    fetch_data() ?
        | .Ok -> data => .Ok(data)
        | .Err -> e => .Err(Error.FetchFailed{ 
            source: e, 
            timestamp: now() 
        })
}
```

## Error Handling with Allocators

Allocators control both memory and execution mode. Error handling remains the same whether using sync or async allocators:

```zen
// Function takes allocator - no 'async' keyword!
fetch_data: (url: String, alloc: *Allocator) Result<Data, Error> = {
    // Connect - errors bubble regardless of allocator type
    conn := http.connect(url, alloc)?
    defer conn.close()
    
    // Send request - same error handling for sync/async
    response := conn.send_request(Request.get("/api"), alloc)?
    
    // Parse response - errors as values, not exceptions
    response.status == 200 ?
        | true => {
            data := json.parse(response.body)?
            .Ok(data)
        }
        | false => .Err(Error.HttpError(response.status))
}

// Usage with different allocators
main: () void = {
    // Sync allocator for debugging
    sync_alloc := SyncAllocator{}
    result := fetch_data("https://api.example.com", &sync_alloc)
    
    // Async allocator for production
    async_alloc := AsyncAllocator{ runtime: Runtime.new() }
    result2 := fetch_data("https://api.example.com", &async_alloc)
    
    // Error handling is identical!
    result ?
        | .Ok -> data => process(data)
        | .Err -> e => log.error("Failed: $(e)")
}
```

### Allocator Error Patterns

```zen
// Allocator operations can fail too
safe_alloc: (size: usize, alloc: *Allocator) Result<*void, AllocError> = {
    ptr := alloc.alloc(size)
    ptr == null ?
        | true => .Err(AllocError.OutOfMemory(size))
        | false => .Ok(ptr)
}

// Chain allocator operations with error handling
process_large_data: (data: []u8, alloc: *Allocator) Result<Output, Error> = {
    // Allocate working buffer - bubble allocation errors
    buffer := safe_alloc(data.len * 2, alloc)?
    defer alloc.free(buffer)
    
    // Process in chunks - errors bubble naturally
    for chunk in data.chunks(1024) {
        process_chunk(chunk, buffer, alloc)?
    }
    
    .Ok(finalize(buffer))
}
```

## Summary

Remember: Zenlang has **NO** `try` keyword and **NO** `async` keywords. The word `try` doesn't exist in the language. The word `async` doesn't exist as a keyword - no `async fn`, no `async` blocks. Instead:

1. **Errors are values** - Use `Result<T, E>` and `Option<T>`
2. **The `?` operator** - For automatic error propagation (NOT `try`!)
3. **Pattern matching** - For explicit error handling (NOT `catch`!)
4. **Allocators** - For colorless concurrency (NOT `async`!)
5. **Early returns** - For clean error flow
6. **Error types** - For domain modeling

This approach eliminates:
- Hidden control flow (no exceptions)
- Function coloring (no async/await)
- Complex error handling (no try/catch/finally)
- Implicit behavior (everything is explicit)

Error handling in Zenlang is visible, testable, and composable - whether running synchronously or concurrently!