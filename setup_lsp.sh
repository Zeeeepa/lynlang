#!/bin/bash
# Quick setup script for Zen LSP

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LSP_BINARY="$SCRIPT_DIR/target/release/zen-lsp"

echo "🔧 Setting up Zen LSP..."

# Build the LSP if it doesn't exist
if [ ! -f "$LSP_BINARY" ]; then
    echo "📦 Building LSP server..."
    cd "$SCRIPT_DIR"
    cargo build --bin zen-lsp --release
fi

echo "✅ LSP binary: $LSP_BINARY"

# Option 1: Add symlink to /usr/local/bin
if [ -w /usr/local/bin ]; then
    echo "🔗 Creating symlink in /usr/local/bin..."
    sudo ln -sf "$LSP_BINARY" /usr/local/bin/zen-lsp
    echo "✅ Symlink created: /usr/local/bin/zen-lsp -> $LSP_BINARY"
elif [ -w "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
    ln -sf "$LSP_BINARY" "$HOME/.local/bin/zen-lsp"
    echo "✅ Symlink created: $HOME/.local/bin/zen-lsp -> $LSP_BINARY"
    echo "💡 Make sure ~/.local/bin is in your PATH"
else
    echo "⚠️  Cannot create symlink (no write permission). Use full path in VS Code settings:"
    echo "   \"zen.serverPath\": \"$LSP_BINARY\""
fi

# Build VS Code extension if needed
if [ -d "$SCRIPT_DIR/vscode-extension" ]; then
    echo "📦 Building VS Code extension..."
    cd "$SCRIPT_DIR/vscode-extension"
    if [ ! -d "node_modules" ]; then
        npm install
    fi
    npm run compile
    echo "✅ VS Code extension compiled"
    echo ""
    echo "📝 To use in VS Code:"
    echo "   1. Open vscode-extension folder in VS Code"
    echo "   2. Press F5 to launch Extension Development Host"
    echo "   OR"
    echo "   3. Package and install the extension"
fi

echo ""
echo "✅ Setup complete! The LSP should now be available."
echo ""
echo "🧪 Test it:"
echo "   $LSP_BINARY --help 2>&1 || echo 'LSP is ready (no --help flag)'"


