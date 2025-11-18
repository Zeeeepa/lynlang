# Test Suite

**Last Updated**: 2025-01-27

## Test Organization

### Unit Tests (in `src/`)
- Lexer, parser, typechecker unit tests
- Run with: `cargo test --lib`

### Integration Tests (in `tests/`)

#### Parser Integration (`parser_integration.rs`)
- Tests that code parses correctly
- Does NOT test codegen or execution
- **Limitation**: Won't catch codegen bugs

#### Lexer Integration (`lexer_integration.rs`)
- Tests token recognition
- Verifies lexer handles all syntax correctly

#### Codegen Integration (`codegen_integration.rs`) ⭐ **NEW**
- **Actually compiles code** (not just parses)
- Catches LLVM verification errors
- Tests control flow, phi nodes, GEP operations
- **8 tests** covering critical codegen bugs

#### LSP Tests (`lsp/`)
- Language Server Protocol tests
- Python-based integration tests
- See `lsp/README.md` for details

### Known Bugs (`known_bugs/`)
- Tests that expose known bugs
- Excluded from main test suite
- See `known_bugs/README.md` for details

## Running Tests

```bash
# All tests
cargo test

# Specific test suites
cargo test --test codegen_integration
cargo test --test parser_integration
cargo test --test lexer_integration

# LSP tests
cd tests/lsp && python3 test_unified_lsp.py
```

## Test Coverage

See [`CODEGEN_BUGS_REVIEW.md`](./CODEGEN_BUGS_REVIEW.md) for:
- Bugs found and fixed
- Test coverage analysis
- Known issues and recommendations

## Adding New Tests

1. **Parser/Lexer tests**: Add to `parser_integration.rs` or `lexer_integration.rs`
2. **Codegen tests**: Add to `codegen_integration.rs` (use `compile_code()` helper)
3. **LSP tests**: Add to `lsp/test_unified_lsp.py`

## Test Coverage Gaps

- ⚠️ Execution/runtime tests (compile AND run)
- ⚠️ Stdlib-based tests (Option/Result pattern matching)
- ⚠️ Complex nested control flow
- ⚠️ Property-based testing

