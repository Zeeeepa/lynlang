# Codegen Bugs Review

**Last Updated**: 2025-01-27

## Critical Bugs Fixed ✅

1. **Pattern Matching: Basic Blocks Without Terminators** (`patterns.rs`)
   - Fixed: Restructured control flow with proper test/body blocks
   - Tests: `test_pattern_matching_compiles`, `test_pattern_matching_with_return`

2. **Pattern Matching: Terminator in Middle of Block** (`patterns.rs:96-101`)
   - Fixed: Check for existing terminators before adding merge branch
   - Tests: `test_pattern_matching_with_return`

3. **Conditional: Missing Terminator Check** (`control_flow.rs:31-45`)
   - Fixed: Added terminator checks before adding merge branches
   - Tests: `test_conditional_with_return`

4. **Phi Node: Wrong Basic Block References** (`patterns/compile.rs:791-804`)
   - Fixed: Capture end blocks after branches instead of start blocks
   - Tests: `test_pattern_matching_phi_node_basic_blocks`

5. **Void Return Type Bug** (`expressions/control.rs:227-232`)
   - Fixed: Check if function is void before building return instruction
   - Tests: `test_void_function_with_expression`, `test_void_function_no_return`

## Known Issues ⚠️

- **Pointer dereferencing**: Hardcoded i32 when type info missing (`pointers.rs:141-142`)
- **Pattern matching**: Returns dummy i32 for void expressions (`patterns.rs:110`)
- **Array codegen**: Hardcoded i32 element type (`functions/arrays.rs`) - breaks `Array<i64>`, `Array<f64>`
- **Nested struct field access**: Runtime bug - field values swapped (`structs.rs`) - see `tests/known_bugs/README.md`

## Test Coverage

**8 codegen integration tests** covering:
- Pattern matching compilation
- Control flow with returns
- Void functions
- Nested struct access (compiles but runtime bug exists)
- Phi node correctness

**Coverage**: ~70% of critical codegen bugs tested

**Missing**: Stdlib-based tests, execution/runtime tests, complex nested control flow

## Status Summary

- ✅ All terminator bugs fixed and tested
- ✅ Basic phi node bugs fixed and tested
- ⚠️ Specific payload extraction phi node bug not directly tested (requires stdlib)
- ⚠️ GEP bugs partially tested (nested structs compile but have runtime bug)
- ❌ Execution/runtime bugs not tested
