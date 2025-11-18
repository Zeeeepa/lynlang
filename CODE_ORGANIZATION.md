# Code Organization

**Last Updated**: 2025-01-27

## Large Files (>1000 lines)

### Well-Organized ✅
- **`src/typechecker/mod.rs`** (1929 lines) - Already split into 11 submodules
- **`src/parser/statements.rs`** (1493 lines) - Statement parsing is inherently large

### Being Split 🔄
- **`src/lsp/navigation.rs`** (1665 → ~400 lines each) - **IN PROGRESS**
  - ✅ `navigation/highlight.rs` extracted (~80 lines)
  - ✅ `navigation/utils.rs` extracted (~180 lines)
  - ✅ `navigation/imports.rs` extracted (~40 lines)
  - ⏳ `navigation/definition.rs` - TODO
  - ⏳ `navigation/type_definition.rs` - TODO
  - ⏳ `navigation/references.rs` - TODO

### Could Be Split ⚠️
- **`src/ffi/mod.rs`** (1511 lines) - Complex FFI domain
- **`src/lsp/document_store.rs`** (1501 lines) - Document management + symbol indexing

**Recommendation**: Continue splitting navigation.rs, then tackle ffi/mod.rs.

## Test Organization ✅

- Unit tests in `src/` alongside code
- Integration tests in `tests/` directory
- Codegen tests: `codegen_integration.rs` (8 tests)
- LSP tests: `tests/lsp/` directory

**Status**: All tests passing (19 unit, 8 codegen, 10 parser, 2 lexer)

## Documentation ✅

- Consolidated: `tests/CODEGEN_BUGS_REVIEW.md` (condensed from 4 files)
- Test overview: `tests/README.md`
- Code organization: This file
