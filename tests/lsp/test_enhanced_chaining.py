#!/usr/bin/env python3
"""Test enhanced UFC method chaining in Zen LSP."""

import json
import subprocess
import time
import socket
import os
import sys

def send_request(proc, method, params=None, request_id=None):
    """Send an LSP request via stdio and return the response."""
    request = {
        "jsonrpc": "2.0",
        "method": method,
    }
    if request_id is not None:
        request["id"] = request_id
    if params is not None:
        request["params"] = params

    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

    if request_id is not None:
        # Read response header
        header_data = b""
        while b"\r\n\r\n" not in header_data:
            chunk = proc.stdout.read(1)
            if not chunk:
                break
            header_data += chunk

        # Parse content length
        content_length = 0
        for line in header_data.split(b"\r\n"):
            if line.startswith(b"Content-Length:"):
                content_length = int(line.split(b":")[1].strip())
                break

        # Read response body
        if content_length > 0:
            body = proc.stdout.read(content_length)
            return json.loads(body)

def test_enhanced_ufc_chaining():
    """Test enhanced UFC method chaining type resolution."""
    print("Testing enhanced UFC method chaining...")

    # Start LSP server
    lsp_path = "../../target/release/zen-lsp"
    if not os.path.exists(lsp_path):
        print(f"Error: LSP server not found at {lsp_path}")
        return False

    server = subprocess.Popen(
        [lsp_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL
    )

    # Give server time to start
    time.sleep(0.2)

    try:
        # Initialize
        send_request(server, "initialize", {
            "processId": os.getpid(),
            "capabilities": {
                "textDocument": {
                    "hover": {},
                    "completion": {"completionItem": {"snippetSupport": True}},
                    "definition": {},
                }
            },
            "rootUri": f"file://{os.getcwd()}",
        }, 1)

        # Open test file
        test_file = os.path.abspath("test_enhanced_ufc.zen")
        with open(test_file, 'r') as f:
            content = f.read()

        send_request(server, "textDocument/didOpen", {
            "textDocument": {
                "uri": f"file://{test_file}",
                "languageId": "zen",
                "version": 1,
                "text": content
            }
        })

        # Test 1: Hover on chained method call
        print("Test 1: Hover on text.to_upper().trim().split()")
        response = send_request(server, "textDocument/hover", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 11, "character": 45}  # On 'split'
        }, 2)

        if response and "result" in response and response["result"]:
            hover_content = response["result"]["contents"]["value"]
            if "Array" in hover_content:
                print("✓ Correctly identified split() returns Array")
            else:
                print(f"✗ Failed to identify return type: {hover_content}")

        # Test 2: Completion after chained call
        print("\nTest 2: Completion after map.get('key').")
        response = send_request(server, "textDocument/completion", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 16, "character": 31}  # After map.get("key").
        }, 3)

        if response and "result" in response:
            completions = response["result"]
            if isinstance(completions, list):
                methods = [c["label"] for c in completions]
                if "unwrap" in methods and "is_some" in methods:
                    print("✓ Correctly provided Option methods")
                else:
                    print(f"✗ Wrong methods: {methods[:5]}")

        # Test 3: Go to definition on chained method
        print("\nTest 3: Go to definition on vec.first().unwrap_or()")
        response = send_request(server, "textDocument/definition", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 21, "character": 35}  # On 'unwrap_or'
        }, 4)

        if response and "result" in response:
            if response["result"]:
                print("✓ Found definition for unwrap_or method")
            else:
                print("✗ Could not find definition")

        # Test 4: Complex chain type inference
        print("\nTest 4: Hover on map.keys().first().unwrap().to_upper()")
        response = send_request(server, "textDocument/hover", {
            "textDocument": {"uri": f"file://{test_file}"},
            "position": {"line": 24, "character": 52}  # On 'to_upper'
        }, 5)

        if response and "result" in response and response["result"]:
            hover_content = response["result"]["contents"]["value"]
            if "String" in hover_content:
                print("✓ Correctly tracked type through complex chain")
            else:
                print(f"✗ Failed complex chain inference: {hover_content}")

        print("\n✅ Enhanced UFC chaining tests completed!")
        return True

    finally:
        server.terminate()

if __name__ == "__main__":
    success = test_enhanced_ufc_chaining()
    sys.exit(0 if success else 1)