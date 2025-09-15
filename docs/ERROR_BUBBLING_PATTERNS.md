# Error Bubbling Patterns in Zenlang

Zenlang embraces **errors as values** with explicit handling through pattern matching. There is no `try` keyword - instead, we use the `?` operator and pattern matching for error propagation.

## Quick Reference

```zen
// Basic error bubbling with ?
result := fallible_operation()?  // Returns early if error

// Pattern matching for custom handling
value := operation() ?
    | .Ok -> v => v
    | .Err -> e => return .Err(wrap_error(e))
```

## Core Error Bubbling Patterns

### 1. The `?` Operator - Automatic Bubbling

The `?` operator is the primary way to bubble errors up the call stack:

```zen
// Automatically returns .Err if any operation fails
process_file = (path: String) Result(Data, Error) {
    content := read_file(path)?      // Bubbles FileError
    parsed := parse_content(content)? // Bubbles ParseError
    validated := validate(parsed)?    // Bubbles ValidationError
    return .Ok(validated)
}
```

### 2. Pattern Match with Early Return

For cases where you need to transform or log errors:

```zen
fetch_user_data = (id: u64) Result(UserData, AppError) {
    // Fetch from database
    user := db.get_user(id) ?
        | .Ok -> u => u
        | .Err -> e => {
            log.error("Failed to fetch user ${id}: ${e}")
            return .Err(AppError.Database(e))
        }
    
    // Fetch related data
    profile := db.get_profile(user.profile_id) ?
        | .Ok -> p => p
        | .Err -> _ => return .Err(AppError.MissingProfile(id))
    
    return .Ok(UserData{ user, profile })
}
```

### 3. Chain Operations with And-Then

For sequential operations that depend on previous results:

```zen
// Extension methods on Result
impl Result {
    and_then = (self, f: (T) Result(U, E)) Result(U, E) {
        self ?
            | .Ok -> v => f(v)
            | .Err -> e => .Err(e)
    }
    
    map_err = (self, f: (E) F) Result(T, F) {
        self ?
            | .Ok -> v => .Ok(v)
            | .Err -> e => .Err(f(e))
    }
}

// Usage
load_and_process = (path: String) Result(Output, Error) {
    read_file(path)
        .and_then(|content| parse_json(content))
        .and_then(|json| validate(json))
        .map_err(|e| Error.LoadFailed(path, e))
}
```

### 4. Collect Multiple Errors

When you need to validate multiple things and report all errors:

```zen
validate_request = (req: Request) Result(ValidRequest, []Error) {
    errors := []Error{}
    
    // Check each field, collecting errors
    auth := validate_auth(req.token) ?
        | .Ok -> a => a
        | .Err -> e => {
            errors.push(Error.Auth(e))
            null
        }
    
    params := validate_params(req.params) ?
        | .Ok -> p => p
        | .Err -> e => {
            errors.push(Error.Params(e))
            null
        }
    
    body := validate_body(req.body) ?
        | .Ok -> b => b
        | .Err -> e => {
            errors.push(Error.Body(e))
            null
        }
    
    // Return all errors or success
    errors.is_empty() ?
        | true => .Ok(ValidRequest{ auth, params, body })
        | false => .Err(errors)
}
```

### 5. Context-Aware Error Bubbling

Add context as errors bubble up:

```zen
Error = 
    | .Io(IoError)
    | .Parse(ParseError)
    | .Validation(String)
    | .WithContext(context: String, inner: Error)

// Wrap errors with context
process_config = (env: String) Result(Config, Error) {
    path := get_config_path(env)
        .map_err(|e| Error.WithContext("resolving config path", e))?
    
    content := read_file(path)
        .map_err(|e| Error.WithContext("reading config file: ${path}", e))?
    
    config := parse_yaml(content)
        .map_err(|e| Error.WithContext("parsing config for ${env}", e))?
    
    validate_config(config)
        .map_err(|e| Error.WithContext("validating ${env} config", e))?
    
    return .Ok(config)
}
```

### 6. Try-With-Resources Pattern

Ensure cleanup even when errors occur:

```zen
with_transaction = (f: (*Transaction) Result(T, E)) Result(T, E) {
    tx := db.begin_transaction()
    defer {
        // Rollback if not committed
        tx.is_active() ? 
            | true => tx.rollback()
            | false => {}
    }
    
    result := f(tx)?
    tx.commit()?
    
    return .Ok(result)
}

// Usage
transfer_funds = (from: u64, to: u64, amount: u64) Result(Receipt, Error) {
    with_transaction(|tx| {
        sender := tx.get_account(from)?
        receiver := tx.get_account(to)?
        
        sender.balance < amount ?
            | true => return .Err(Error.InsufficientFunds)
            | false => {}
        
        tx.update_balance(from, sender.balance - amount)?
        tx.update_balance(to, receiver.balance + amount)?
        
        receipt := tx.create_receipt(from, to, amount)?
        return .Ok(receipt)
    })
}
```

### 7. Pipeline Error Handling

For data processing pipelines:

```zen
Pipeline = {
    steps: []((Data) Result(Data, Error))
    
    add_step: (self, step: (Data) Result(Data, Error)) Pipeline {
        self.steps.push(step)
        return self
    }
    
    run: (self, input: Data) Result(Data, Error) {
        data := input
        
        self.steps.each() |step, i| {
            data = step(data) ?
                | .Ok -> d => d
                | .Err -> e => return .Err(Error.PipelineStep(i, e))
        }
        
        return .Ok(data)
    }
}

// Usage
etl_pipeline := Pipeline{}
    .add_step(|d| extract_data(d))
    .add_step(|d| transform_data(d))
    .add_step(|d| validate_data(d))
    .add_step(|d| load_data(d))

result := etl_pipeline.run(raw_data)
```

## Error Bubbling with Allocators

When using allocator-based concurrency, errors bubble the same way:

```zen
fetch_data = (url: String, alloc: *Allocator) Result(Data, Error) {
    // The allocator determines sync/async, errors bubble the same way
    response := http.get(url, alloc)?
    
    response.status != 200 ?
        | true => return .Err(Error.HttpStatus(response.status))
        | false => {}
    
    data := parse_response(response.body)?
    return .Ok(data)
}
```

## Best Practices

1. **Use `?` for simple bubbling** - Default to `?` when you just need to propagate
2. **Add context at boundaries** - Add error context at module/service boundaries
3. **Handle errors where you have context** - Don't bubble too far from where you can handle
4. **Document error types** - Be explicit about what errors functions can return
5. **Use defer for cleanup** - Ensure resources are released on all paths
6. **Avoid silent failures** - Always explicitly handle or propagate errors

## Anti-Patterns to Avoid

```zen
// DON'T: Ignore errors silently
result := dangerous_operation()
// Missing error handling!

// DO: Handle or propagate explicitly
result := dangerous_operation()?

// DON'T: Lose error context
parse_file(path) ?
    | .Err -> _ => .Err(Error.Generic("failed"))

// DO: Preserve error information
parse_file(path) ?
    | .Err -> e => .Err(Error.ParseFailed(path, e))

// DON'T: Panic unnecessarily
config := load_config() ?
    | .Err -> e => @panic("Config error: ${e}")

// DO: Return errors for the caller to handle
config := load_config()?
```

## Summary

Zenlang's error handling is:
- **Explicit** - Errors are visible in types
- **Composable** - Errors can be transformed and combined
- **Efficient** - No runtime overhead from exceptions
- **Colorless** - Same error handling for sync and async (via allocators)

Remember: There is no `try` keyword and no `async` keyword in Zenlang. Use `?` for error bubbling and allocators for concurrency.