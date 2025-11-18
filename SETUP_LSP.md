# Setting Up Zen LSP

## Quick Setup

The LSP server is built at: `target/release/zen-lsp`

### Option 1: Automated Setup (Recommended)

Run the setup script which handles everything automatically:

```bash
./setup_lsp.sh
```

This script will:
- Build the LSP server if needed
- Create a symlink in `/usr/local/bin` or `~/.local/bin`
- Build the VS Code extension if present
- Provide instructions for next steps

### Option 2: Manual Setup

#### Add to PATH

```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export PATH="$PATH:$(pwd)/target/release"

# Or create a symlink
sudo ln -s "$(pwd)/target/release/zen-lsp" /usr/local/bin/zen-lsp
```

#### Configure VS Code Extension

1. Install the extension from `vscode-extension/` folder
2. In VS Code settings (`.vscode/settings.json`), set one of:
   
   **Relative path (recommended):**
   ```json
   {
     "zen.serverPath": "target/release/zen-lsp"
   }
   ```
   
   **VS Code variable (also supported):**
   ```json
   {
     "zen.serverPath": "${workspaceFolder}/target/release/zen-lsp"
   }
   ```
   
   **Or if the binary is in your PATH:**
   ```json
   {
     "zen.serverPath": "zen-lsp"
   }
   ```

### Option 3: Test Directly

```bash
# Test that the LSP works
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | ./target/release/zen-lsp
```

## Verifying It Works

1. Open a `.zen` file in VS Code
2. Check the Output panel → "Zen Language Server" 
3. You should see LSP messages
4. Hover over symbols - should show type info
5. Go to definition should work
6. Error diagnostics should appear

## Troubleshooting

- **"zen-lsp not found"**: Make sure it's in PATH or configure the full path in settings
- **No hover info**: Check that symbols are being indexed (look in Output panel)
- **Go-to-def doesn't work**: Check that the file parses correctly (look for parse errors)
