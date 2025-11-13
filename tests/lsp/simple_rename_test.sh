#!/bin/bash

# Simple test of rename functionality

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LSP_BINARY="$PROJECT_ROOT/target/release/zen-lsp"

if [ ! -f "$LSP_BINARY" ]; then
    echo "Error: LSP binary not found at $LSP_BINARY"
    echo "Please build it first: cargo build --release --bin zen-lsp"
    exit 1
fi

# Start LSP and redirect stderr to a file
"$LSP_BINARY" 2> /tmp/lsp_debug.log &
LSP_PID=$!

sleep 0.5

# Send initialize request
cat <<'EOF' | nc localhost 9999 2>/dev/null || true
Content-Length: 123

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":$$,"rootUri":"file:///tmp","capabilities":{}}}
EOF

# Kill LSP
kill $LSP_PID 2>/dev/null || true

# Show debug output
echo "Debug output:"
cat /tmp/lsp_debug.log
