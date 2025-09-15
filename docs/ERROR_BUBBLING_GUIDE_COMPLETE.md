# Zenlang Error Bubbling Guide - Complete

## Executive Summary

Zenlang has **NO** `try` keyword. The word "try" does not exist in the language - not as a keyword, not as an operator, not as part of any syntax. Instead, Zenlang uses the `?` suffix operator for error propagation and pattern matching for error handling. This guide explains how to handle errors effectively without `try`.

## Core Concepts

### 1. Errors Are Values
```zen
// Functions return Result<T, E> or Option<T>
read_file: (path: String) Result<String, io.Error>
parse_int: (s: String) Result<i32, ParseError>
find_user: (id: u64) Option<User>
```

### 2. The `?` Suffix Operator

The `?` operator is the primary mechanism for error propagation:

```zen
// If read_file returns Err, the function returns that error immediately
// If it returns Ok(value), execution continues with the value
content := read_file(path)?
```

### 3. Pattern Matching with `?`

For explicit error handling, use pattern matching:

```zen
result ?
    | .Ok -> value => process(value)
    | .Err -> error => handle_error(error)
```

## Error Bubbling Patterns

### Pattern 1: Simple Propagation

The most common pattern - just add `?` to bubble errors up:

```zen
process_file: (path: String) Result<Data, Error> = {
    content := read_file(path)?       // Bubble file errors
    parsed := parse_json(content)?    // Bubble parse errors
    validated := validate(parsed)?    // Bubble validation errors
    .Ok(validated)
}
```

### Pattern 2: Error Transformation

Transform errors as they bubble up to add context:

```zen
load_config: (path: String) Result<Config, AppError> = {
    // Read file and transform io.Error to AppError
    content := fs.read_file(path)
        .map_err((e) => AppError.FileError(path, e))?
    
    // Parse JSON and transform parse error
    data := json.parse(content)
        .map_err((e) => AppError.ParseError(e.to_string()))?
    
    .Ok(extract_config(data))
}
```

### Pattern 3: Contextual Error Handling

Add context while handling errors:

```zen
fetch_user_data: (id: u64) Result<UserData, Error> = {
    // Try primary source
    fetch_from_database(id) ?
        | .Ok -> data => .Ok(data)
        | .Err -> db_err => {
            // Log database error
            log.warn("Database fetch failed: $(db_err)")
            
            // Try cache as fallback
            fetch_from_cache(id) ?
                | .Ok -> data => .Ok(data)
                | .Err -> cache_err => {
                    // Both failed, return comprehensive error
                    .Err(Error.FetchFailed{
                        user_id: id,
                        db_error: db_err,
                        cache_error: cache_err
                    })
                }
        }
}
```

### Pattern 4: Collecting Multiple Errors

When processing batches, collect all errors:

```zen
process_batch: (items: []Item) ([]Success, []Error) = {
    mut successes := []Success{}
    mut errors := []Error{}
    
    for item in items {
        process_item(item) ?
            | .Ok -> result => successes.push(result)
            | .Err -> err => errors.push(err)
    }
    
    (successes, errors)
}
```

### Pattern 5: Early Return with Cleanup

Use defer for cleanup when errors bubble:

```zen
process_with_resources: (input: Input) Result<Output, Error> = {
    resource := acquire_resource()?
    defer resource.release()  // Always runs, even on error
    
    // Multiple operations that might fail
    step1 := resource.operation1(input)?
    step2 := resource.operation2(step1)?
    step3 := resource.operation3(step2)?
    
    .Ok(step3)
}
```

### Pattern 6: Validation Chain

Chain multiple validations:

```zen
validate_request: (req: Request) Result<ValidRequest, ValidationError> = {
    // Chain validations - any failure bubbles up
    validated := validate_auth(req)?
    validated := validate_permissions(validated)?
    validated := validate_data(validated)?
    validated := validate_business_rules(validated)?
    
    .Ok(validated)
}
```

### Pattern 7: Result Combinators

Use functional combinators for complex error flows:

```zen
complex_operation: (input: String) Result<Output, Error> = {
    parse(input)
        .and_then(validate)           // Chain successful operations
        .and_then(transform)
        .map_err((e) => {             // Transform any error
            log.error("Operation failed: $(e)")
            Error.OperationFailed(e)
        })
        .or_else((e) => {            // Try recovery on error
            e ?
                | .Recoverable -> _ => try_alternative(input)
                | _ => .Err(e)
        })
}
```

## Error Types Best Practices

### 1. Domain-Specific Error Types

Create error types that match your domain:

```zen
// Domain-specific error enum
ApiError = 
    | NotFound(resource: String)
    | Unauthorized(user: String)
    | RateLimited(retry_after: Duration)
    | ValidationFailed(errors: []FieldError)
    | InternalError(cause: String)

// Convert between error types
from_io_error: (e: io.Error) ApiError = {
    e ?
        | .NotFound => ApiError.NotFound("file")
        | .PermissionDenied => ApiError.Unauthorized("system")
        | _ => ApiError.InternalError(e.to_string())
}
```

### 2. Error Context

Include relevant context in errors:

```zen
FileOperationError: {
    path: String
    operation: String  // "read", "write", "delete"
    cause: io.Error
    timestamp: Time
}

// Use it
read_config: (path: String) Result<Config, FileOperationError> = {
    fs.read_file(path) ?
        | .Ok -> content => parse_config(content)
        | .Err -> io_err => .Err(FileOperationError{
            path: path,
            operation: "read",
            cause: io_err,
            timestamp: Time.now()
        })
}
```

## Common Scenarios

### HTTP Request Handling

```zen
handle_request: (req: HttpRequest) Result<HttpResponse, ApiError> = {
    // Validate request
    validated := validate_request(req)?
    
    // Authenticate
    user := authenticate(validated.auth_token)?
    
    // Authorize
    check_permissions(user, validated.resource)?
    
    // Process
    result := process_business_logic(validated, user)?
    
    // Return response
    .Ok(HttpResponse.json(result))
}
```

### Database Operations

```zen
save_user: (user: User, db: Database) Result<UserId, DbError> = {
    // Start transaction
    tx := db.begin_transaction()?
    defer tx.rollback_if_not_committed()
    
    // Validate uniqueness
    existing := db.query("SELECT id FROM users WHERE email = ?", [user.email])?
    existing.is_empty() ?
        | false => .Err(DbError.DuplicateEmail(user.email))
        | true => {}
    
    // Insert user
    user_id := db.insert("users", user)?
    
    // Commit transaction
    tx.commit()?
    
    .Ok(user_id)
}
```

### File Processing

```zen
process_csv: (path: String) Result<ProcessedData, Error> = {
    // Read file
    content := fs.read_file(path)?
    
    // Parse CSV
    lines := content.split('\n')
    mut records := []Record{}
    
    for (i, line) in lines.enumerate() {
        i == 0 ?  // Skip header
            | true => continue
            | false => {}
        
        parse_csv_line(line) ?
            | .Ok -> record => records.push(record)
            | .Err -> e => {
                log.warn("Skipping line $(i): $(e)")
                continue  // Skip bad lines
            }
    }
    
    // Validate we got some data
    records.is_empty() ?
        | true => .Err(Error.NoValidRecords)
        | false => .Ok(ProcessedData{ records: records })
}
```

## Testing Error Paths

```zen
test "error propagation" {
    // Test successful case
    result := process_file("valid.json")
    assert(result.is_ok())
    
    // Test file not found
    result := process_file("nonexistent.json")
    assert(result.is_err())
    result ?
        | .Err -> .FileError -> path, _ => assert(path == "nonexistent.json")
        | _ => assert(false, "Expected FileError")
    
    // Test parse error
    fs.write_file("invalid.json", "not json")
    result := process_file("invalid.json")
    assert(result.is_err())
    result ?
        | .Err -> .ParseError -> _ => assert(true)
        | _ => assert(false, "Expected ParseError")
}
```

## Migration Guide

### From Languages with try/catch

**Before (JavaScript/Java/C++):**
```javascript
try {
    const data = readFile(path);
    const parsed = JSON.parse(data);
    return process(parsed);
} catch (error) {
    console.error('Error:', error);
    throw new ProcessError(error);
}
```

**After (Zenlang):**
```zen
process: (path: String) Result<Processed, ProcessError> = {
    data := read_file(path)?
    parsed := parse_json(data)?
    processed := process(parsed)?
    .Ok(processed)
}
```

### From Languages with exceptions

**Before (Python):**
```python
def process_data(data):
    try:
        validated = validate(data)
        transformed = transform(validated)
        return save(transformed)
    except ValidationError as e:
        logger.error(f"Validation failed: {e}")
        raise
    except TransformError as e:
        logger.error(f"Transform failed: {e}")
        raise ProcessingError(e)
```

**After (Zenlang):**
```zen
process_data: (data: Data) Result<Saved, ProcessingError> = {
    validate(data) ?
        | .Ok -> validated => {
            transform(validated) ?
                | .Ok -> transformed => save(transformed)
                | .Err -> e => {
                    log.error("Transform failed: $(e)")
                    .Err(ProcessingError.TransformFailed(e))
                }
        }
        | .Err -> e => {
            log.error("Validation failed: $(e)")
            .Err(ProcessingError.ValidationFailed(e))
        }
}
```

## Key Takeaways

1. **No `try` keyword exists** - Use `?` operator for error propagation
2. **Errors are values** - Handle them with pattern matching
3. **Explicit propagation** - Every `?` is a potential return point
4. **Type safety** - Can't forget to handle errors
5. **Zero overhead** - No exception unwinding or hidden costs
6. **Composable** - Errors compose naturally through Result types
7. **Local reasoning** - Error handling is explicit and visible

Remember: In Zenlang, there is no `try` keyword, no exceptions, and no hidden control flow. Every error is a value, and every error propagation point is marked with `?`.