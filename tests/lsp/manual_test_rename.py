#!/usr/bin/env python3
"""Quick manual test for rename functionality"""

import json
import subprocess
import os
import sys

def send_request(proc, method, params, req_id=1):
    """Send LSP request"""
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()

def read_response(proc, req_id=None):
    """Read LSP response"""
    while True:
        line = proc.stdout.readline().decode()
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            proc.stdout.readline()  # empty line
            content = proc.stdout.read(length).decode()
            response = json.loads(content)
            # If we're looking for a specific ID, keep reading until we find it
            if req_id is None or response.get("id") == req_id:
                return response

# Start LSP
lsp = subprocess.Popen(
    ["./target/release/zen-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

try:
    # Initialize
    send_request(lsp, "initialize", {
        "processId": os.getpid(),
        "rootUri": f"file://{os.getcwd()}",
        "capabilities": {}
    })
    init_response = read_response(lsp)
    print("✅ Initialized")

    # Send initialized notification
    request = {"jsonrpc": "2.0", "method": "initialized", "params": {}}
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    lsp.stdin.write(message.encode())
    lsp.stdin.flush()

    # Create test file
    test_code = """main = () i32 {
    value = 42
    result = value + 1
    return result
}"""

    # Open document
    send_request(lsp, "textDocument/didOpen", {
        "textDocument": {
            "uri": "file:///tmp/test_rename.zen",
            "languageId": "zen",
            "version": 1,
            "text": test_code
        }
    })

    # Request rename at position of "value" (line 1, char 4)
    send_request(lsp, "textDocument/rename", {
        "textDocument": {"uri": "file:///tmp/test_rename.zen"},
        "position": {"line": 1, "character": 4},
        "newName": "myValue"
    }, req_id=2)

    response = read_response(lsp, req_id=2)
    print("\n📝 Rename response:")
    print(json.dumps(response, indent=2))

    if "result" in response and response["result"]:
        print("\n✅ Rename returned workspace edit!")
        if "changes" in response["result"]:
            for uri, edits in response["result"]["changes"].items():
                print(f"\nFile: {uri}")
                for edit in edits:
                    print(f"  Line {edit['range']['start']['line']}: {edit['newText']}")
    else:
        print("\n❌ No rename results")

    # Print any stderr output
    import time
    time.sleep(0.1)

finally:
    stderr_output = lsp.stderr.read().decode()
    if stderr_output:
        print("\n📋 LSP stderr:")
        print(stderr_output[:1000])  # First 1000 chars
    lsp.terminate()
    lsp.wait()
