#!/usr/bin/env python3
"""Debug workspace symbol search for Zen LSP server."""

import json
import subprocess
import sys
import os
import time

def send_request(server, request_id, method, params):
    """Send a JSON-RPC request to the LSP server."""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    print(f"→ Sending: {method}")
    server.stdin.write(message.encode())
    server.stdin.flush()

def send_notification(server, method, params):
    """Send a JSON-RPC notification to the LSP server."""
    notification = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    }
    content = json.dumps(notification)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    print(f"→ Sending notification: {method}")
    server.stdin.write(message.encode())
    server.stdin.flush()

def read_response(server, timeout=5):
    """Read a JSON-RPC response from the LSP server."""
    import select
    
    # Check if data is available with timeout
    ready, _, _ = select.select([server.stdout], [], [], timeout)
    if not ready:
        print("⚠ Timeout waiting for response")
        return None
    
    # Read header
    headers = {}
    while True:
        line = server.stdout.readline().decode('utf-8')
        if line == '\r\n':
            break
        if ':' in line:
            key, value = line.split(':', 1)
            headers[key.strip()] = value.strip()

    # Read content
    content_length = int(headers.get('Content-Length', 0))
    if content_length == 0:
        return None

    content = server.stdout.read(content_length).decode('utf-8')
    response = json.loads(content)
    print(f"← Received response: {json.dumps(response, indent=2)}")
    return response

# Get LSP server path
lsp_path = os.path.join(os.path.dirname(__file__), "../../target/release/zen-lsp")

# Start LSP server
print(f"Starting LSP server: {lsp_path}")
server = subprocess.Popen(
    [lsp_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT
)

try:
    # Initialize
    send_request(server, 1, "initialize", {
        "processId": None,
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    })
    response = read_response(server)
    
    if not response:
        print("✗ Failed to get initialize response")
        sys.exit(1)

    # Send initialized notification
    send_notification(server, "initialized", {})
    time.sleep(0.1)  # Give server time to process

    # Open a test document
    test_uri = f"file://{os.getcwd()}/test_workspace_symbol.zen"
    test_content = """main = () void {
    x = 42;
    io.println(x);
}

helper_function = (value: i32) i32 {
    value * 2
}
"""

    send_notification(server, "textDocument/didOpen", {
        "textDocument": {
            "uri": test_uri,
            "languageId": "zen",
            "version": 1,
            "text": test_content
        }
    })
    time.sleep(0.5)  # Give server time to parse

    # Test workspace symbol search
    send_request(server, 2, "workspace/symbol", {
        "query": "function"
    })
    response = read_response(server)

    if response and "result" in response:
        print(f"\n✓ Workspace symbol response: {json.dumps(response['result'], indent=2)}")
    else:
        print(f"\n✗ No result in response")

finally:
    server.terminate()
    server.wait()
