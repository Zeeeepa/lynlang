# Tests

## Rust Tests
```bash
cargo test          # All tests
cargo test --tests  # Integration tests only
```

## Test Categories

### Behavioral (tests/ directory)
Actually compile AND run code, verify correct output:
- `behavioral_tests.rs` - Runtime verification (catches semantic bugs)
- `ptr_ref_tests.rs` - Pointer and enum pattern tests
- `allocator_compilation.rs` - Memory allocation tests

### Codegen (tests/ directory)
Compile to LLVM IR and verify it's valid:
- `codegen_integration.rs` - LLVM verification

### Parser/Lexer (tests/ directory)
- `lexer_integration.rs` - Token handling
- `lexer_tests.rs` - Lexer edge cases
- `parser_tests.rs` - Parser regression tests

### LSP (tests/ directory)
- `lsp_text_edit.rs` - Text edit operations
- `lsp/` - Python-based LSP protocol tests

## Running Specific Tests
```bash
cargo test --test behavioral_tests
cargo test --test ptr_ref_tests
cargo test --test codegen_integration
```
