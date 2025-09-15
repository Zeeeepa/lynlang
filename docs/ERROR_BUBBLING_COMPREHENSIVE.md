# Comprehensive Error Bubbling Guide for Zenlang

## Core Principles

Zenlang has **NO** `try` keyword, **NO** exceptions, and **NO** `async` keyword. Instead:

1. **Errors are values** - Use `Result<T, E>` and `Option<T>` types
2. **The `?` operator** - Propagates errors up the call stack
3. **Pattern matching** - Handle errors explicitly
4. **Allocators** - Control execution mode (sync/concurrent)

## Error Bubbling Patterns

### 1. Basic Error Propagation with `?`

The `?` suffix operator is the primary tool for error propagation:

```zen
// Automatically propagate errors up the call stack
process: (input: String) Result<Output, Error> = {
    parsed := parse(input)?        // If parse returns Err, function returns that Err
    validated := validate(parsed)? // If validate returns Err, function returns that Err
    transformed := transform(validated)? // If transform returns Err, function returns that Err
    .Ok(transformed)
}
```

### 2. Pattern Matching for Custom Error Handling

Use pattern matching when you need to handle errors explicitly:

```zen
// Handle specific errors differently
handle_file: (path: String) Result<Data, Error> = {
    read_result := fs.read(path)
    
    read_result ?
        | .Ok -> content => parse(content)
        | .Err -> FileNotFound => {
            // Try default file
            default_content := fs.read("default.json")?
            parse(default_content)
        }
        | .Err -> PermissionDenied => {
            .Err(Error.AccessDenied(path))
        }
        | .Err -> other => .Err(other)
}
```

### 3. Adding Context While Bubbling

Enhance errors with context as they propagate:

```zen
// Add context to errors
load_config: (name: String) Result<Config, AppError> = {
    path := find_config_path(name)
        .map_err(e -> AppError.ConfigNotFound{ name, source: e })?
    
    content := fs.read(path)
        .map_err(e -> AppError.ReadError{ path, source: e })?
    
    config := parse_json(content)
        .map_err(e -> AppError.ParseError{ file: path, source: e })?
    
    .Ok(config)
}
```

### 4. Early Returns with Guard Patterns

Use pattern matching for validation and early returns:

```zen
// Early return on validation failure
process_user: (user_id: u64) Result<User, Error> = {
    // Validate input
    user_id > 0 ?
        | false => return .Err(Error.InvalidId(user_id))
        | true => {}
    
    // Fetch user
    user := db.get_user(user_id)?
    
    // Validate user state
    user.active ?
        | false => return .Err(Error.InactiveUser(user_id))
        | true => {}
    
    // Process active user
    .Ok(process_active_user(user)?)
}
```

### 5. Collecting Multiple Errors

Handle multiple operations that might fail:

```zen
// Collect results from multiple operations
validate_all: (items: []Item) Result<[]ValidItem, []Error> = {
    results := []Result<ValidItem, Error>{}
    
    loop item in items {
        results.append(validate_item(item))
    }
    
    errors := []Error{}
    valid := []ValidItem{}
    
    loop result in results {
        result ?
            | .Ok -> v => valid.append(v)
            | .Err -> e => errors.append(e)
    }
    
    errors.is_empty() ?
        | true => .Ok(valid)
        | false => .Err(errors)
}
```

### 6. Resource Management with `defer`

Ensure cleanup even when errors occur:

```zen
// Proper resource cleanup with defer
process_file: (path: String) Result<ProcessedData, Error> = {
    file := fs.open(path)?
    defer fs.close(file)  // Always runs, even if error occurs
    
    lock := acquire_lock()?
    defer release_lock(lock)  // Always runs, even if error occurs
    
    data := read_all(file)?
    processed := process(data)?
    
    .Ok(processed)
}
```

### 7. Fallback Chain Pattern

Try multiple sources in order:

```zen
// Try multiple fallback options
get_config: () Result<Config, Error> = {
    // Try environment variable
    env_config := load_from_env()
    env_config ?
        | .Ok -> config => return .Ok(config)
        | .Err -> _ => {}
    
    // Try config file
    file_config := load_from_file("config.json")
    file_config ?
        | .Ok -> config => return .Ok(config)
        | .Err -> _ => {}
    
    // Try default config
    default_config := load_defaults()
    default_config ?
        | .Ok -> config => .Ok(config)
        | .Err -> e => .Err(Error.NoConfigFound(e))
}
```

### 8. Async-like Operations with Allocators

Since there's no `async` keyword, use allocators for concurrent operations:

```zen
// Colorless function - works sync or concurrent based on allocator
fetch_data: (url: String, alloc: *Allocator) Result<Data, Error> = {
    // Allocate buffer
    buffer := alloc.alloc(4096)
    defer alloc.free(buffer)
    
    // Fetch data - allocator determines if blocking or non-blocking
    response := net.http_get(url, alloc)?
    
    // Parse response
    data := parse_response(response, alloc)?
    
    .Ok(data)
}

// Usage - same function, different execution modes
main: () void = {
    // Synchronous execution
    sync_alloc := SyncAllocator{}
    sync_result := fetch_data("https://api.example.com", &sync_alloc)
    
    // Concurrent execution - NO async keyword needed!
    concurrent_alloc := ConcurrentAllocator{ runtime: Runtime.init() }
    concurrent_result := fetch_data("https://api.example.com", &concurrent_alloc)
}
```

### 9. Pipeline Pattern with Error Propagation

Chain operations with consistent error handling:

```zen
// Pipeline of transformations
process_pipeline: (input: String) Result<Output, Error> = {
    input
        |> parse?           // Each step can fail
        |> validate?
        |> normalize?
        |> transform?
        |> optimize?
        |> finalize?
        |> .Ok
}

// Or with intermediate handling
process_with_logging: (input: String) Result<Output, Error> = {
    parsed := parse(input)?
    log.debug("Parsed: $(parsed)")
    
    validated := validate(parsed)?
    log.debug("Validated: $(validated)")
    
    transformed := transform(validated)?
    log.debug("Transformed: $(transformed)")
    
    .Ok(transformed)
}
```

### 10. Transaction Pattern

Ensure all-or-nothing operations:

```zen
// Transaction with rollback on error
update_account: (account_id: u64, amount: i64) Result<Account, Error> = {
    // Start transaction
    tx := db.begin_transaction()?
    defer tx.rollback()  // Auto-rollback if not committed
    
    // Lock account
    account := tx.lock_account(account_id)?
    
    // Validate balance
    new_balance := account.balance + amount
    new_balance >= 0 ?
        | false => return .Err(Error.InsufficientFunds)
        | true => {}
    
    // Update balance
    account.balance = new_balance
    tx.update_account(account)?
    
    // Log transaction
    tx.log_transaction(account_id, amount)?
    
    // Commit on success
    tx.commit()?
    .Ok(account)
}
```

## Best Practices

1. **Use `?` for simple propagation** - Don't wrap in unnecessary pattern matching
2. **Add context when bubbling** - Use `map_err` to add relevant information
3. **Handle at the right level** - Bubble up until you can handle meaningfully
4. **Use `defer` for cleanup** - Ensure resources are freed even on error
5. **Be explicit about errors** - Define clear error types with context
6. **Avoid error swallowing** - Always handle or propagate, never ignore
7. **Test error paths** - Write tests for error conditions
8. **Document error cases** - Specify what errors functions can return

## Common Anti-patterns to Avoid

```zen
// ❌ BAD: Swallowing errors
bad_example: () void = {
    result := might_fail()
    // Error is ignored!
}

// ✅ GOOD: Handle or propagate
good_example: () Result<void, Error> = {
    might_fail()?
    .Ok(())
}

// ❌ BAD: Losing error context
bad_context: () Result<Data, Error> = {
    parse(read_file(path)?)?  // Which operation failed?
}

// ✅ GOOD: Add context
good_context: () Result<Data, Error> = {
    content := read_file(path)
        .map_err(e -> Error.ReadFailed(path, e))?
    parse(content)
        .map_err(e -> Error.ParseFailed(path, e))?
}

// ❌ BAD: Trying to use 'try' keyword (doesn't exist!)
// try {
//     dangerous_operation()
// } catch (e) {
//     handle_error(e)
// }

// ✅ GOOD: Use pattern matching
good_pattern: () void = {
    result := dangerous_operation()
    result ?
        | .Ok -> value => process(value)
        | .Err -> error => handle_error(error)
}
```

## Remember

- **NO `try` keyword** - Use `?` operator instead
- **NO exceptions** - Errors are values
- **NO `async` keyword** - Use allocators for concurrency
- **Explicit is better** - Make error handling visible
- **Type safety** - Compiler ensures errors are handled