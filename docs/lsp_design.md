# Zen Language Server Protocol (LSP) Design

## Overview
The Zen LSP provides IDE support for the Zen programming language, offering features like syntax highlighting, auto-completion, diagnostics, and more.

## Architecture

### Core Components

1. **LSP Server** (`src/lsp/main.rs`)
   - Handles communication with IDE clients
   - Implements the Language Server Protocol
   - Manages document state and workspace

2. **Document Manager**
   - Tracks open documents
   - Maintains document versions
   - Handles incremental updates

3. **Analysis Engine**
   - Performs syntax analysis
   - Type checking
   - Semantic analysis
   - Error detection

4. **Symbol Database**
   - Maintains symbol tables
   - Tracks definitions and references
   - Supports cross-file analysis

## Implemented Features

### Basic Features
- [x] Text document synchronization
- [x] Diagnostics (errors and warnings)
- [x] Document symbols
- [x] Workspace symbols

### Advanced Features (Planned)
- [ ] Code completion
- [ ] Hover information
- [ ] Go to definition
- [ ] Find references
- [ ] Rename symbol
- [ ] Code formatting
- [ ] Code actions (quick fixes)

## Quick Syntax Check Tool

For immediate syntax checking without full LSP, use `zen-check`:

```bash
# Check a single file
zen-check file.zen

# Check all files in a directory
zen-check src/

# Watch mode for continuous checking
zen-check --watch file.zen
```

## Implementation Status

### Current Implementation
The basic LSP server is implemented in `src/lsp/main.rs` with:
- Tower-lsp framework integration
- Basic document handling
- Diagnostic reporting
- Symbol extraction

### Usage

1. **Build the LSP server:**
```bash
cargo build --bin zen-lsp
```

2. **Configure your editor:**

**VS Code** (settings.json):
```json
{
  "zen.languageServer.path": "path/to/zen-lsp",
  "zen.languageServer.args": []
}
```

**Neovim** (with nvim-lspconfig):
```lua
require'lspconfig'.zen.setup{
  cmd = {"zen-lsp"},
  filetypes = {"zen"},
  root_dir = require'lspconfig'.util.root_pattern("Cargo.toml", ".git"),
}
```

## Testing the LSP

Run the LSP test suite:
```bash
cargo test --bin zen-lsp
```

Test with a client:
```bash
# Start the LSP server
zen-lsp

# In another terminal, send LSP messages
echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}' | zen-lsp
```

## Development Workflow

1. **Syntax Checking During Development:**
   ```bash
   # Run syntax check on save
   cargo watch -x "run --bin zen-check -- src/"
   ```

2. **LSP Development:**
   ```bash
   # Run LSP with logging
   RUST_LOG=debug zen-lsp
   ```

3. **Integration Testing:**
   ```bash
   # Test with sample files
   ./scripts/test-lsp.sh
   ```

## Error Reporting Format

The LSP reports errors in the standard format:
```json
{
  "uri": "file:///path/to/file.zen",
  "diagnostics": [{
    "range": {
      "start": {"line": 10, "character": 5},
      "end": {"line": 10, "character": 15}
    },
    "severity": 1,
    "message": "Undefined variable: foo"
  }]
}
```

## Future Enhancements

1. **Incremental Compilation**
   - Cache parsed ASTs
   - Incremental type checking
   - Faster response times

2. **Semantic Tokens**
   - Enhanced syntax highlighting
   - Semantic coloring

3. **Code Intelligence**
   - Type-aware completions
   - Smart rename with cascading updates
   - Extract function/variable refactoring

4. **Debugging Support**
   - Debug adapter protocol (DAP) implementation
   - Breakpoint validation
   - Variable inspection