# Zenlang Error Bubbling Guide

## Core Principle: Errors Are Values

Zenlang has **NO** exceptions, **NO** `try` keyword, and **NO** throw/catch mechanisms. The word `try` does not exist in Zenlang - it is not a keyword, not an operator, and not part of any syntax. All errors are regular values that you handle with:

1. **The `?` operator** - Automatic error propagation
2. **Pattern matching** - Explicit error handling  
3. **Result/Option types** - Type-safe error representation

## The `?` Operator - Your Primary Tool

The `?` suffix operator automatically propagates errors up the call stack:

```zen
// If any operation returns Err, the function returns that Err immediately
// If it returns Ok(value), execution continues with that value
process_file: (path: String) Result<Data, Error> = {
    content := read_file(path)?      // Bubbles read errors
    parsed := parse_json(content)?   // Bubbles parse errors
    validated := validate(parsed)?   // Bubbles validation errors
    .Ok(validated)
}
```

## Error Bubbling Patterns

### 1. Simple Error Propagation

Use `?` as a suffix operator to automatically propagate errors:

```zen
read_and_parse: (path: String) Result<Config, Error> = {
    // Each ? automatically returns on error
    content := fs.readFile(path)?     // Returns if file read fails
    parsed := json.parse(content)?    // Returns if parse fails
    config := validate(parsed)?       // Returns if validation fails
    .Ok(config)
}
```

### 2. Chaining Operations

Chain multiple fallible operations cleanly:

```zen
process_data: (input: String) Result<Output, Error> = {
    parse(input)?
        |> validate?
        |> transform?
        |> optimize?
        |> .Ok
}
```

### 3. Adding Error Context

Wrap errors with additional context as they bubble:

```zen
load_user_profile: (id: u64) Result<Profile, AppError> = {
    // Add context to each potential failure point
    user := db.get_user(id)
        .map_err((e) => AppError.Database("Failed to load user", e))?
    
    settings := db.get_settings(user.settings_id)
        .map_err((e) => AppError.Database("Failed to load settings for user $(id)", e))?
    
    avatar := storage.get_avatar(user.avatar_id)
        .map_err((e) => AppError.Storage("Failed to load avatar", e))?
    
    .Ok(Profile{ user, settings, avatar })
}
```

### 4. Custom Error Handling

Use pattern matching for custom error logic:

```zen
safe_divide: (a: f64, b: f64) Result<f64, MathError> = {
    b == 0.0 ?
        | true => .Err(MathError.DivisionByZero)
        | false => .Ok(a / b)
}

calculate: (x: f64, y: f64) Result<f64, MathError> = {
    // Try division, with custom handling
    result := safe_divide(x, y)
    
    result ?
        | .Ok -> value => .Ok(value * 2.0)
        | .Err -> MathError.DivisionByZero => {
            // Handle specific error with fallback
            .Ok(f64.INFINITY)
        }
        | .Err -> other => .Err(other)
}
```

### 5. Early Return Pattern

Combine `?` with early returns for validation:

```zen
validate_request: (req: Request) Result<ValidRequest, ValidationError> = {
    // Check authentication
    auth_header := req.headers.get("Authorization")?
        | .None => return .Err(ValidationError.MissingAuth)
        | .Some -> header => header
    
    // Validate token
    token := parse_token(auth_header)?
    token.is_expired() ?
        | true => return .Err(ValidationError.ExpiredToken)
        | false => {}
    
    // Check request size
    req.body.len() > MAX_SIZE ?
        | true => return .Err(ValidationError.PayloadTooLarge)
        | false => {}
    
    .Ok(ValidRequest{ 
        token: token,
        body: req.body,
        validated_at: time.now()
    })
}
```

### 6. Collecting Errors from Multiple Operations

```zen
validate_batch: (items: []Item) Result<[]ValidItem, BatchError> = {
    results := items.map(validate_item)
    
    // Separate successes and failures
    successes := []ValidItem{}
    failures := []Error{}
    
    results.each((result) => {
        result ?
            | .Ok -> item => successes.push(item)
            | .Err -> error => failures.push(error)
    })
    
    // Decide based on failure policy
    failures.len() > 0 ?
        | true => .Err(BatchError{ 
            failed: failures.len(),
            total: items.len(),
            errors: failures
        })
        | false => .Ok(successes)
}
```

### 7. Transaction Pattern

Ensure cleanup even when errors occur:

```zen
execute_transaction: <T>(operations: (Transaction) Result<T, Error>) Result<T, Error> = {
    tx := db.begin_transaction()?
    defer tx.rollback()  // Always rollback unless we explicitly commit
    
    result := operations(tx)?
    tx.commit()?  // Only reached if operations succeed
    
    .Ok(result)
}

// Usage
transfer_money: (from: AccountId, to: AccountId, amount: Money) Result<TransferReceipt, Error> = {
    execute_transaction((tx) => {
        // All operations use the transaction
        from_account := tx.get_account(from)?
        to_account := tx.get_account(to)?
        
        // Validate
        from_account.balance < amount ?
            | true => return .Err(Error.InsufficientFunds)
            | false => {}
        
        // Execute transfer
        tx.debit(from, amount)?
        tx.credit(to, amount)?
        
        // Create receipt
        receipt := tx.create_receipt(from, to, amount)?
        .Ok(receipt)
    })
}
```

### 8. Retry Pattern with Backoff

```zen
retry_with_backoff: <T>(
    operation: () Result<T, Error>,
    max_attempts: u32 = 3,
    initial_delay_ms: u32 = 100
) Result<T, Error> = {
    attempt := 0
    delay := initial_delay_ms
    
    loop {
        result := operation()
        
        result ?
            | .Ok -> value => return .Ok(value)
            | .Err -> error => {
                attempt += 1
                
                // Check if we should retry
                attempt >= max_attempts ?
                    | true => return .Err(error)
                    | false => {}
                
                error.is_retryable() ?
                    | false => return .Err(error)
                    | true => {
                        time.sleep(delay)
                        delay *= 2  // Exponential backoff
                    }
            }
    }
}

// Usage
fetch_data: (url: String) Result<Data, Error> = {
    retry_with_backoff(() => http.get(url))
}
```

### 9. Circuit Breaker Pattern

```zen
CircuitBreaker<T> = {
    failure_count: u32,
    threshold: u32,
    timeout_ms: u32,
    state: CircuitState,
    last_failure: ?time.Instant,
}

CircuitState = 
    | Closed
    | Open(until: time.Instant)
    | HalfOpen

circuit_breaker_execute: <T>(
    breaker: *CircuitBreaker,
    operation: () Result<T, Error>
) Result<T, CircuitBreakerError> = {
    // Check circuit state
    breaker.state ?
        | .Open -> until => {
            time.now() < until ?
                | true => return .Err(CircuitBreakerError.CircuitOpen)
                | false => breaker.state = .HalfOpen
        }
        | _ => {}
    
    // Execute operation
    result := operation()
    
    result ?
        | .Ok -> value => {
            // Reset on success
            breaker.failure_count = 0
            breaker.state = .Closed
            .Ok(value)
        }
        | .Err -> error => {
            breaker.failure_count += 1
            breaker.last_failure = .Some(time.now())
            
            // Open circuit if threshold exceeded
            breaker.failure_count >= breaker.threshold ?
                | true => {
                    breaker.state = .Open(time.now() + breaker.timeout_ms)
                }
                | false => {}
            
            .Err(CircuitBreakerError.OperationFailed(error))
        }
}
```

### 10. Pipeline Pattern with Error Handling

```zen
// Define a pipeline that can fail at any step
Pipeline<T> = {
    steps: []((T) Result<T, Error>),
}

Pipeline<T>.execute: (self: Pipeline<T>, input: T) Result<T, PipelineError> = {
    current := input
    step_index := 0
    
    self.steps.each((step) => {
        result := step(current)
        
        result ?
            | .Ok -> value => current = value
            | .Err -> error => {
                return .Err(PipelineError{
                    step: step_index,
                    error: error,
                    partial_result: current
                })
            }
        
        step_index += 1
    })
    
    .Ok(current)
}

// Usage
data_pipeline := Pipeline{
    steps: [
        validate_format,
        normalize_data,
        enrich_with_metadata,
        compress,
        encrypt
    ]
}

processed := data_pipeline.execute(raw_data)?
```

## Advanced Error Bubbling Techniques

### Async Error Handling with Allocators

```zen
// Errors bubble through async operations via allocators
fetch_all: (urls: []String, alloc: *Allocator) Result<[]Response, Error> = {
    // Start all fetches concurrently
    tasks := urls.map((url) => {
        spawn(alloc, () => http.get(url, alloc))
    })
    
    // Collect results, bubble first error
    responses := []Response{}
    tasks.each((task) => {
        result := task.await(alloc)?  // ? bubbles any fetch error
        responses.push(result)
    })
    
    .Ok(responses)
}
```

### Error Aggregation

```zen
// Collect all errors instead of failing on first
ValidationResult = 
    | Valid(data: ValidatedData)
    | Invalid(errors: []ValidationError)

validate_form: (form: FormData) ValidationResult = {
    errors := []ValidationError{}
    
    // Check each field
    form.email.is_valid_email() ?
        | false => errors.push(ValidationError.InvalidEmail)
        | true => {}
    
    form.password.len() < 8 ?
        | true => errors.push(ValidationError.WeakPassword)
        | false => {}
    
    form.age < 13 ?
        | true => errors.push(ValidationError.TooYoung)
        | false => {}
    
    errors.len() > 0 ?
        | true => .Invalid(errors)
        | false => .Valid(ValidatedData.from(form))
}
```

### Partial Success Handling

```zen
BatchResult<T> = {
    successful: []T,
    failed: [](index: usize, error: Error),
}

process_batch_partial: (items: []Item) BatchResult<ProcessedItem> = {
    result := BatchResult{
        successful: [],
        failed: [],
    }
    
    items.enumerate().each(((index, item)) => {
        process_result := process_item(item)
        
        process_result ?
            | .Ok -> processed => result.successful.push(processed)
            | .Err -> error => result.failed.push((index, error))
    })
    
    result
}
```

## Error Type Design

### Hierarchical Error Types

```zen
// Define error hierarchy for different layers
AppError = 
    | DomainError(rule: DomainRule, context: String)
    | InfrastructureError(kind: InfraKind, details: String)
    | ExternalServiceError(service: String, status: u16, message: String)
    | ValidationError(field: String, constraint: String)

DomainRule = 
    | InsufficientBalance
    | OrderAlreadyProcessed
    | UserNotAuthorized
    | ResourceLocked

InfraKind = 
    | DatabaseConnection
    | FileSystem
    | Network
    | Configuration

// Add helper methods
AppError.is_retryable: (self: AppError) bool = {
    self ?
        | .ExternalServiceError -> _, status, _ => status >= 500
        | .InfrastructureError -> kind, _ => {
            kind ?
                | .DatabaseConnection => true
                | .Network => true
                | _ => false
        }
        | _ => false
}

AppError.to_http_status: (self: AppError) u16 = {
    self ?
        | .ValidationError -> _ => 400
        | .DomainError -> UserNotAuthorized, _ => 403
        | .DomainError -> _ => 422
        | .ExternalServiceError -> _, status, _ => status
        | .InfrastructureError -> _ => 500
}
```

## Integration with Result Methods

### Functional Composition

```zen
// Chain Result methods for clean error handling
fetch_and_process: (id: Id) Result<Processed, Error> = {
    fetch_raw(id)
        .and_then(validate)           // Continue if Ok
        .and_then(transform)          // Chain operations
        .map(optimize)                // Transform success value
        .map_err(wrap_error)          // Transform error value
        .or_else(fetch_from_cache)    // Fallback on error
}

// Custom Result extensions
Result<T, E>.retry: (self: Result<T, E>, times: u32) Result<T, E> = {
    self ?
        | .Ok -> _ => self
        | .Err -> e => {
            times > 0 && e.is_retryable() ?
                | true => retry(times - 1)
                | false => self
        }
}
```

## Best Practices

### 1. Use `?` for Linear Error Flow

```zen
// Good: Clear, linear error propagation
process: () Result<Output, Error> = {
    input := read_input()?
    validated := validate(input)?
    transformed := transform(validated)?
    .Ok(transformed)
}
```

### 2. Add Context at Boundaries

```zen
// Good: Add context when crossing module boundaries
repository_get_user: (id: UserId) Result<User, RepoError> = {
    db.execute_query(query, [id])
        .map_err((e) => RepoError.QueryFailed("get_user", id, e))?
        .first()
        .ok_or(RepoError.NotFound("user", id))?
}
```

### 3. Handle Errors at the Right Level

```zen
// Good: Handle at the level that can take action
controller_handle_request: (req: Request) Response = {
    result := service.process(req)
    
    result ?
        | .Ok -> data => Response.ok(data)
        | .Err -> error => {
            log.error("Request failed", error)
            
            response := error ?
                | .ValidationError -> _ => Response.bad_request(error.message)
                | .AuthError -> _ => Response.unauthorized()
                | .NotFound -> _ => Response.not_found()
                | _ => Response.internal_error()
            
            response
        }
}
```

### 4. Document Error Conditions

```zen
// Good: Document what errors can occur
/// Fetches user data from the database
/// 
/// Errors:
/// - DatabaseError: Connection failed or query error
/// - NotFound: User with given ID doesn't exist
/// - ValidationError: Invalid user ID format
get_user: (id: String) Result<User, Error> = {
    // Implementation
}
```

## Summary

Zenlang's error handling without `try`:
- **No `try` keyword** - Completely absent from the language
- **`?` operator only** - Single, consistent error propagation mechanism  
- **Errors as values** - Type-safe, explicit error handling
- **Composable patterns** - Build complex error handling from simple parts
- **Clear error flow** - Errors bubble predictably through `?`
- **Context preservation** - Add meaning as errors propagate
- **Flexible strategies** - Choose between fail-fast, retry, fallback, etc.

The absence of `try` and the unified use of `?` makes Zenlang's error handling more consistent, predictable, and easier to reason about than traditional exception-based or multi-keyword approaches.