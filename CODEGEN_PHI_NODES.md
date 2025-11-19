# Phi Node Implementation Status

## Summary
Phi nodes are used in LLVM to merge values from multiple basic blocks. They're essential for correct code generation of conditionals and pattern matches that return values.

## Current Status

### ✅ Implemented Correctly
- **control_flow.rs** - Full phi node implementation for conditionals (LLVM-level)
- **patterns/compile.rs** - Phi nodes for enum pattern matching
- **functions/stdlib/fs.rs** - Phi nodes for Result merging
- **functions/arrays.rs** - Phi nodes for Option merging

### ⚠️ Needs Attention
- **expressions/patterns.rs** - QuestionMatch and Conditional return dummy i32 instead of phi nodes
  - Lines 107-110: QuestionMatch returns `i32(0)` 
  - Lines 175-176: Conditional returns `i32(0)`

## Architecture

Phi nodes are created in blocks that have multiple predecessors:

```rust
// Compile then block
compiler.builder.position_at_end(then_block);
let then_val = compiler.compile_expression(&then_expr)?;

// Compile else block  
compiler.builder.position_at_end(else_block);
let else_val = compiler.compile_expression(&else_expr)?;

// At merge point with two predecessors
compiler.builder.position_at_end(merge_block);
let phi = compiler.builder.build_phi(then_val.get_type(), "result")?;
phi.add_incoming(&[(&then_val, then_block), (&else_val, else_block)]);
Ok(phi.as_basic_value())
```

## Issues with Current Implementation in patterns.rs

### Problem 1: No Value Tracking
`QuestionMatch` and `Conditional` in patterns.rs:
- Create multiple blocks for arms
- Don't capture the values produced by each arm
- Return dummy value instead of merging via phi node

### Problem 2: Type Mismatches
- If arms have different types, phi node type must match
- Currently just returns i32 regardless of actual arm types

### Problem 3: Void Handling
- Void expressions still return dummy i32
- Should properly handle void returns without phi nodes

## Fix Strategy

### Phase 1: Value Tracking (Current)
Modify patterns.rs to track values from each arm:

```rust
// Store values from each arm
let mut arm_values = Vec::new();
let mut arm_blocks = Vec::new();

for arm in arms {
    // ... compile arm body ...
    let value = compiler.compile_expression(&arm.body)?;
    arm_values.push(value);
    arm_blocks.push(compiler.builder.get_insert_block().unwrap());
    // ... branch to merge ...
}

// Merge with phi node
compiler.builder.position_at_end(merge_block);
if !arm_values.is_empty() {
    let phi = compiler.builder.build_phi(arm_values[0].get_type(), "result")?;
    for (val, block) in arm_values.iter().zip(arm_blocks.iter()) {
        phi.add_incoming(&[(val, *block)]);
    }
    Ok(phi.as_basic_value())
} else {
    // Void case
    Ok(compiler.context.i32_type().const_int(0, false).into())
}
```

### Phase 2: Type Inference
- Infer the common type of all arms
- Handle type mismatch errors
- Cast incompatible types if needed

### Phase 3: Void Expression Handling
- Distinguish between expressions that return void vs. those that return values
- For void returns, don't create phi nodes
- Just merge control flow

## Test Cases Needed

```rust
#[test]
fn test_conditional_with_value_merge() {
    // Both branches return same type -> phi node
    let x = (true ? { 42 } : { 24 });
    assert_eq!(x, 42);
}

#[test]
fn test_conditional_void_branches() {
    // Both branches return void -> no phi
    true ? { print("yes") } : { print("no") };
}

#[test]
fn test_nested_conditionals() {
    // Nested phi nodes should work correctly
    let x = (true ? { (false ? { 1 } : { 2 }) } : { 3 });
    assert_eq!(x, 2);
}

#[test]
fn test_question_match_merge() {
    // Pattern match with value merge
    let x = (Some(42) ? 
        | Some(v) { v + 1 }
        | None { 0 }
    );
    assert_eq!(x, 43);
}
```

## Dependencies

- Requires proper type inference (expressions/inference.rs) 
- Requires tracking expression types through compilation
- Requires understanding of which arms can reach merge block (early returns)

## Priority
**Medium** - Correctness issue, but patterns.rs currently used mainly for pattern matching (which does use phi nodes). The QuestionMatch/Conditional variants in patterns.rs may not be heavily used.

Check if patterns.rs QuestionMatch/Conditional are actually used in practice vs. going through different code paths.

