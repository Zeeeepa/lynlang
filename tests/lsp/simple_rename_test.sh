#!/bin/bash

# Simple test of rename functionality

# Start LSP and redirect stderr to a file
./target/release/zen-lsp 2> /tmp/lsp_debug.log &
LSP_PID=$!

sleep 0.5

# Send initialize request
cat <<'EOF' | nc localhost 9999 || true
Content-Length: 123

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":$$,"rootUri":"file:///tmp","capabilities":{}}}
EOF

# Kill LSP
kill $LSP_PID 2>/dev/null || true

# Show debug output
echo "Debug output:"
cat /tmp/lsp_debug.log
