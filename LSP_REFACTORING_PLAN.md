# LSP Server Refactoring Plan

## Current State
- `server.rs`: **6099 lines** - This is a "god file" that does too much
- Contains: 20+ handler functions, DocumentStore implementation, analysis functions, symbol extraction, etc.

## Refactoring Strategy

### Phase 1: Extract Hover Handler ✅ (In Progress)
- Create `hover.rs` module
- Move `handle_hover()` and related helper functions:
  - `get_pattern_match_hover()`
  - `get_enum_variant_hover()`
  - `infer_variable_type()` (or move to type_inference.rs)
  - Helper functions for hover content generation

### Phase 2: Extract Semantic Tokens
- Create `semantic_tokens.rs` module
- Move `handle_semantic_tokens()` and token generation logic

### Phase 3: Extract Call Hierarchy
- Create `call_hierarchy.rs` module
- Move:
  - `handle_prepare_call_hierarchy()`
  - `handle_incoming_calls()`
  - `handle_outgoing_calls()`

### Phase 4: Extract Analysis Functions
- Create `analysis.rs` module
- Move validation/analysis functions:
  - `check_allocator_usage()`
  - `check_pattern_exhaustiveness()`
  - `infer_expression_type_string()`
  - Type inference helpers

### Phase 5: Clean Up DocumentStore
- The `server.rs` has a duplicate `DocumentStore` struct
- Should use the one from `document_store.rs` module
- Remove duplicate implementation

### Phase 6: Extract Remaining Handlers
- Move remaining handlers to appropriate modules:
  - `handle_rename()` → `rename.rs` or keep in `server.rs`
  - `handle_code_action()` → `code_actions.rs`
  - `handle_inlay_hints()` → `inlay_hints.rs`
  - `handle_code_lens()` → `code_lens.rs`
  - `handle_workspace_symbol()` → `workspace.rs`

## Target Structure
```
src/lsp/
├── mod.rs
├── server.rs          (~500-800 lines - main loop + routing)
├── document_store.rs  (already exists)
├── hover.rs           (NEW - hover handler)
├── semantic_tokens.rs (NEW - semantic tokens)
├── call_hierarchy.rs  (NEW - call hierarchy)
├── analysis.rs        (NEW - analysis/validation)
├── navigation.rs      (already exists)
├── completion.rs      (already exists)
├── formatting.rs      (already exists)
├── symbols.rs        (already exists)
├── type_inference.rs  (already exists)
├── types.rs           (already exists)
└── utils.rs           (already exists)
```

## Benefits
1. **Maintainability**: Each module has a single responsibility
2. **Testability**: Easier to test individual modules
3. **Readability**: Smaller files are easier to understand
4. **Performance**: No performance impact, just better organization

