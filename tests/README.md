# Tests

## Rust Tests
```bash
cargo test          # All tests
cargo test --tests  # Integration tests only
```

## LSP Tests
```bash
python3 tests/lsp/test_lsp.py
```

## Test Files

| File | Tests | Description |
|------|-------|-------------|
| `codegen_integration.rs` | 8 | LLVM codegen |
| `parser_integration.rs` | 10 | Parser syntax |
| `lexer_integration.rs` | 8 | Lexer tokens |
| `allocator_compilation.rs` | 11 | Memory allocation |
| `lsp_text_edit.rs` | 11 | LSP text edits |
| `lexer_tests.rs` | 2 | Lexer edge cases |
| `parser_tests.rs` | 2 | Parser edge cases |
| `lsp/test_lsp.py` | 21 | LSP JSON-RPC |
| `lsp/client.py` | - | LSP client library |
