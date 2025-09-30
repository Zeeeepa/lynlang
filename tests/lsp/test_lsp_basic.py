#!/usr/bin/env python3
"""Basic test for Zen LSP server."""

import json
import subprocess
import sys

def test_lsp_basic():
    """Test basic LSP functionality."""
    print("Testing basic Zen LSP server...")

    # Start LSP server
    lsp_path = "../../target/release/zen-lsp"
    try:
        server = subprocess.Popen(
            [lsp_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
    except FileNotFoundError:
        print(f"Error: LSP server not found at {lsp_path}")
        return False

    try:
        # Send initialize request
        request = {
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "processId": None,
                "capabilities": {}
            }
        }

        content = json.dumps(request)
        message = f"Content-Length: {len(content)}\r\n\r\n{content}"
        server.stdin.write(message.encode())
        server.stdin.flush()

        # Try to read response
        response_header = b""
        for _ in range(1000):  # Read up to 1000 bytes for header
            byte = server.stdout.read(1)
            if not byte:
                break
            response_header += byte
            if b"\r\n\r\n" in response_header:
                break

        if response_header:
            print("✓ LSP server responded to initialize")

            # Parse content length
            for line in response_header.split(b"\r\n"):
                if line.startswith(b"Content-Length:"):
                    length = int(line.split(b":")[1].strip())
                    body = server.stdout.read(length)
                    response = json.loads(body)
                    if "result" in response:
                        print("✓ Initialize successful")
                        print(f"  Server capabilities: {list(response['result'].get('capabilities', {}).keys())[:5]}")
                        return True
        else:
            print("✗ No response from LSP server")
            stderr = server.stderr.read(1000).decode('utf-8', errors='ignore')
            if stderr:
                print(f"  Server error: {stderr}")

        return False

    finally:
        server.terminate()
        server.wait(timeout=2)

if __name__ == "__main__":
    success = test_lsp_basic()
    sys.exit(0 if success else 1)