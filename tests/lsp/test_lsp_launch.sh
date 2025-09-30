#!/bin/bash
# Simple test script to verify LSP can be launched

echo "Testing Zen LSP launch..."

# Start the LSP in background and capture its PID
timeout 2s ../../target/debug/zen-lsp 2>&1 | head -20 &
LSP_PID=$!

# Give it a moment to start
sleep 0.5

# Check if it's running
if ps -p $LSP_PID > /dev/null 2>&1; then
    echo "✓ LSP launched successfully"
    # Kill it gracefully
    kill $LSP_PID 2>/dev/null
else
    echo "✗ LSP failed to launch or exited immediately"
    exit 1
fi

echo "LSP test completed"