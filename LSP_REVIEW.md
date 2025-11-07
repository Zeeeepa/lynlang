# LSP Hardcoded Logic Review

## Issues Found

### 1. Hardcoded Method Lists
**Location**: `navigation.rs`, `type_inference.rs`, `completion.rs`

**Problem**: Method names are hardcoded as arrays:
- `result_methods = ["raise", "is_ok", "is_err", ...]`
- `option_methods = ["is_some", "is_none", ...]`
- `string_methods = ["len", "to_i32", ...]`
- etc.

**Should**: Query actual method definitions from stdlib AST/symbol table

### 2. Hardcoded Type Inference
**Location**: `type_inference.rs::get_method_return_type()`, `navigation.rs::get_method_return_type()`

**Problem**: Return types are hardcoded in match statements:
```rust
match receiver_type {
    "String" => match method_name {
        "len" => Some("i32".to_string()),
        "split" => Some("Array".to_string()),
        ...
    }
}
```

**Should**: Use TypeChecker to infer actual return types from function signatures

### 3. String-Based Type Parsing
**Location**: `type_inference.rs::parse_generic_type()`, `completion.rs::parse_generic_type()`

**Problem**: Uses regex patterns to parse types:
```rust
if let Some(angle_pos) = type_str.find('<') {
    // Manual parsing...
}
```

**Should**: Use Parser to parse type expressions properly

### 4. Hardcoded Type Names
**Location**: Throughout LSP modules

**Problem**: Type names like "Result", "Option", "String" are hardcoded as strings

**Should**: Use AST types or compiler's type system

### 5. Manual Symbol Extraction
**Location**: `document_store.rs::extract_symbols()`

**Problem**: Manually walks AST to extract symbols

**Should**: Use compiler's symbol table if available, or at least leverage compiler's AST traversal

### 6. Hardcoded Stdlib Paths
**Location**: `navigation.rs::resolve_ufc_method()`

**Problem**: Paths like "core/result.zen", "core/option.zen" are hardcoded

**Should**: Discover stdlib paths or use compiler's module resolution

### 7. Regex-Based Type Inference
**Location**: `type_inference.rs::infer_receiver_type()`, `completion.rs::infer_receiver_type()`

**Problem**: Uses regex patterns to infer types from code strings

**Should**: Use TypeChecker to infer types from AST

### 8. Hardcoded Completion Items
**Location**: `completion.rs::handle_completion()`

**Problem**: Keywords and types are hardcoded as CompletionItem structs

**Should**: Query from compiler/parser or AST

## Recommendations

1. **Create a Compiler Integration Module** (`lsp/compiler_integration.rs`)
   - Wrapper around Compiler that provides LSP-friendly APIs
   - Methods to query symbols, types, methods from compiled code

2. **Use TypeChecker for Type Inference**
   - Replace all hardcoded type inference with TypeChecker calls
   - Use actual type information from AST

3. **Use Parser for Type Parsing**
   - Replace regex-based type parsing with Parser
   - Parse type expressions as proper AST nodes

4. **Query Stdlib from AST**
   - Parse stdlib files once
   - Build symbol table from AST
   - Query methods/types from symbol table instead of hardcoding

5. **Leverage Compiler's Symbol Table**
   - If compiler has a symbol table, use it
   - Otherwise, build one from AST and reuse it

