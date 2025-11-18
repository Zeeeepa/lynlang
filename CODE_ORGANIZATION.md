# Code Organization

**Last Updated**: 2025-01-27

## Large Files (>1000 lines)

### Well-Organized ✅
- **`src/typechecker/mod.rs`** (1929 lines) - Already split into 11 submodules
- **`src/parser/statements.rs`** (1493 lines) - Statement parsing is inherently large

### Split ✅
- **`src/lsp/navigation.rs`** (1665 → 8 modules) - **COMPLETE**
  - ✅ `navigation/highlight.rs` (~75 lines)
  - ✅ `navigation/utils.rs` (~217 lines)
  - ✅ `navigation/imports.rs` (~37 lines)
  - ✅ `navigation/definition.rs` (~354 lines)
  - ✅ `navigation/type_definition.rs` (~107 lines)
  - ✅ `navigation/references.rs` (~169 lines)
  - ✅ `navigation/scope.rs` (~60 lines)
  - ✅ `navigation/ufc.rs` (~383 lines)
  - ✅ `navigation/mod.rs` (~17 lines)

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
