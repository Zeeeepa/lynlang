# Zen LSP Status

**Status**: Feature Complete | **Updated**: January 2026

---

## Current Capabilities

The Zen Language Server provides full IDE support:

| Feature | Status | Notes |
|---------|--------|-------|
| Diagnostics | Complete | Real compiler errors via TypeChecker |
| Hover | Complete | Type info from TypeContext |
| Completion | Complete | Semantic + auto-import |
| Go-to-Definition | Complete | Cross-file, UFC-aware |
| Find References | Complete | Cross-document search |
| Rename | Complete | Scope-aware |
| Signature Help | Complete | Function parameters |
| Document Symbols | Complete | Functions, structs, enums |
| Workspace Symbols | Complete | Cross-file search |
| Formatting | Complete | Uses zen-format |
| Semantic Tokens | Complete | Syntax highlighting |
| Inlay Hints | Complete | Type annotations |
| Code Actions | Complete | Quick fixes |
| Code Lens | Complete | Run/Build/Test buttons |
| Call Hierarchy | Complete | Incoming/outgoing calls |
| Folding Ranges | Complete | Block-based |

---

## Architecture

```
Document Edit
    |
    v
+--------+     +----------+     +-------------+
| Parser | --> | Typecheck| --> | TypeContext |
+--------+     +----------+     +------+------+
                                       |
                                       v
                              +------------------+
                              | Stored in Doc    |
                              +--------+---------+
                                       |
                   +-------------------+-------------------+
                   |                   |                   |
                   v                   v                   v
             Completion            Hover            Inlay Hints
```

The LSP uses `TypeContext` from the TypeChecker as its primary data source:

```rust
pub struct TypeContext {
    pub functions: HashMap<String, FunctionType>,
    pub structs: HashMap<String, Vec<(String, AstType)>>,
    pub enums: HashMap<String, Vec<(String, Option<AstType>)>>,
    pub methods: HashMap<String, AstType>,
    pub method_params: HashMap<String, Vec<(String, AstType)>>,
    pub behavior_impls: HashMap<String, Vec<String>>,
    pub constructors: HashMap<String, AstType>,
    pub variables: HashMap<String, AstType>,
}
```

---

## Recent Improvements

### Semantic Completion (Jan 2026)
- TypeContext-based field/method completion
- Generic type specialization (e.g., `Vec<User>.pop() -> Option<User>`)
- Auto-import with `additionalTextEdits`

### Variable Type Tracking (Jan 2026)
- TypeChecker now records variable types during analysis
- Hover shows `name: Type` instead of just `name`
- Inlay hints use authoritative type information

### Parser Error Recovery (Jan 2026)
- `parse_program_with_recovery()` extracts partial AST on syntax errors
- Enables completions/hover even with errors elsewhere in file

---

## Module Structure

```
src/lsp/
+-- server.rs            # Main server loop, request routing
+-- types.rs             # Document, SymbolInfo types
+-- document_store.rs    # Open document management
+-- analyzer.rs          # Background analysis coordination
|
+-- completion.rs        # Code completion
+-- semantic_completion.rs  # TypeContext-based completion
+-- signature_help.rs    # Function signatures
+-- inlay_hints.rs       # Inline type hints
+-- semantic_tokens.rs   # Syntax highlighting
+-- rename.rs            # Symbol renaming
+-- code_action.rs       # Quick fixes
+-- code_lens.rs         # Run/Build/Test
+-- call_hierarchy.rs    # Call tree
+-- symbols.rs           # Document/workspace symbols
+-- formatting.rs        # Code formatting
|
+-- hover/               # Hover information
|   +-- mod.rs           # Main dispatcher
|   +-- expressions.rs
|   +-- structs.rs
|   +-- builtins.rs
|
+-- navigation/          # Navigation features
    +-- definition.rs    # Go-to-definition
    +-- references.rs    # Find references
    +-- highlight.rs     # Document highlight
```

---

## Configuration

The LSP reads environment variables:
- `RUST_LOG` - Log level (debug, info, warn, error)

Example VS Code settings:
```json
{
    "zen.lsp.path": "/path/to/zen-lsp",
    "zen.lsp.trace.server": "verbose"
}
```

---

## Known Limitations

1. **References** use text-based search (not semantic)
2. **Rename** doesn't track shadowed variables across scopes
3. **Type Hierarchy** not implemented (behaviors don't form hierarchy)

---

## Future Improvements

1. Build semantic symbol index for accurate references
2. Incremental analysis for large files
3. Better cross-module type resolution
