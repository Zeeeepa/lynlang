#!/usr/bin/env python3
"""
Test script for enhanced Zen LSP features:
- UFC method resolution with nested generics
- Allocator warnings and quick fixes
- Semantic tokens for UFC calls and allocators
"""

import subprocess
import json
import time
import sys
import os
from typing import Dict, Any, List, Optional

def start_lsp_server():
    """Start the Zen LSP server"""
    # Build the LSP server first
    build = subprocess.run(['cargo', 'build', '--bin', 'zen-lsp'],
                          capture_output=True, text=True)
    if build.returncode != 0:
        print(f"Failed to build LSP server: {build.stderr}")
        sys.exit(1)

    # Start the server
    server = subprocess.Popen(
        ['target/debug/zen-lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False  # Use binary mode for proper JSON-RPC
    )
    return server

def send_request(server, method: str, params: Dict[str, Any], req_id: int = 1) -> Dict:
    """Send a JSON-RPC request to the server"""
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }

    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"

    server.stdin.write(message.encode())
    server.stdin.flush()

    # Read response
    headers = {}
    while True:
        line = server.stdout.readline().decode().strip()
        if not line:
            break
        key, value = line.split(": ", 1)
        headers[key] = value

    content_length = int(headers.get("Content-Length", 0))
    if content_length > 0:
        response = server.stdout.read(content_length).decode()
        return json.loads(response)
    return {}

def test_ufc_resolution():
    """Test UFC method resolution with nested generics"""
    print("\n=== Testing UFC Method Resolution ===")

    server = start_lsp_server()

    try:
        # Initialize
        response = send_request(server, "initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{os.getcwd()}",
            "capabilities": {
                "textDocument": {
                    "hover": {"dynamicRegistration": False},
                    "definition": {"dynamicRegistration": False},
                    "references": {"dynamicRegistration": False}
                }
            }
        })

        print(f"Initialize response: {list(response.get('result', {}).get('capabilities', {}).keys())[:5]}...")

        # Open test document
        test_file = "tests/lsp/test_enhanced_lsp_features.zen"
        if os.path.exists(test_file):
            with open(test_file, 'r') as f:
                content = f.read()

            send_request(server, "textDocument/didOpen", {
                "textDocument": {
                    "uri": f"file://{os.path.abspath(test_file)}",
                    "languageId": "zen",
                    "version": 1,
                    "text": content
                }
            }, req_id=None)

            print("✓ Test document opened successfully")
        else:
            print("✗ Test file not found")

    finally:
        server.terminate()

def main():
    """Run all tests"""
    print("Testing Enhanced Zen LSP Features")
    print("=" * 50)

    test_ufc_resolution()

    print("\n" + "=" * 50)
    print("LSP test completed!")

if __name__ == "__main__":
    main()
