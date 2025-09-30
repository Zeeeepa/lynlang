#!/usr/bin/env python3
"""Test Zen LSP capabilities by simulating a simple client"""

import json
import subprocess
import threading
import time
import sys

def send_request(proc, id, method, params=None):
    """Send a JSON-RPC request to the LSP server"""
    request = {
        "jsonrpc": "2.0",
        "id": id,
        "method": method
    }
    if params:
        request["params"] = params

    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"

    proc.stdin.write(message.encode())
    proc.stdin.flush()

def read_response(proc):
    """Read a response from the LSP server"""
    # Read headers
    headers = {}
    while True:
        line = proc.stdout.readline().decode().strip()
        if not line:
            break
        key, value = line.split(": ", 1)
        headers[key] = value

    # Read content
    if "Content-Length" in headers:
        length = int(headers["Content-Length"])
        content = proc.stdout.read(length).decode()
        return json.loads(content)
    return None

def test_lsp():
    """Test basic LSP functionality"""
    print("Starting Zen LSP test...")

    # Start LSP server
    proc = subprocess.Popen(
        ["../../target/debug/zen-lsp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    try:
        # Send initialize request
        print("Sending initialize request...")
        send_request(proc, 1, "initialize", {
            "processId": None,
            "rootUri": None,
            "capabilities": {
                "textDocument": {
                    "hover": {"dynamicRegistration": False},
                    "completion": {"dynamicRegistration": False},
                    "definition": {"dynamicRegistration": False},
                    "references": {"dynamicRegistration": False}
                }
            }
        })

        # Read initialize response
        response = read_response(proc)
        if response and "result" in response:
            capabilities = response["result"]["capabilities"]
            print("✓ Server initialized successfully")

            # Check capabilities
            print("\nServer capabilities:")
            if capabilities.get("hoverProvider"):
                print("  ✓ Hover support")
            if capabilities.get("completionProvider"):
                print("  ✓ Completion support")
            if capabilities.get("definitionProvider"):
                print("  ✓ Goto definition support")
            if capabilities.get("referencesProvider"):
                print("  ✓ Find references support")
            if capabilities.get("documentSymbolProvider"):
                print("  ✓ Document symbols support")
            if capabilities.get("documentFormattingProvider"):
                print("  ✓ Formatting support")
            if capabilities.get("renameProvider"):
                print("  ✓ Rename support")
            if capabilities.get("codeActionProvider"):
                print("  ✓ Code actions support")

            # Send shutdown request
            print("\nSending shutdown request...")
            send_request(proc, 2, "shutdown")

            # Send exit notification
            send_request(proc, None, "exit")

            print("✓ LSP test completed successfully")
            return 0
        else:
            print("✗ Failed to initialize server")
            return 1

    except Exception as e:
        print(f"✗ Test failed: {e}")
        return 1
    finally:
        # Ensure process is terminated
        proc.terminate()
        time.sleep(0.5)
        if proc.poll() is None:
            proc.kill()

if __name__ == "__main__":
    sys.exit(test_lsp())