# Code Organization and Architecture

**Last Updated**: 2025-01-27

## Large Files Analysis

### Files Over 1000 Lines

These files are large but appear to be well-organized:

1. **`src/typechecker/mod.rs` (1929 lines)**
   - ✅ **Well-organized**: Already split into 11 submodules
   - ✅ **Single responsibility**: Type checking only
   - ✅ **No god file concerns**: Functions delegate to submodules
   - **Status**: Acceptable - large but modular

2. **`src/lsp/navigation.rs` (1665 lines)**
   - ⚠️ **Could be split**: Contains 4 main handlers (definition, type_definition, references, highlight)
   - **Recommendation**: Consider splitting into:
     - `navigation/definition.rs`
     - `navigation/references.rs`
     - `navigation/highlight.rs`
   - **Status**: Moderate concern - functional but could be cleaner

3. **`src/ffi/mod.rs` (1511 lines)**
   - ⚠️ **Could be split**: FFI is complex domain
   - **Recommendation**: Split by functionality:
     - `ffi/c_types.rs` - C type handling
     - `ffi/calls.rs` - Function call generation
     - `ffi/structs.rs` - Struct handling
   - **Status**: Moderate concern - complex domain warrants splitting

4. **`src/lsp/document_store.rs` (1501 lines)**
   - ⚠️ **Could be split**: Document management + symbol indexing
   - **Recommendation**: Split into:
     - `document_store.rs` - Core document management
     - `symbol_indexing.rs` - Symbol extraction/indexing (already partially separated)
   - **Status**: Moderate concern - two distinct responsibilities

5. **`src/parser/statements.rs` (1493 lines)**
   - ✅ **Well-organized**: Statement parsing is inherently large
   - ✅ **Single responsibility**: Parse statements
   - **Status**: Acceptable - parsing is complex

### Files Under 1000 Lines

All other files are reasonably sized and well-organized.

## Recommendations

### High Priority (Not Critical)
1. **Split `lsp/navigation.rs`** into separate handler files
2. **Split `ffi/mod.rs`** by functionality
3. **Extract symbol indexing** from `document_store.rs`

### Low Priority
- Current organization is functional
- Large files are modular internally
- No immediate refactoring needed

## Test Organization

### ✅ Well-Organized
- **Unit tests**: In `src/` alongside code
- **Integration tests**: Separate `tests/` directory
- **Codegen tests**: New `codegen_integration.rs` for LLVM verification
- **LSP tests**: Separate `tests/lsp/` directory

### Test Status
- **19 unit tests** passing
- **8 codegen integration tests** passing
- **8 parser integration tests** passing
- **9/10 lexer integration tests** passing (1 failure to investigate)

## Documentation Organization

### ✅ Consolidated
- **`tests/CODEGEN_BUGS_REVIEW.md`**: Single comprehensive review document
- **`tests/README.md`**: Test suite overview
- **`CODE_ORGANIZATION.md`**: This file

### Remaining Documentation
- `README.md` - Main project README
- `DESIGN_NOTES.md` - Design decisions
- `SETUP_LSP.md` - LSP setup guide
- `tests/known_bugs/README.md` - Known bugs
- `tests/lsp/README.md` - LSP test guide

All documentation is appropriately placed and not redundant.

