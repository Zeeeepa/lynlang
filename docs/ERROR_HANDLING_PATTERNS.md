# Error Handling Patterns in Zenlang

Zenlang uses **errors as values** rather than exceptions or `try/catch` blocks. This provides explicit, composable error handling that's visible in function signatures.

## Core Concepts

### No `try` Keyword
Zenlang does not have a `try` keyword. Instead, errors are regular values that are returned and handled through pattern matching.

### Result Type Pattern
```zen
// Functions return Result types for fallible operations
Result(T, E) = 
    | .Ok(value: T)
    | .Err(error: E)

// Example usage
read_file = (path: String) Result(String, FileError) {
    // Implementation returns either Ok or Err
}
```

## Error Handling Patterns

### Pattern 1: Explicit Pattern Matching
```zen
// Handle each case explicitly
result := read_file("config.json")
content := result ?
    | .Ok -> value => value
    | .Err -> err => {
        io.print("Error reading file: ${err}\n")
        return .Err(err)  // Propagate error
    }
```

### Pattern 2: Error Propagation with `?`
```zen
// The ? operator propagates errors early
process_config = () Result(Config, Error) {
    // If read_file returns Err, this function returns immediately
    content := read_file("config.json")?
    
    // If parse_json returns Err, this function returns immediately  
    config := parse_json(content)?
    
    return .Ok(config)
}
```

### Pattern 3: Default Values on Error
```zen
// Provide a default value if operation fails
config := read_file("config.json") ?
    | .Ok -> content => parse_json(content)
    | .Err -> _ => default_config()
```

### Pattern 4: Transform Errors
```zen
// Map error types
load_user = (id: u64) Result(User, AppError) {
    user := db.query("SELECT * FROM users WHERE id = ?", [id]) ?
        | .Ok -> u => .Ok(u)
        | .Err -> db_err => .Err(AppError.Database(db_err))
    
    return user
}
```

### Pattern 5: Collect Multiple Errors
```zen
// Validate multiple fields and collect all errors
validate_user = (data: UserData) Result(User, []ValidationError) {
    errors := []
    
    name := data.name.is_empty() ?
        | true => errors.push(.EmptyName)
        | false => data.name
    
    email := validate_email(data.email) ?
        | .Err -> e => errors.push(.InvalidEmail(e))
        | .Ok -> v => v
    
    errors.is_empty() ?
        | true => .Ok(User{ name, email })
        | false => .Err(errors)
}
```

## Error Bubbling Strategies

### Strategy 1: Early Return
```zen
process = () Result(Output, Error) {
    // Each ? bubbles errors up immediately
    step1 := do_step1()?
    step2 := do_step2(step1)?
    step3 := do_step3(step2)?
    return .Ok(step3)
}
```

### Strategy 2: Defer Cleanup on Error
```zen
process_with_resources = () Result(Data, Error) {
    resource := acquire_resource()
    defer release_resource(resource)
    
    // If any step fails, defer ensures cleanup
    data := process_data(resource)?
    validated := validate(data)?
    
    return .Ok(validated)
}
```

### Strategy 3: Error Context
```zen
// Add context while bubbling errors
load_config = (path: String) Result(Config, Error) {
    content := read_file(path) ?
        | .Ok -> c => c
        | .Err -> e => return .Err(Error.Context("loading config", e))
    
    config := parse_json(content) ?
        | .Ok -> c => c
        | .Err -> e => return .Err(Error.Context("parsing config", e))
    
    return .Ok(config)
}
```

## Common Error Types

```zen
// Standard library error types
FileError = 
    | .NotFound(path: String)
    | .PermissionDenied(path: String)
    | .IoError(msg: String)

NetworkError =
    | .ConnectionRefused(addr: String)
    | .Timeout(duration: u64)
    | .DnsError(host: String)

ParseError =
    | .InvalidSyntax(line: u32, col: u32)
    | .UnexpectedToken(token: String)
    | .InvalidValue(expected: String, got: String)
```

## Best Practices

1. **Make errors explicit in signatures** - Always include error types in function signatures
2. **Use Result for fallible operations** - Any operation that can fail should return Result
3. **Provide context** - Add context when bubbling errors up the stack
4. **Handle errors at appropriate levels** - Don't bubble errors too far; handle them where you have context
5. **Use defer for cleanup** - Ensure resources are released even on error paths
6. **Document error conditions** - Comment what errors a function can return and why

## Migration from Exception-Based Code

If migrating from exception-based languages:

```zen
// Instead of:
// try {
//     data = readFile(path)
//     json = parseJson(data)
// } catch (e) {
//     handleError(e)
// }

// Use:
result := read_file(path)
    .and_then(|data| parse_json(data))
    
result ?
    | .Ok -> json => process(json)
    | .Err -> e => handle_error(e)
```

## No Async Keyword

Zenlang uses allocator-based concurrency rather than `async/await`. Functions are "colorless" - the same function can run synchronously or asynchronously based on the allocator passed to it. See the [Allocator Concurrency Guide](./zen-allocator-concurrent.md) for details.