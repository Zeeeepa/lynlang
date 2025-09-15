# Error Bubbling in Zenlang Without `try`

## Why Zenlang Has NO `try` Keyword

Zenlang takes a radical approach to error handling by completely eliminating the `try` keyword from the language. This isn't just a syntax preference - it's a fundamental design decision that makes error handling more explicit, composable, and predictable.

### Problems with `try` in Other Languages

1. **Hidden Control Flow** - `try` blocks hide where errors can occur
2. **Exception Hell** - Deep call stacks with unclear error propagation
3. **Function Coloring** - `try` often requires special function types
4. **Implicit Behavior** - Errors can skip arbitrary amounts of code
5. **Resource Cleanup** - Complex `finally` blocks and cleanup patterns

### Zenlang's Solution: Errors as Values

In Zenlang, **errors are just values** that you handle with the same pattern matching and operators you use for any other data. There is no special error syntax, no exception mechanism, and definitely no `try` keyword.

## The Three Tools for Error Handling

1. **The `?` suffix operator** - Automatic error propagation
2. **Pattern matching with `?`** - Explicit error handling
3. **Result and Option types** - Type-safe error representation

## The `?` Operator

The `?` suffix operator is your primary tool for error propagation:

```zen
// If parse_int returns an Err, the function returns that Err immediately
// If it returns Ok(value), execution continues with that value
convert_and_double: (s: String) Result<i32, ParseError> = {
    num := parse_int(s)?  // Bubbles error if parse fails
    .Ok(num * 2)
}
```

## Comparison: Traditional vs Zenlang

### Traditional (with try/catch) - NOT ZENLANG
```javascript
// THIS IS NOT ZENLANG - Just for comparison
try {
    const data = readFile(path);
    const parsed = JSON.parse(data);
    return process(parsed);
} catch (e) {
    console.error(e);
    throw e;
}
```

### Zenlang Way (NO try keyword!)
```zen
// Zenlang - errors as values with ? operator
process_file: (path: String) Result<Output, Error> = {
    data := read_file(path)?      // Bubbles read errors
    parsed := json.parse(data)?   // Bubbles parse errors
    processed := process(parsed)? // Bubbles process errors
    .Ok(processed)
}
```

## Error Handling Patterns

### Pattern 1: Let It Bubble
```zen
// Just add ? to bubble errors up
chain_operations: () Result<Data, Error> = {
    step1()?
    step2()?
    step3()?
    .Ok(final_result)
}
```

### Pattern 2: Handle Specific Errors
```zen
// Use pattern matching instead of try/catch
safe_divide: (a: i32, b: i32) i32 = {
    result := divide(a, b)
    
    result ?
        | .Ok -> value => value
        | .Err -> DivisionByZero => {
            log.warn("Division by zero, returning 0")
            0
        }
        | .Err -> other => panic("Unexpected error: $(other)")
}
```

### Pattern 3: Transform Errors
```zen
// Add context while bubbling
load_config: (name: String) Result<Config, AppError> = {
    path := find_config_path(name)?
        .map_err(e -> AppError.ConfigNotFound(name, e))?
    
    content := read_file(path)?
        .map_err(e -> AppError.ReadError(path, e))?
    
    config := parse_config(content)?
        .map_err(e -> AppError.ParseError(name, e))?
    
    .Ok(config)
}
```

### Pattern 4: Multiple Error Sources
```zen
// Handle different error types
fetch_user_data: (id: u64) Result<UserData, Error> = {
    // Database errors bubble as-is
    user := db.get_user(id)?
    
    // Network errors bubble as-is
    profile := api.fetch_profile(user.profile_id)?
    
    // Validation errors bubble as-is
    validated := validate_profile(profile)?
    
    .Ok(UserData{ user, profile: validated })
}
```

## No Need for `try` Because:

1. **`?` is simpler** - One character vs a keyword
2. **Explicit control flow** - You see exactly where errors can occur
3. **Type safety** - Compiler ensures you handle Result types
4. **Composable** - Works naturally with function composition
5. **No hidden control flow** - No invisible exception propagation

## Quick Migration Guide

If you're coming from languages with `try`:

| Other Languages | Zenlang Equivalent |
|----------------|-------------------|
| `try { ... }` | Not needed - use `?` |
| `throw error` | `return .Err(error)` |
| `catch (e) { ... }` | Pattern match: `result ? \| .Err -> e => ...` |
| `finally { ... }` | `defer` statement |
| `try expression` | Just use `?` operator |

## Complete Example

```zen
// A complete example showing error handling without try
process_order: (order_id: u64) Result<Receipt, OrderError> = {
    // Validate order - bubbles validation errors
    order := fetch_order(order_id)?
    validate_order(order)?
    
    // Check inventory - bubbles inventory errors
    check_inventory(order.items)?
    
    // Process payment - handle payment errors explicitly
    payment_result := charge_payment(order.payment_info)
    
    payment_result ?
        | .Ok -> transaction_id => {
            // Update inventory - bubbles update errors
            update_inventory(order.items)?
            
            // Send confirmation - bubbles send errors
            send_confirmation(order.customer_email, transaction_id)?
            
            // Generate receipt
            receipt := generate_receipt(order, transaction_id)?
            .Ok(receipt)
        }
        | .Err -> payment_error => {
            // Log the error and return it
            log.error("Payment failed for order $(order_id): $(payment_error)")
            .Err(OrderError.PaymentFailed(order_id, payment_error))
        }
}

// Using the function
main: () void = {
    result := process_order(12345)
    
    result ?
        | .Ok -> receipt => {
            print("Order successful! Receipt: $(receipt.number)")
        }
        | .Err -> error => {
            print("Order failed: $(error)")
            // No need to catch - errors are just values!
        }
}
```

## Remember

- **There is NO `try` keyword in Zenlang**
- Use `?` for error propagation
- Use pattern matching for error handling
- Errors are values, not exceptions
- This makes error handling explicit and type-safe