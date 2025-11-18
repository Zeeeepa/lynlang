# Test Coverage Gap Analysis

## Problem: Pattern Matching Bug Slipped Through

**Date**: 2025-01-27  
**Bug**: Pattern matching with `?` operator created basic blocks without terminators, causing LLVM verification errors.

**Root Cause**: The test suite only tested **parsing**, not **codegen**. The bug was in LLVM IR generation, which parser tests couldn't catch.

## Current Test Coverage

### ✅ What We Test
- **Parser tests** (`tests/parser_integration.rs`): Verify code parses correctly
- **Lexer tests** (`tests/lexer_integration.rs`): Verify tokens are recognized
- **LSP tests**: Test language server features

### ❌ What We DON'T Test
- **Codegen tests**: Verify LLVM IR generation is correct
- **Control flow**: Verify basic blocks are properly terminated
- **Type checking in codegen**: Verify types are correctly handled
- **Memory management**: Verify heap/stack allocation is correct

## The Bug That Got Through

```zen
person.age > 18 ?
    | true { io.println("Hello") }
    | false { io.println("Hi") }
```

**What happened**:
1. Parser tests passed ✅ (code parsed correctly)
2. Codegen created `arm_blocks` but never terminated them ❌
3. LLVM verification failed: "Basic Block does not have terminator"
4. Bug only discovered when user tried to compile real code

## Solution: Add Codegen Integration Tests

Created `tests/codegen_integration.rs` to:
- Actually compile code (not just parse)
- Catch LLVM verification errors
- Verify control flow is correct
- Test pattern matching codegen specifically

## Recommendations

1. **Add codegen tests for all control flow constructs**:
   - Pattern matching (`?`)
   - Conditionals (`if/else`)
   - Loops (`loop`, `while`)
   - Returns and breaks

2. **Add tests for complex type scenarios**:
   - Generic types
   - Nested structs
   - Enum variants with payloads

3. **Consider adding execution tests**:
   - Compile AND run code
   - Verify output matches expectations
   - Catch runtime bugs

4. **Review existing tests**:
   - Many tests in `tests/known_bugs/` should be moved to proper test suite once bugs are fixed
   - Consolidate duplicate tests

## Test Infrastructure

The new `codegen_integration.rs` test file:
- Uses `Compiler::get_module()` to compile code
- Automatically catches LLVM verification errors
- Can be extended to test more codegen scenarios

