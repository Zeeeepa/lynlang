# Complete Guide to Error Bubbling in Zenlang

## Core Principle: Errors Are Values

Zenlang has **NO** `try` keyword, **NO** exceptions, and **NO** hidden control flow. Errors are ordinary values that you handle using the same pattern matching and operators as any other data.

## The Three Error Handling Tools

### 1. The `?` Suffix Operator - Automatic Propagation

The `?` operator is your primary tool for error bubbling:

```zen
// If any operation returns Err, the function returns that Err immediately
// If it returns Ok(value), execution continues with that value
process_file: (path: String) Result<Data, Error> = {
    content := read_file(path)?      // Bubbles file errors
    parsed := parse_json(content)?   // Bubbles parse errors
    validated := validate(parsed)?   // Bubbles validation errors
    .Ok(validated)
}
```

### 2. Pattern Matching with `?` - Explicit Handling

Use pattern matching for custom error handling:

```zen
divide_with_fallback: (a: i32, b: i32) i32 = {
    result := safe_divide(a, b)
    
    result ?
        | .Ok -> value => value
        | .Err -> _ => 0  // Return 0 on division error
}
```

### 3. Result Combinators - Functional Style

Chain operations with combinators:

```zen
process_functional: (input: String) Result<Output, Error> = {
    parse(input)
        .and_then(validate)
        .and_then(transform)
        .map_err((e) => Error.WithContext(e, input))
}
```

## Comprehensive Error Bubbling Patterns

### Pattern 1: Simple Linear Bubbling

When you just need errors to flow up:

```zen
fetch_user_profile: (user_id: u64) Result<Profile, Error> = {
    user := db.get_user(user_id)?
    settings := db.get_settings(user.id)?
    preferences := db.get_preferences(user.id)?
    
    .Ok(Profile{
        user: user,
        settings: settings,
        preferences: preferences
    })
}
```

### Pattern 2: Adding Context While Bubbling

Enrich errors with context as they bubble:

```zen
load_configuration: (name: String) Result<Config, AppError> = {
    // Find config file
    path_result := find_config_path(name)
    path := path_result ?
        | .Ok -> p => p
        | .Err -> e => return .Err(AppError.ConfigNotFound{ name: name, cause: e })
    
    // Read file
    content_result := read_file(path)
    content := content_result ?
        | .Ok -> c => c
        | .Err -> e => return .Err(AppError.FileReadError{ path: path, cause: e })
    
    // Parse configuration
    config_result := parse_config(content)
    config := config_result ?
        | .Ok -> cfg => cfg
        | .Err -> e => return .Err(AppError.ParseError{ file: path, cause: e })
    
    .Ok(config)
}
```

### Pattern 3: Selective Error Recovery

Handle some errors, bubble others:

```zen
get_data_with_cache: (key: String) Result<Data, Error> = {
    // Try cache first
    cache_result := cache.get(key)
    
    cache_result ?
        | .Ok -> data => .Ok(data)
        | .Err -> CacheError.NotFound => {
            // Cache miss is OK, fetch from source
            data := fetch_from_source(key)?  // Other errors bubble
            cache.set(key, data)?            // Cache errors bubble
            .Ok(data)
        }
        | .Err -> other => .Err(other)  // Bubble non-NotFound errors
}
```

### Pattern 4: Transaction Pattern with Rollback

Handle complex operations that need cleanup:

```zen
transfer_funds: (from: Account, to: Account, amount: u64) Result<Receipt, TransferError> = {
    // Begin transaction
    tx := db.begin_transaction()?
    defer tx.rollback_if_not_committed()
    
    // Check balance
    balance := tx.get_balance(from)?
    balance < amount ?
        | true => return .Err(TransferError.InsufficientFunds)
        | false => {}
    
    // Perform transfer
    tx.debit(from, amount)?
    tx.credit(to, amount)?
    
    // Record in ledger
    entry := LedgerEntry{
        from: from,
        to: to,
        amount: amount,
        timestamp: time.now()
    }
    tx.record_ledger(entry)?
    
    // Commit and generate receipt
    tx.commit()?
    receipt := generate_receipt(from, to, amount)?
    .Ok(receipt)
}
```

### Pattern 5: Parallel Error Collection

Handle multiple independent operations:

```zen
validate_order: (order: Order) Result<ValidatedOrder, ValidationError> = {
    // Run validations in parallel
    results := [
        validate_customer(order.customer),
        validate_items(order.items),
        validate_payment(order.payment),
        validate_shipping(order.shipping)
    ]
    
    // Collect all errors
    errors := []
    results.each((r) => {
        r ?
            | .Err -> e => errors.push(e)
            | .Ok -> _ => {}
    })
    
    // Return errors if any
    errors.is_empty() ?
        | false => .Err(ValidationError.Multiple(errors))
        | true => .Ok(ValidatedOrder.from(order))
}
```

### Pattern 6: Pipeline with Error Transformation

Transform errors through a processing pipeline:

```zen
process_request: (raw: String) Result<Response, RequestError> = {
    // Parse request
    request := parse_request(raw)
        .map_err((e) => RequestError.ParseFailed(e))?
    
    // Authenticate
    auth := authenticate(request.token)
        .map_err((e) => RequestError.AuthFailed(e))?
    
    // Authorize
    authorized := authorize(auth, request.resource)
        .map_err((e) => RequestError.NotAuthorized(e))?
    
    // Process
    result := execute_request(request, auth)
        .map_err((e) => RequestError.ExecutionFailed(e))?
    
    // Format response
    response := format_response(result)
        .map_err((e) => RequestError.FormatFailed(e))?
    
    .Ok(response)
}
```

### Pattern 7: Early Return with Cleanup

Ensure cleanup happens regardless of error path:

```zen
process_temp_file: (data: Data) Result<Output, Error> = {
    // Create temp file
    temp_file := create_temp_file()?
    defer delete_file(temp_file.path)  // Always cleanup
    
    // Write data
    write_file(temp_file, data)?
    
    // Process
    processed := run_processor(temp_file.path)?
    
    // Validate output
    validate_output(processed)?
    
    .Ok(processed)
}
```

### Pattern 8: Nested Result Handling

Handle Results within Results:

```zen
fetch_optional_metadata: (id: u64) Result<Option<Metadata>, Error> = {
    // Check if metadata exists
    exists := db.metadata_exists(id)?
    
    exists ?
        | false => .Ok(.None)
        | true => {
            // Fetch metadata
            data := db.get_metadata(id)?
            parsed := parse_metadata(data)?
            .Ok(.Some(parsed))
        }
}

// Using the function
use_metadata: (id: u64) Result<String, Error> = {
    meta_result := fetch_optional_metadata(id)?
    
    meta_result ?
        | .Some -> m => .Ok(m.description)
        | .None => .Ok("No metadata available")
}
```

### Pattern 9: Custom Error Types with Context

Define rich error types:

```zen
HttpError: 
    | NetworkError(cause: io.Error)
    | Timeout(duration: Duration)
    | InvalidResponse(status: u16, body: String)
    | ParseError(cause: json.Error)

http_get: (url: String) Result<Response, HttpError> = {
    // Connect with timeout
    conn := net.connect(url)
        .map_err((e) => HttpError.NetworkError(e))?
    
    // Set timeout
    conn.set_timeout(Duration.seconds(30))
        .map_err((e) => HttpError.NetworkError(e))?
    
    // Send request
    conn.send_request()
        .map_err((e) => HttpError.NetworkError(e))?
    
    // Read response
    raw := conn.read_response()
        .map_err((e) => HttpError.Timeout(Duration.seconds(30)))?
    
    // Parse response
    response := parse_response(raw)
        .map_err((e) => HttpError.ParseError(e))?
    
    // Check status
    response.status >= 400 ?
        | true => .Err(HttpError.InvalidResponse(response.status, response.body))
        | false => .Ok(response)
}
```

### Pattern 10: Async-like Operations with Allocators

Since Zen has no `async` keyword, use allocators for concurrent operations:

```zen
fetch_data_concurrent: (urls: []String, alloc: *Allocator) Result<[]Data, Error> = {
    // Launch concurrent fetches
    futures := urls.map((url) => {
        spawn_task(() => fetch_url(url, alloc), alloc)
    })
    
    // Collect results
    results := []
    futures.each((future) => {
        result := future.await(alloc)?
        results.push(result)
    })
    
    .Ok(results)
}
```

## Best Practices

### 1. Use `?` for Simple Propagation
```zen
// Good - clear and concise
data := read_file(path)?
parsed := parse(data)?
```

### 2. Add Context When It Helps
```zen
// Good - meaningful error context
config := load_config(name)
    .map_err((e) => Error.ConfigLoadFailed{ name: name, cause: e })?
```

### 3. Handle Expected Errors Explicitly
```zen
// Good - handle expected cases
cache_result ?
    | .Err -> CacheError.NotFound => fetch_from_source()
    | .Err -> other => .Err(other)
    | .Ok -> data => .Ok(data)
```

### 4. Use defer for Cleanup
```zen
// Good - guaranteed cleanup
resource := acquire_resource()?
defer resource.release()
```

### 5. Create Domain-Specific Error Types
```zen
// Good - meaningful error types
DatabaseError:
    | ConnectionFailed(host: String, port: u16)
    | QueryFailed(sql: String, error: String)
    | TransactionAborted(reason: String)
```

## Common Anti-Patterns to Avoid

### 1. Swallowing Errors
```zen
// BAD - loses error information
result ?
    | .Err -> _ => .Ok(default_value)  // Don't ignore errors!
    | .Ok -> v => .Ok(v)

// GOOD - log or handle appropriately
result ?
    | .Err -> e => {
        log.warn("Using default due to: $(e)")
        .Ok(default_value)
    }
    | .Ok -> v => .Ok(v)
```

### 2. Unnecessary Wrapping
```zen
// BAD - redundant wrapping
.Ok(.Ok(value))

// GOOD - single level
.Ok(value)
```

### 3. Mixing Error Types Without Context
```zen
// BAD - loses error source
parse_error := parse(data)?
network_error := send(data)?  // Different error type!

// GOOD - unified error type
parse(data).map_err((e) => AppError.Parse(e))?
send(data).map_err((e) => AppError.Network(e))?
```

## Remember

- **NO `try` keyword exists in Zenlang** - Use `?` operator
- **NO exceptions or throw/catch** - Errors are values
- **NO async/await keywords** - Use allocators for concurrency
- **Explicit is better than implicit** - Make error handling visible
- **Type safety** - Compiler ensures Result types are handled

This approach makes error handling:
- **Predictable** - No hidden control flow
- **Composable** - Errors flow naturally through functions
- **Type-safe** - Can't ignore errors accidentally
- **Visible** - See exactly where errors can occur