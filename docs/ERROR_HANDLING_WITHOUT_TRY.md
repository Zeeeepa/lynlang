# Error Handling in Zen - No Try, Just Values

Zen does not have `try` keywords or exceptions. Instead, errors are regular values that are handled explicitly through pattern matching and the `?` operator for convenient error propagation.

## Core Concepts

### Result Type
```zen
Result<T, E> = 
    | Ok(value: T)
    | Err(error: E)
```

### Error Bubbling with `?`
The `?` operator is syntactic sugar for early return on error:

```zen
// With ? operator
read_config = () Result<Config, Error> {
    content := read_file("config.json")?  // Returns early if Err
    data := parse_json(content)?          // Returns early if Err
    config := validate_config(data)?      // Returns early if Err
    return .Ok(config)
}

// Equivalent without ?
read_config = () Result<Config, Error> {
    content := read_file("config.json")
    content ? 
        | .Ok -> value => {
            data := parse_json(value)
            data ?
                | .Ok -> parsed => {
                    config := validate_config(parsed)
                    return config
                }
                | .Err -> e => return .Err(e)
        }
        | .Err -> e => return .Err(e)
}
```

## Error Handling Patterns

### 1. Direct Pattern Matching
```zen
process_user = (id: u64) Result<User, Error> {
    result := fetch_user(id)
    
    result ?
        | .Ok -> user => {
            // Process the user
            return .Ok(user)
        }
        | .Err -> error => {
            // Handle specific errors
            error.code ?
                | 404 => return .Err(Error.not_found("User not found"))
                | 500 => return .Err(Error.internal("Database error"))
                | _ => return .Err(error)
        }
}
```

### 2. Chaining with and_then
```zen
process_data = (input: String) Result<Output, Error> {
    parse_input(input)
        .and_then(validate)
        .and_then(transform)
        .map_err(|e| Error.wrap(e, "Failed to process data"))
}
```

### 3. Default Values with unwrap_or
```zen
get_config_value = (key: String) String {
    config.get(key)
        .unwrap_or("default_value")
}
```

### 4. Early Returns with Multiple Errors
```zen
complex_operation = () Result<Data, Error> {
    // Each ? bubbles up errors automatically
    user := authenticate()?
    permissions := check_permissions(user)?
    data := fetch_data(user.id)?
    processed := process(data)?
    
    return .Ok(processed)
}
```

### 5. Collecting Results
```zen
process_batch = (items: []Item) Result<[]ProcessedItem, Error> {
    mut results := []
    
    for item in items {
        // Process each item, bubble up first error
        processed := process_item(item)?
        results.push(processed)
    }
    
    return .Ok(results)
}

// Or collect all errors
process_batch_all = (items: []Item) ([]ProcessedItem, []Error) {
    mut successes := []
    mut errors := []
    
    for item in items {
        process_item(item) ?
            | .Ok -> value => successes.push(value)
            | .Err -> error => errors.push(error)
    }
    
    return (successes, errors)
}
```

### 6. Error Context
```zen
// Add context to errors as they bubble up
read_user_file = (user_id: u64) Result<UserData, Error> {
    path := format("/users/{}.json", user_id)
    
    read_file(path)
        .map_err(|e| Error.with_context(e, "Failed to read user file"))
        .and_then(parse_json)
        .map_err(|e| Error.with_context(e, format("Invalid data for user {}", user_id)))
}
```

### 7. Custom Error Types
```zen
// Define domain-specific errors
DbError = 
    | ConnectionFailed(reason: String)
    | QueryError(query: String, error: String)
    | TransactionRollback(cause: String)

// Convert to Result
execute_query = (sql: String) Result<QueryResult, DbError> {
    conn := get_connection() ?
        | null => return .Err(.ConnectionFailed("No connection available"))
        | c => c
    
    conn.execute(sql) ?
        | .Success -> result => .Ok(result)
        | .Error -> msg => .Err(.QueryError(sql, msg))
}
```

### 8. Nested Error Handling
```zen
// Handle errors at different levels
process_request = (req: Request) Response {
    // Top level catches all errors
    result := validate_request(req)
        .and_then(|r| {
            // Nested operation with its own error handling
            fetch_data(r.id) ?
                | .Ok -> data => process_data(data)
                | .Err -> e => {
                    // Log but continue with default
                    log.warn("Failed to fetch: {}", e)
                    .Ok(default_data())
                }
        })
    
    result ?
        | .Ok -> data => Response.success(data)
        | .Err -> error => Response.error(error.code, error.message)
}
```

### 9. Defer Cleanup on Error
```zen
with_transaction = (f: (Transaction) Result<T, Error>) Result<T, Error> {
    tx := db.begin_transaction()?
    defer {
        // Rollback if we're returning an error
        if result.is_err() {
            tx.rollback()
        }
    }
    
    result := f(tx)
    
    result ?
        | .Ok -> value => {
            tx.commit()?
            .Ok(value)
        }
        | .Err -> e => .Err(e)
}
```

## Best Practices

1. **Use `?` for simple propagation** - When you just want to bubble errors up
2. **Pattern match for handling** - When you need to handle specific error cases
3. **Add context as errors bubble** - Use `map_err` to add context
4. **Define domain errors** - Create specific error types for your domain
5. **Handle at the right level** - Don't catch errors too early or too late
6. **Document error cases** - Make it clear what errors a function can return

## Migration from Try-based Languages

If you're coming from languages with try/catch:

```javascript
// JavaScript with try/catch
async function processUser(id) {
    try {
        const user = await fetchUser(id);
        const validated = validateUser(user);
        return await saveUser(validated);
    } catch (error) {
        if (error.code === 404) {
            throw new NotFoundError("User not found");
        }
        throw error;
    }
}
```

Becomes in Zen:

```zen
process_user = (id: u64, alloc: *Allocator) Result<User, Error> {
    user := fetch_user(id, alloc)?
    validated := validate_user(user)?
    saved := save_user(validated, alloc)?
    
    return .Ok(saved)
}

// Or with specific error handling
process_user_handled = (id: u64, alloc: *Allocator) Result<User, Error> {
    fetch_user(id, alloc) ?
        | .Ok -> user => {
            validated := validate_user(user)?
            save_user(validated, alloc)
        }
        | .Err -> e => {
            e.code ?
                | 404 => .Err(Error.not_found("User not found"))
                | _ => .Err(e)
        }
}
```

## No Exceptions, Just Values

Remember: In Zen, errors are just values. There's no hidden control flow, no stack unwinding, and no unexpected jumps. Every error is visible in the type signature and must be handled explicitly or propagated consciously.