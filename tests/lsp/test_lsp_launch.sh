#!/bin/bash
# Simple test script to verify LSP can be launched

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LSP_BINARY="$PROJECT_ROOT/target/debug/zen-lsp"

if [ ! -f "$LSP_BINARY" ]; then
    echo "Error: LSP binary not found at $LSP_BINARY"
    echo "Please build it first: cargo build --bin zen-lsp"
    exit 1
fi

echo "Testing Zen LSP launch..."

# Start the LSP in background and capture its PID
timeout 2s "$LSP_BINARY" 2>&1 | head -20 &
LSP_PID=$!

# Give it a moment to start
sleep 0.5

# Check if it's running
if ps -p $LSP_PID > /dev/null 2>&1; then
    echo "✓ LSP launched successfully"
    # Kill it gracefully
    kill $LSP_PID 2>/dev/null || true
else
    echo "✗ LSP failed to launch or exited immediately"
    exit 1
fi

echo "LSP test completed"