# Known Issues

## Nested Pattern Matching Without Parentheses

**Status**: Open  
**Severity**: Medium  
**Component**: Parser  
**Test**: `test_nested_pattern_matching` in `tests/test_language_features.rs`

### Description
Nested conditional expressions without parentheses are parsed incorrectly. The parser cannot distinguish between arms of outer and inner conditionals without explicit delimiters.

### Example
```zen
classify = (x: i32, y: i32) i32 {
    return x == 0 ? 
        | true => y == 0 ?
            | true => 0 
            | false => 1
        | false => y == 0 ? 
            | true => 2 
            | false => 3
}
```

### Expected Behavior
- `classify(0, 0)` should return `0` ✓
- `classify(0, 1)` should return `1` ✓
- `classify(1, 0)` should return `2` ✗ (returns 0)
- `classify(1, 1)` should return `3` ✗ (returns 0)

### Root Cause
The parser in `src/parser/expressions.rs::parse_pattern_match` consumes all pattern arms greedily when parsing nested conditionals. Without parentheses, it cannot determine where the inner conditional ends and the outer conditional's next arm begins. The outer conditional is parsed as having only one arm, with the second arm being consumed by the inner conditional.

### Workaround
Use parentheses to explicitly delimit nested conditionals:
```zen
// This works correctly:
classify = (x: i32, y: i32) i32 {
    return x == 0 ? 
        | true => (y == 0 ? 
            | true => 0 
            | false => 1)
        | false => (y == 0 ? 
            | true => 2 
            | false => 3)
}
```

### Files Involved
- `src/parser/expressions.rs` - parse_pattern_match function (line 539)
- `src/ast.rs` - ConditionalArm and Pattern structures

### Fix Strategy
To properly fix this, the parser would need to:
1. Track nesting depth of conditionals
2. Stop parsing arms when encountering a `|` at the same depth as the current conditional
3. Implement a more sophisticated lookahead mechanism

For now, requiring parentheses for nested conditionals is the recommended approach.