#!/usr/bin/env python3
"""Simple test for LSP features"""

import json
import subprocess
import time
import os
import sys
from pathlib import Path

def test_lsp():
    """Test basic LSP functionality"""
    print("Testing Zen LSP server...")

    # Check if binary exists
    lsp_binary = Path(__file__).parent.parent.parent / "target" / "debug" / "zen-lsp"
    if not lsp_binary.exists():
        print(f"Error: LSP binary not found at {lsp_binary}")
        return False

    # Create a test file
    test_file = Path(__file__).parent / "test_temp.zen"
    test_content = """
{io, Result, Option, get_default_allocator, HashMap, DynVec} = @std

main = () i32 {
    // Test allocator warning
    bad_map = HashMap()  // Should warn: missing allocator

    // Test UFC method completion
    test_str = "hello"
    length = test_str.len()

    // Test Result UFC
    result = Result.Ok(42)
    val = result.raise()

    // Correct usage with allocator
    alloc = get_default_allocator()
    good_map = HashMap(alloc)
    good_map.insert("key", 100)

    0
}
"""

    try:
        # Write test file
        test_file.write_text(test_content)
        print(f"Created test file: {test_file}")

        # Start LSP server
        proc = subprocess.Popen(
            [str(lsp_binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )

        # Send initialize request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": os.getpid(),
                "rootUri": f"file://{Path.cwd()}",
                "capabilities": {}
            }
        }

        content = json.dumps(init_request)
        message = f"Content-Length: {len(content)}\r\n\r\n{content}"
        proc.stdin.write(message)
        proc.stdin.flush()

        # Wait a bit for response
        time.sleep(1)

        # Try to read response (non-blocking)
        proc.poll()
        if proc.returncode is not None:
            stderr = proc.stderr.read()
            print(f"LSP server exited with code {proc.returncode}")
            print(f"Stderr: {stderr}")
            return False

        print("LSP server appears to be running")

        # Clean up
        proc.terminate()
        proc.wait(timeout=2)
        test_file.unlink()

        return True

    except Exception as e:
        print(f"Error: {e}")
        if test_file.exists():
            test_file.unlink()
        return False

if __name__ == "__main__":
    success = test_lsp()
    sys.exit(0 if success else 1)